//! Authentication and Secure API Key Storage Module
//!
//! This module provides:
//! - User registration/login with Argon2id password hashing
//! - AES-256-GCM encrypted API key storage per provider
//! - SQLite-based local user database

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::RngCore;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

// ------------------------------------------------------------------
// Error Types
// ------------------------------------------------------------------

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Password hashing error: {0}")]
    PasswordHash(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("User already exists")]
    UserExists,

    #[error("User not found")]
    UserNotFound,

    #[error("No user logged in")]
    NotLoggedIn,

    #[error("Validation error: {0}")]
    Validation(String),
}

// ------------------------------------------------------------------
// Data Structures
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
    pub location: Option<String>,
}

/// User profile for updating profile information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub role: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub provider: String,
    pub has_key: bool,  // Don't expose actual key, just presence
}

/// Full brief with content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Brief {
    pub id: i64,
    pub user_id: i64,
    pub company: String,
    pub model: String,
    pub manifest_name: Option<String>,
    pub content: String,
    pub created_at: String,
}

/// Brief summary for listing (without full content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BriefSummary {
    pub id: i64,
    pub company: String,
    pub model: String,
    pub manifest_name: Option<String>,
    pub created_at: String,
}

/// Conversation message (user or assistant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub id: i64,
    pub brief_id: i64,
    pub role: String,  // "user" or "assistant"
    pub content: String,
    pub created_at: String,
}

/// Custom LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProvider {
    pub id: i64,
    pub user_id: i64,
    pub name: String,           // Display name (e.g., "My Local LLM")
    pub provider_key: String,   // Unique key for storage (e.g., "custom_my_local_llm")
    pub endpoint_url: String,   // API endpoint (e.g., "http://localhost:11434/v1")
    pub model_id: String,       // Model identifier (e.g., "llama3:70b")
    pub api_key_header: String, // Header name for API key (e.g., "Authorization" or "x-api-key")
    pub created_at: String,
}

/// Custom provider summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderSummary {
    pub id: i64,
    pub name: String,
    pub provider_key: String,
    pub model_id: String,
    pub has_key: bool,
}

/// Research session for persisting workflow progress
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSession {
    pub id: i64,
    pub user_id: i64,
    pub company: String,
    pub model: String,
    pub manifest_name: Option<String>,
    pub status: String, // "running", "completed", "failed"
    pub current_phase_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Research session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchSessionSummary {
    pub id: i64,
    pub company: String,
    pub model: String,
    pub manifest_name: Option<String>,
    pub status: String,
    pub current_phase_id: Option<String>,
    pub phase_count: i64,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
}

/// Individual phase output stored during workflow execution
#[derive(Debug, Clone, Serialize, Deserialize)]
/// Phase output record for session persistence
/// Extended for user data accessibility (IM-5011): system_prompt and user_input fields
pub struct PhaseOutput {
    pub id: i64,
    pub session_id: i64,
    pub phase_id: String,
    pub phase_name: String,
    pub status: String, // "running", "completed", "failed"
    pub system_prompt: Option<String>,  // IM-5001: System prompt sent to LLM
    pub user_input: Option<String>,     // IM-5002: User input/manifest data sent to LLM
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Session conversation message for user data accessibility (IM-5030, IM-5031, IM-5032)
/// Used to track full conversation history for session resume functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: i64,
    pub session_id: i64,
    pub phase_id: Option<String>,
    pub role: String,  // "user", "assistant", "system"
    pub content: String,
    pub created_at: String,
}

/// Result of session resume operation (IM-5020)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeSessionResult {
    pub session: ResearchSession,
    pub next_phase_id: String,
    pub context: SessionContext,
}

/// Session context for resume (IM-5020)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub history: Vec<SessionHistoryMessage>,
    pub last_completed_phase: String,
    pub total_phases: usize,
    pub completed_phases: usize,
}

/// Session history message (simplified for serialization to frontend)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHistoryMessage {
    pub role: String,  // "user" or "assistant"
    pub content: String,
}

/// Research project for grouping related sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Project summary for listing (with session count)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub session_count: i64,
    pub archived: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Supported AI providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    Anthropic,
    DeepSeek,
    Google,
    OpenAI,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Anthropic => "anthropic",
            Provider::DeepSeek => "deepseek",
            Provider::Google => "google",
            Provider::OpenAI => "openai",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "anthropic" => Some(Provider::Anthropic),
            "deepseek" => Some(Provider::DeepSeek),
            "google" => Some(Provider::Google),
            "openai" => Some(Provider::OpenAI),
            _ => None,
        }
    }
}

// ------------------------------------------------------------------
// Auth Manager
// ------------------------------------------------------------------

pub struct AuthManager {
    conn: Connection,
    current_user: Option<User>,
    /// Derived encryption key from user's password (only valid while logged in)
    encryption_key: Option<[u8; 32]>,
}

impl AuthManager {
    /// Create a new AuthManager with database at the specified path
    pub fn new(db_path: &PathBuf) -> Result<Self, AuthError> {
        let conn = Connection::open(db_path)?;

        let mut manager = Self {
            conn,
            current_user: None,
            encryption_key: None,
        };

        manager.init_database()?;
        Ok(manager)
    }

    /// Initialize database schema
    fn init_database(&mut self) -> Result<(), AuthError> {
        // Migration: Fix research_sessions table schema if it has the wrong CHECK constraint
        // This handles the 'in_progress' -> 'running' status migration
        self.migrate_research_sessions_schema()?;

        // Migration: Add user profile fields to existing users table
        self.migrate_users_profile_fields()?;

        // Migration: Add prompt fields to phase_outputs for user data accessibility (Sprint 2)
        self.migrate_phase_outputs_prompt_fields()?;

        // Migration: Add archived column to projects and research_sessions tables
        self.migrate_archive_columns()?;

        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                first_name TEXT,
                last_name TEXT,
                role TEXT,
                location TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                provider TEXT NOT NULL,
                encrypted_key BLOB NOT NULL,
                nonce BLOB NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, provider)
            );

            CREATE INDEX IF NOT EXISTS idx_api_keys_user_provider
            ON api_keys(user_id, provider);

            -- Briefs table: stores generated opportunity briefs
            CREATE TABLE IF NOT EXISTS briefs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                company TEXT NOT NULL,
                model TEXT NOT NULL,
                manifest_name TEXT,
                content TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_briefs_user_id
            ON briefs(user_id);

            CREATE INDEX IF NOT EXISTS idx_briefs_created_at
            ON briefs(created_at DESC);

            -- Conversations table: stores follow-up Q&A for each brief
            CREATE TABLE IF NOT EXISTS conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                brief_id INTEGER NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
                content TEXT NOT NULL,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (brief_id) REFERENCES briefs(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_conversations_brief_id
            ON conversations(brief_id);

            -- Custom providers table: stores user-defined LLM providers
            CREATE TABLE IF NOT EXISTS custom_providers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                provider_key TEXT NOT NULL,
                endpoint_url TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key_header TEXT NOT NULL DEFAULT 'Authorization',
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                UNIQUE(user_id, provider_key)
            );

            CREATE INDEX IF NOT EXISTS idx_custom_providers_user_id
            ON custom_providers(user_id);

            -- Research sessions table: tracks ongoing research workflows
            CREATE TABLE IF NOT EXISTS research_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                company TEXT NOT NULL,
                model TEXT NOT NULL,
                manifest_name TEXT,
                status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed')),
                current_phase_id TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_research_sessions_user_id
            ON research_sessions(user_id);

            CREATE INDEX IF NOT EXISTS idx_research_sessions_status
            ON research_sessions(status);

            -- Phase outputs table: stores individual phase results
            CREATE TABLE IF NOT EXISTS phase_outputs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                phase_id TEXT NOT NULL,
                phase_name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running' CHECK(status IN ('running', 'completed', 'failed')),
                output TEXT,
                error TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES research_sessions(id) ON DELETE CASCADE,
                UNIQUE(session_id, phase_id)
            );

            CREATE INDEX IF NOT EXISTS idx_phase_outputs_session_id
            ON phase_outputs(session_id);

            -- Session conversations table: stores session-level prompts and responses (IM-5030)
            -- Separate from briefs.conversations which stores post-research Q&A
            CREATE TABLE IF NOT EXISTS session_conversations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                phase_id TEXT,
                role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (session_id) REFERENCES research_sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_session_conversations_session_id
            ON session_conversations(session_id);

            -- Projects table: groups related research sessions
            CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_projects_user_id
            ON projects(user_id);

            -- Project-Session junction table: many-to-many relationship
            CREATE TABLE IF NOT EXISTS project_sessions (
                project_id INTEGER NOT NULL,
                session_id INTEGER NOT NULL,
                added_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (project_id, session_id),
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
                FOREIGN KEY (session_id) REFERENCES research_sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_project_sessions_project_id
            ON project_sessions(project_id);

            CREATE INDEX IF NOT EXISTS idx_project_sessions_session_id
            ON project_sessions(session_id);
            "#
        )?;

        Ok(())
    }

    /// Migrate research_sessions table schema from 'in_progress' to 'running' status
    /// SQLite doesn't support ALTER COLUMN, so we need to recreate the table
    fn migrate_research_sessions_schema(&mut self) -> Result<(), AuthError> {
        // Check if table exists
        let table_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='research_sessions')",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

        if !table_exists {
            // Table doesn't exist yet, no migration needed
            return Ok(());
        }

        // Check if the table has the old schema by looking for 'in_progress' in the CHECK constraint
        let sql: String = self.conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='research_sessions'",
            [],
            |row| row.get(0),
        ).unwrap_or_default();

        if sql.contains("in_progress") {
            println!("[AUTH] Migrating research_sessions schema: 'in_progress' -> 'running'");

            // Drop the old table and let init_database create the new one
            // Note: This loses any existing session data, but the feature is new
            self.conn.execute("DROP TABLE IF EXISTS phase_outputs", [])?;
            self.conn.execute("DROP TABLE IF EXISTS research_sessions", [])?;

            println!("[AUTH] Migration complete: research_sessions table recreated with 'running' status");
        }

        Ok(())
    }

    /// Migrate users table to add profile fields if they don't exist
    /// SQLite supports ALTER TABLE ADD COLUMN, so we can add columns one by one
    fn migrate_users_profile_fields(&mut self) -> Result<(), AuthError> {
        // Check if table exists
        let table_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='users')",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

        if !table_exists {
            // Table doesn't exist yet, no migration needed (init_database will create it)
            return Ok(());
        }

        // Check if first_name column exists by trying to query it
        let has_first_name = self.conn.query_row(
            "SELECT first_name FROM users LIMIT 1",
            [],
            |_row| Ok(()),
        );

        if has_first_name.is_err() {
            println!("[AUTH] Migrating users table: adding profile fields");

            // Add profile columns one at a time
            let _ = self.conn.execute("ALTER TABLE users ADD COLUMN first_name TEXT", []);
            let _ = self.conn.execute("ALTER TABLE users ADD COLUMN last_name TEXT", []);
            let _ = self.conn.execute("ALTER TABLE users ADD COLUMN role TEXT", []);
            let _ = self.conn.execute("ALTER TABLE users ADD COLUMN location TEXT", []);

            println!("[AUTH] Migration complete: added first_name, last_name, role, location columns");
        }

        Ok(())
    }

    /// Migrate phase_outputs table to add prompt fields for user data accessibility
    /// IM-5001: system_prompt, IM-5002: user_input
    /// Uses idempotent ALTER TABLE ADD COLUMN pattern (safe to run multiple times)
    fn migrate_phase_outputs_prompt_fields(&mut self) -> Result<(), AuthError> {
        // Check if table exists
        let table_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='phase_outputs')",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

        if !table_exists {
            // Table doesn't exist yet, no migration needed (init_database will create it)
            return Ok(());
        }

        // Check if system_prompt column exists by trying to query it
        let has_system_prompt = self.conn.query_row(
            "SELECT system_prompt FROM phase_outputs LIMIT 1",
            [],
            |_row| Ok(()),
        );

        if has_system_prompt.is_err() {
            println!("[AUTH] Migrating phase_outputs table: adding prompt fields (IM-5001, IM-5002)");

            // Add prompt columns using idempotent pattern (let _ = ignores "column exists" errors)
            let _ = self.conn.execute("ALTER TABLE phase_outputs ADD COLUMN system_prompt TEXT", []);
            let _ = self.conn.execute("ALTER TABLE phase_outputs ADD COLUMN user_input TEXT", []);

            println!("[AUTH] Migration complete: added system_prompt, user_input columns to phase_outputs");
        }

        Ok(())
    }

    /// Migrate projects and research_sessions tables to add archived column
    /// Uses idempotent ALTER TABLE ADD COLUMN pattern (safe to run multiple times)
    fn migrate_archive_columns(&mut self) -> Result<(), AuthError> {
        // Check if projects table exists
        let projects_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='projects')",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

        if projects_exists {
            // Check if archived column exists in projects by trying to query it
            let has_archived = self.conn.query_row(
                "SELECT archived FROM projects LIMIT 1",
                [],
                |_row| Ok(()),
            );

            if has_archived.is_err() {
                println!("[AUTH] Migrating projects table: adding archived column");
                let _ = self.conn.execute("ALTER TABLE projects ADD COLUMN archived INTEGER DEFAULT 0", []);
                println!("[AUTH] Migration complete: added archived column to projects");
            }
        }

        // Check if research_sessions table exists
        let sessions_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='research_sessions')",
            [],
            |row| row.get(0),
        ).unwrap_or(false);

        if sessions_exists {
            // Check if archived column exists in research_sessions
            let has_archived = self.conn.query_row(
                "SELECT archived FROM research_sessions LIMIT 1",
                [],
                |_row| Ok(()),
            );

            if has_archived.is_err() {
                println!("[AUTH] Migrating research_sessions table: adding archived column");
                let _ = self.conn.execute("ALTER TABLE research_sessions ADD COLUMN archived INTEGER DEFAULT 0", []);
                println!("[AUTH] Migration complete: added archived column to research_sessions");
            }
        }

        Ok(())
    }

    // ------------------------------------------------------------------
    // User Management
    // ------------------------------------------------------------------

    /// Register a new user
    pub fn register(&mut self, username: &str, password: &str) -> Result<User, AuthError> {
        // Validate input
        let username = username.trim();
        if username.is_empty() || password.is_empty() {
            return Err(AuthError::InvalidCredentials);
        }

        // Check if user exists
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = ?1)",
            params![username],
            |row| row.get(0),
        )?;

        if exists {
            return Err(AuthError::UserExists);
        }

        // Hash password with Argon2id
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?
            .to_string();

        // Insert user
        self.conn.execute(
            "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
            params![username, password_hash],
        )?;

        let id = self.conn.last_insert_rowid();

        Ok(User {
            id,
            username: username.to_string(),
            first_name: None,
            last_name: None,
            role: None,
            location: None,
        })
    }

    /// Login an existing user
    pub fn login(&mut self, username: &str, password: &str) -> Result<User, AuthError> {
        let username = username.trim();

        // Fetch user, password hash, and profile fields
        let (id, stored_hash, first_name, last_name, role, location): (i64, String, Option<String>, Option<String>, Option<String>, Option<String>) = self.conn.query_row(
            "SELECT id, password_hash, first_name, last_name, role, location FROM users WHERE username = ?1",
            params![username],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        ).map_err(|_| AuthError::InvalidCredentials)?;

        // Verify password
        let parsed_hash = PasswordHash::new(&stored_hash)
            .map_err(|e| AuthError::PasswordHash(e.to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AuthError::InvalidCredentials)?;

        // Derive encryption key from password
        let encryption_key = self.derive_encryption_key(password, username);

        let user = User {
            id,
            username: username.to_string(),
            first_name,
            last_name,
            role,
            location,
        };

        self.current_user = Some(user.clone());
        self.encryption_key = Some(encryption_key);

        Ok(user)
    }

    /// Logout the current user
    pub fn logout(&mut self) {
        self.current_user = None;
        self.encryption_key = None;
    }

    /// Get the currently logged in user
    pub fn current_user(&self) -> Option<&User> {
        self.current_user.as_ref()
    }

    /// Check if a user is logged in
    pub fn is_logged_in(&self) -> bool {
        self.current_user.is_some()
    }

    /// Get current user's profile
    pub fn get_user_profile(&self) -> Result<UserProfile, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        Ok(UserProfile {
            first_name: user.first_name.clone(),
            last_name: user.last_name.clone(),
            role: user.role.clone(),
            location: user.location.clone(),
        })
    }

    /// Update current user's profile
    pub fn update_user_profile(&mut self, profile: UserProfile) -> Result<User, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;
        let user_id = user.id;
        let username = user.username.clone();

        // Update the database
        self.conn.execute(
            r#"
            UPDATE users
            SET first_name = ?1, last_name = ?2, role = ?3, location = ?4
            WHERE id = ?5
            "#,
            params![profile.first_name, profile.last_name, profile.role, profile.location, user_id],
        )?;

        // Update the cached current user
        let updated_user = User {
            id: user_id,
            username,
            first_name: profile.first_name,
            last_name: profile.last_name,
            role: profile.role,
            location: profile.location,
        };

        self.current_user = Some(updated_user.clone());

        Ok(updated_user)
    }

    // ------------------------------------------------------------------
    // API Key Management
    // ------------------------------------------------------------------

    /// Store an API key for a provider (encrypted)
    pub fn store_api_key(&self, provider: Provider, api_key: &str) -> Result<(), AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;
        let key = self.encryption_key.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Encrypt the API key
        let (encrypted, nonce) = self.encrypt_api_key(api_key, key)?;

        // Upsert the key
        self.conn.execute(
            r#"
            INSERT INTO api_keys (user_id, provider, encrypted_key, nonce)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(user_id, provider) DO UPDATE SET
                encrypted_key = excluded.encrypted_key,
                nonce = excluded.nonce,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![user.id, provider.as_str(), encrypted, nonce.as_slice()],
        )?;

        Ok(())
    }

    /// Retrieve a decrypted API key for a provider
    pub fn get_api_key(&self, provider: Provider) -> Result<Option<String>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;
        let key = self.encryption_key.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result: Result<(Vec<u8>, Vec<u8>), _> = self.conn.query_row(
            "SELECT encrypted_key, nonce FROM api_keys WHERE user_id = ?1 AND provider = ?2",
            params![user.id, provider.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((encrypted, nonce)) => {
                let decrypted = self.decrypt_api_key(&encrypted, &nonce, key)?;
                Ok(Some(decrypted))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Delete an API key for a provider
    pub fn delete_api_key(&self, provider: Provider) -> Result<(), AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        self.conn.execute(
            "DELETE FROM api_keys WHERE user_id = ?1 AND provider = ?2",
            params![user.id, provider.as_str()],
        )?;

        Ok(())
    }

    /// List all API keys for current user (without exposing actual keys)
    pub fn list_api_keys(&self) -> Result<Vec<ApiKeyEntry>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            "SELECT provider FROM api_keys WHERE user_id = ?1"
        )?;

        let providers: Vec<String> = stmt
            .query_map(params![user.id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();

        // Return all providers with their status
        let all_providers = vec![
            Provider::Anthropic,
            Provider::DeepSeek,
            Provider::Google,
            Provider::OpenAI,
        ];

        let entries = all_providers
            .into_iter()
            .map(|p| ApiKeyEntry {
                provider: p.as_str().to_string(),
                has_key: providers.contains(&p.as_str().to_string()),
            })
            .collect();

        Ok(entries)
    }

    // ------------------------------------------------------------------
    // Brief Management
    // ------------------------------------------------------------------

    /// Save a new brief for the current user
    pub fn save_brief(
        &self,
        company: &str,
        model: &str,
        manifest_name: Option<&str>,
        content: &str,
    ) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        self.conn.execute(
            r#"
            INSERT INTO briefs (user_id, company, model, manifest_name, content)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![user.id, company, model, manifest_name, content],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// List all briefs for the current user (summaries only, most recent first)
    pub fn list_briefs(&self) -> Result<Vec<BriefSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, company, model, manifest_name, created_at
            FROM briefs
            WHERE user_id = ?1
            ORDER BY created_at DESC
            "#,
        )?;

        let briefs = stmt
            .query_map(params![user.id], |row| {
                Ok(BriefSummary {
                    id: row.get(0)?,
                    company: row.get(1)?,
                    model: row.get(2)?,
                    manifest_name: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(briefs)
    }

    /// Get a specific brief by ID (must belong to current user)
    pub fn get_brief(&self, brief_id: i64) -> Result<Option<Brief>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result = self.conn.query_row(
            r#"
            SELECT id, user_id, company, model, manifest_name, content, created_at
            FROM briefs
            WHERE id = ?1 AND user_id = ?2
            "#,
            params![brief_id, user.id],
            |row| {
                Ok(Brief {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    company: row.get(2)?,
                    model: row.get(3)?,
                    manifest_name: row.get(4)?,
                    content: row.get(5)?,
                    created_at: row.get(6)?,
                })
            },
        );

        match result {
            Ok(brief) => Ok(Some(brief)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Delete a brief by ID (must belong to current user, cascades to conversations)
    pub fn delete_brief(&self, brief_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_deleted = self.conn.execute(
            "DELETE FROM briefs WHERE id = ?1 AND user_id = ?2",
            params![brief_id, user.id],
        )?;

        Ok(rows_deleted > 0)
    }

    // ------------------------------------------------------------------
    // Conversation Management
    // ------------------------------------------------------------------

    /// Add a message to a brief's conversation
    pub fn add_conversation_message(
        &self,
        brief_id: i64,
        role: &str,
        content: &str,
    ) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the brief belongs to the current user
        let brief_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM briefs WHERE id = ?1 AND user_id = ?2)",
            params![brief_id, user.id],
            |row| row.get(0),
        )?;

        if !brief_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        self.conn.execute(
            r#"
            INSERT INTO conversations (brief_id, role, content)
            VALUES (?1, ?2, ?3)
            "#,
            params![brief_id, role, content],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get all conversation messages for a brief
    pub fn get_conversation(&self, brief_id: i64) -> Result<Vec<ConversationMessage>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the brief belongs to the current user
        let brief_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM briefs WHERE id = ?1 AND user_id = ?2)",
            params![brief_id, user.id],
            |row| row.get(0),
        )?;

        if !brief_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, brief_id, role, content, created_at
            FROM conversations
            WHERE brief_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;

        let messages = stmt
            .query_map(params![brief_id], |row| {
                Ok(ConversationMessage {
                    id: row.get(0)?,
                    brief_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(messages)
    }

    // ------------------------------------------------------------------
    // Custom Provider Management
    // ------------------------------------------------------------------

    /// Add a custom LLM provider
    pub fn add_custom_provider(
        &self,
        name: &str,
        endpoint_url: &str,
        model_id: &str,
        api_key_header: &str,
    ) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Generate a unique provider key from the name
        let provider_key = format!(
            "custom_{}",
            name.to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '_' })
                .collect::<String>()
        );

        self.conn.execute(
            r#"
            INSERT INTO custom_providers (user_id, name, provider_key, endpoint_url, model_id, api_key_header)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![user.id, name, provider_key, endpoint_url, model_id, api_key_header],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// List all custom providers for the current user (with API key status)
    pub fn list_custom_providers(&self) -> Result<Vec<CustomProviderSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                cp.id,
                cp.name,
                cp.provider_key,
                cp.model_id,
                EXISTS(SELECT 1 FROM api_keys WHERE user_id = cp.user_id AND provider = cp.provider_key) as has_key
            FROM custom_providers cp
            WHERE cp.user_id = ?1
            ORDER BY cp.name ASC
            "#,
        )?;

        let providers = stmt
            .query_map(params![user.id], |row| {
                Ok(CustomProviderSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider_key: row.get(2)?,
                    model_id: row.get(3)?,
                    has_key: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(providers)
    }

    /// Get a specific custom provider by ID
    pub fn get_custom_provider(&self, provider_id: i64) -> Result<Option<CustomProvider>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result = self.conn.query_row(
            r#"
            SELECT id, user_id, name, provider_key, endpoint_url, model_id, api_key_header, created_at
            FROM custom_providers
            WHERE id = ?1 AND user_id = ?2
            "#,
            params![provider_id, user.id],
            |row| {
                Ok(CustomProvider {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    provider_key: row.get(3)?,
                    endpoint_url: row.get(4)?,
                    model_id: row.get(5)?,
                    api_key_header: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        );

        match result {
            Ok(provider) => Ok(Some(provider)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Get a custom provider by provider_key
    pub fn get_custom_provider_by_key(&self, provider_key: &str) -> Result<Option<CustomProvider>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result = self.conn.query_row(
            r#"
            SELECT id, user_id, name, provider_key, endpoint_url, model_id, api_key_header, created_at
            FROM custom_providers
            WHERE provider_key = ?1 AND user_id = ?2
            "#,
            params![provider_key, user.id],
            |row| {
                Ok(CustomProvider {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    provider_key: row.get(3)?,
                    endpoint_url: row.get(4)?,
                    model_id: row.get(5)?,
                    api_key_header: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        );

        match result {
            Ok(provider) => Ok(Some(provider)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Delete a custom provider by ID (also deletes associated API key)
    pub fn delete_custom_provider(&self, provider_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // First get the provider_key to delete associated API key
        let provider_key: Option<String> = self.conn.query_row(
            "SELECT provider_key FROM custom_providers WHERE id = ?1 AND user_id = ?2",
            params![provider_id, user.id],
            |row| row.get(0),
        ).ok();

        // Delete the API key if it exists
        if let Some(key) = &provider_key {
            self.conn.execute(
                "DELETE FROM api_keys WHERE user_id = ?1 AND provider = ?2",
                params![user.id, key],
            )?;
        }

        // Delete the custom provider
        let rows_deleted = self.conn.execute(
            "DELETE FROM custom_providers WHERE id = ?1 AND user_id = ?2",
            params![provider_id, user.id],
        )?;

        Ok(rows_deleted > 0)
    }

    /// Store an API key for a custom provider (by provider_key string)
    pub fn store_custom_api_key(&self, provider_key: &str, api_key: &str) -> Result<(), AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;
        let key = self.encryption_key.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Encrypt the API key
        let (encrypted, nonce) = self.encrypt_api_key(api_key, key)?;

        // Upsert the key
        self.conn.execute(
            r#"
            INSERT INTO api_keys (user_id, provider, encrypted_key, nonce)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(user_id, provider) DO UPDATE SET
                encrypted_key = excluded.encrypted_key,
                nonce = excluded.nonce,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![user.id, provider_key, encrypted, nonce.as_slice()],
        )?;

        Ok(())
    }

    /// Get an API key for a custom provider (by provider_key string)
    pub fn get_custom_api_key(&self, provider_key: &str) -> Result<Option<String>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;
        let key = self.encryption_key.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result: Result<(Vec<u8>, Vec<u8>), _> = self.conn.query_row(
            "SELECT encrypted_key, nonce FROM api_keys WHERE user_id = ?1 AND provider = ?2",
            params![user.id, provider_key],
            |row| Ok((row.get(0)?, row.get(1)?)),
        );

        match result {
            Ok((encrypted, nonce)) => {
                let decrypted = self.decrypt_api_key(&encrypted, &nonce, key)?;
                Ok(Some(decrypted))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    // ------------------------------------------------------------------
    // Research Session Management
    // ------------------------------------------------------------------

    /// Create a new research session (auto-saves workflow progress)
    pub fn create_research_session(
        &self,
        company: &str,
        model: &str,
        manifest_name: Option<&str>,
    ) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        self.conn.execute(
            r#"
            INSERT INTO research_sessions (user_id, company, model, manifest_name, status)
            VALUES (?1, ?2, ?3, ?4, 'running')
            "#,
            params![user.id, company, model, manifest_name],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Update research session status and current phase
    pub fn update_research_session(
        &self,
        session_id: i64,
        status: &str,
        current_phase_id: Option<&str>,
    ) -> Result<(), AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        self.conn.execute(
            r#"
            UPDATE research_sessions
            SET status = ?1, current_phase_id = ?2, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?3 AND user_id = ?4
            "#,
            params![status, current_phase_id, session_id, user.id],
        )?;

        Ok(())
    }

    /// List all research sessions for the current user (most recent first)
    /// Includes project association info for display in session history
    pub fn list_research_sessions(&self) -> Result<Vec<ResearchSessionSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                rs.id,
                rs.company,
                rs.model,
                rs.manifest_name,
                rs.status,
                rs.current_phase_id,
                (SELECT COUNT(*) FROM phase_outputs WHERE session_id = rs.id) as phase_count,
                COALESCE(rs.archived, 0) as archived,
                rs.created_at,
                rs.updated_at,
                ps.project_id,
                p.name as project_name
            FROM research_sessions rs
            LEFT JOIN project_sessions ps ON rs.id = ps.session_id
            LEFT JOIN projects p ON ps.project_id = p.id
            WHERE rs.user_id = ?1 AND COALESCE(rs.archived, 0) = 0
            ORDER BY rs.updated_at DESC
            "#,
        )?;

        let sessions = stmt
            .query_map(params![user.id], |row| {
                Ok(ResearchSessionSummary {
                    id: row.get(0)?,
                    company: row.get(1)?,
                    model: row.get(2)?,
                    manifest_name: row.get(3)?,
                    status: row.get(4)?,
                    current_phase_id: row.get(5)?,
                    phase_count: row.get(6)?,
                    archived: row.get::<_, i64>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    project_id: row.get(10)?,
                    project_name: row.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    /// Get a specific research session by ID
    pub fn get_research_session(&self, session_id: i64) -> Result<Option<ResearchSession>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result = self.conn.query_row(
            r#"
            SELECT id, user_id, company, model, manifest_name, status, current_phase_id, created_at, updated_at
            FROM research_sessions
            WHERE id = ?1 AND user_id = ?2
            "#,
            params![session_id, user.id],
            |row| {
                Ok(ResearchSession {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    company: row.get(2)?,
                    model: row.get(3)?,
                    manifest_name: row.get(4)?,
                    status: row.get(5)?,
                    current_phase_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        );

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Delete a research session by ID (cascades to phase_outputs)
    pub fn delete_research_session(&self, session_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_deleted = self.conn.execute(
            "DELETE FROM research_sessions WHERE id = ?1 AND user_id = ?2",
            params![session_id, user.id],
        )?;

        Ok(rows_deleted > 0)
    }

    /// Rename a research session (update the company field which serves as the name)
    pub fn rename_research_session(&self, session_id: i64, new_name: &str) -> Result<(), AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        self.conn.execute(
            "UPDATE research_sessions SET company = ?1 WHERE id = ?2 AND user_id = ?3",
            params![new_name, session_id, user.id],
        )?;

        Ok(())
    }

    // ------------------------------------------------------------------
    // Phase Output Management
    // ------------------------------------------------------------------

    /// Save or update a phase output (upserts on session_id + phase_id)
    /// Extended for user data accessibility (IM-5010): system_prompt and user_input fields
    /// Uses COALESCE pattern: "running" sends prompts, "completed" sends output - both preserved
    pub fn save_phase_output(
        &self,
        session_id: i64,
        phase_id: &str,
        phase_name: &str,
        status: &str,
        system_prompt: Option<&str>,
        user_input: Option<&str>,
        output: Option<&str>,
        error: Option<&str>,
    ) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the session belongs to the current user
        let session_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?1 AND user_id = ?2)",
            params![session_id, user.id],
            |row| row.get(0),
        )?;

        if !session_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        // Upsert the phase output with COALESCE pattern for all nullable fields
        // - "running" emits: prompts filled, output NULL -> prompts preserved
        // - "completed" emits: prompts NULL, output filled -> output preserved, prompts untouched
        self.conn.execute(
            r#"
            INSERT INTO phase_outputs (session_id, phase_id, phase_name, status, system_prompt, user_input, output, error)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ON CONFLICT(session_id, phase_id) DO UPDATE SET
                status = excluded.status,
                system_prompt = COALESCE(excluded.system_prompt, phase_outputs.system_prompt),
                user_input = COALESCE(excluded.user_input, phase_outputs.user_input),
                output = COALESCE(excluded.output, phase_outputs.output),
                error = excluded.error,
                updated_at = CURRENT_TIMESTAMP
            "#,
            params![session_id, phase_id, phase_name, status, system_prompt, user_input, output, error],
        )?;

        // Get the ID of the inserted/updated row
        let id: i64 = self.conn.query_row(
            "SELECT id FROM phase_outputs WHERE session_id = ?1 AND phase_id = ?2",
            params![session_id, phase_id],
            |row| row.get(0),
        )?;

        // Update the research session's current phase and updated_at
        self.conn.execute(
            r#"
            UPDATE research_sessions
            SET current_phase_id = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            "#,
            params![phase_id, session_id],
        )?;

        Ok(id)
    }

    /// Get all phase outputs for a research session (in order)
    pub fn get_phase_outputs(&self, session_id: i64) -> Result<Vec<PhaseOutput>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the session belongs to the current user
        let session_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?1 AND user_id = ?2)",
            params![session_id, user.id],
            |row| row.get(0),
        )?;

        if !session_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        // Extended for user data accessibility (IM-5011): includes system_prompt and user_input
        let mut stmt = self.conn.prepare(
            r#"
            SELECT id, session_id, phase_id, phase_name, status, system_prompt, user_input, output, error, created_at, updated_at
            FROM phase_outputs
            WHERE session_id = ?1
            ORDER BY created_at ASC
            "#,
        )?;

        let outputs = stmt
            .query_map(params![session_id], |row| {
                Ok(PhaseOutput {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    phase_id: row.get(2)?,
                    phase_name: row.get(3)?,
                    status: row.get(4)?,
                    system_prompt: row.get(5)?,
                    user_input: row.get(6)?,
                    output: row.get(7)?,
                    error: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(outputs)
    }

    /// Get the last completed phase output for a session (for resumption)
    pub fn get_last_completed_phase(&self, session_id: i64) -> Result<Option<PhaseOutput>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the session belongs to the current user
        let session_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?1 AND user_id = ?2)",
            params![session_id, user.id],
            |row| row.get(0),
        )?;

        if !session_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        // Extended for user data accessibility (IM-5011): includes system_prompt and user_input
        let result = self.conn.query_row(
            r#"
            SELECT id, session_id, phase_id, phase_name, status, system_prompt, user_input, output, error, created_at, updated_at
            FROM phase_outputs
            WHERE session_id = ?1 AND status = 'completed'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
            params![session_id],
            |row| {
                Ok(PhaseOutput {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    phase_id: row.get(2)?,
                    phase_name: row.get(3)?,
                    status: row.get(4)?,
                    system_prompt: row.get(5)?,
                    user_input: row.get(6)?,
                    output: row.get(7)?,
                    error: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        );

        match result {
            Ok(output) => Ok(Some(output)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    // ------------------------------------------------------------------
    // Session Conversation Management (IM-5030, IM-5031, IM-5032)
    // ------------------------------------------------------------------

    /// Add a message to session conversation history (IM-5031)
    /// Used for tracking full conversation for resume functionality
    pub fn add_session_message(
        &self,
        session_id: i64,
        phase_id: Option<&str>,
        role: &str,
        content: &str,
    ) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the session belongs to the current user
        let session_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?1 AND user_id = ?2)",
            params![session_id, user.id],
            |row| row.get(0),
        )?;

        if !session_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        // Validate role
        if !["user", "assistant", "system"].contains(&role) {
            return Err(AuthError::Validation(format!("Invalid role: {}", role)));
        }

        self.conn.execute(
            r#"
            INSERT INTO session_conversations (session_id, phase_id, role, content)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![session_id, phase_id, role, content],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get session conversation history (IM-5032)
    /// Returns messages ordered by creation time for resume context reconstruction
    /// Optionally filtered by phase_id, with sliding window limit
    pub fn get_session_conversation(
        &self,
        session_id: i64,
        phase_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<SessionMessage>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify the session belongs to the current user
        let session_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?1 AND user_id = ?2)",
            params![session_id, user.id],
            |row| row.get(0),
        )?;

        if !session_exists {
            return Err(AuthError::Database(rusqlite::Error::QueryReturnedNoRows));
        }

        // Build query based on whether phase_id filter is provided
        let (_query, messages): (String, Vec<SessionMessage>) = if let Some(pid) = phase_id {
            let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
            let query = format!(
                r#"
                SELECT id, session_id, phase_id, role, content, created_at
                FROM session_conversations
                WHERE session_id = ?1 AND phase_id = ?2
                ORDER BY created_at ASC
                {}
                "#,
                limit_clause
            );
            let mut stmt = self.conn.prepare(&query)?;
            let msgs = stmt
                .query_map(params![session_id, pid], |row| {
                    Ok(SessionMessage {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        phase_id: row.get(2)?,
                        role: row.get(3)?,
                        content: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();
            (query, msgs)
        } else {
            let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();
            let query = format!(
                r#"
                SELECT id, session_id, phase_id, role, content, created_at
                FROM session_conversations
                WHERE session_id = ?1
                ORDER BY created_at ASC
                {}
                "#,
                limit_clause
            );
            let mut stmt = self.conn.prepare(&query)?;
            let msgs = stmt
                .query_map(params![session_id], |row| {
                    Ok(SessionMessage {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        phase_id: row.get(2)?,
                        role: row.get(3)?,
                        content: row.get(4)?,
                        created_at: row.get(5)?,
                    })
                })?
                .filter_map(|r| r.ok())
                .collect();
            (query, msgs)
        };

        Ok(messages)
    }

    // ------------------------------------------------------------------
    // Encryption Helpers
    // ------------------------------------------------------------------

    /// Derive a 256-bit encryption key from password using Argon2
    fn derive_encryption_key(&self, password: &str, username: &str) -> [u8; 32] {
        // Use username as salt for key derivation (deterministic for same user)
        let salt = format!("fullintel-key-{}", username);

        let mut output = [0u8; 32];

        // Use Argon2 for key derivation
        argon2::Argon2::default()
            .hash_password_into(
                password.as_bytes(),
                salt.as_bytes(),
                &mut output,
            )
            .expect("Key derivation failed");

        output
    }

    /// Encrypt an API key using AES-256-GCM
    fn encrypt_api_key(&self, api_key: &str, key: &[u8; 32]) -> Result<(Vec<u8>, [u8; 12]), AuthError> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| AuthError::Encryption(e.to_string()))?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted = cipher
            .encrypt(nonce, api_key.as_bytes())
            .map_err(|e| AuthError::Encryption(e.to_string()))?;

        Ok((encrypted, nonce_bytes))
    }

    /// Decrypt an API key using AES-256-GCM
    fn decrypt_api_key(&self, encrypted: &[u8], nonce: &[u8], key: &[u8; 32]) -> Result<String, AuthError> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| AuthError::Encryption(e.to_string()))?;

        let nonce = Nonce::from_slice(nonce);

        let decrypted = cipher
            .decrypt(nonce, encrypted)
            .map_err(|e| AuthError::Encryption(e.to_string()))?;

        String::from_utf8(decrypted)
            .map_err(|e| AuthError::Encryption(e.to_string()))
    }

    // ------------------------------------------------------------------
    // Project Management
    // ------------------------------------------------------------------

    /// Create a new project
    pub fn create_project(&self, name: &str, description: Option<&str>) -> Result<i64, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Validate name
        if name.trim().is_empty() {
            return Err(AuthError::Validation("Project name cannot be empty".to_string()));
        }

        self.conn.execute(
            r#"
            INSERT INTO projects (user_id, name, description)
            VALUES (?1, ?2, ?3)
            "#,
            params![user.id, name.trim(), description],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// List all projects for the current user
    pub fn list_projects(&self) -> Result<Vec<ProjectSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT p.id, p.name, p.description, p.created_at, p.updated_at,
                   COALESCE(p.archived, 0) as archived,
                   COUNT(ps.session_id) as session_count
            FROM projects p
            LEFT JOIN project_sessions ps ON p.id = ps.project_id
            WHERE p.user_id = ?1 AND COALESCE(p.archived, 0) = 0
            GROUP BY p.id
            ORDER BY p.updated_at DESC
            "#,
        )?;

        let projects = stmt
            .query_map(params![user.id], |row| {
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    archived: row.get::<_, i64>(5)? != 0,
                    session_count: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    /// Get a project by ID
    pub fn get_project(&self, project_id: i64) -> Result<Option<Project>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let result = self.conn.query_row(
            r#"
            SELECT id, user_id, name, description, COALESCE(archived, 0), created_at, updated_at
            FROM projects
            WHERE id = ?1 AND user_id = ?2
            "#,
            params![project_id, user.id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    archived: row.get::<_, i64>(4)? != 0,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AuthError::Database(e)),
        }
    }

    /// Update a project
    pub fn update_project(&self, project_id: i64, name: &str, description: Option<&str>) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        if name.trim().is_empty() {
            return Err(AuthError::Validation("Project name cannot be empty".to_string()));
        }

        let rows_updated = self.conn.execute(
            r#"
            UPDATE projects
            SET name = ?1, description = ?2, updated_at = datetime('now')
            WHERE id = ?3 AND user_id = ?4
            "#,
            params![name.trim(), description, project_id, user.id],
        )?;

        Ok(rows_updated > 0)
    }

    /// Delete a project (cascades to project_sessions junction table)
    pub fn delete_project(&self, project_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_deleted = self.conn.execute(
            "DELETE FROM projects WHERE id = ?1 AND user_id = ?2",
            params![project_id, user.id],
        )?;

        Ok(rows_deleted > 0)
    }

    // =====================================================
    // ARCHIVE FUNCTIONS
    // =====================================================

    /// Archive a project (soft delete - hides from main list)
    pub fn archive_project(&self, project_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_updated = self.conn.execute(
            "UPDATE projects SET archived = 1, updated_at = datetime('now') WHERE id = ?1 AND user_id = ?2",
            params![project_id, user.id],
        )?;

        Ok(rows_updated > 0)
    }

    /// Unarchive a project (restore to main list)
    pub fn unarchive_project(&self, project_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_updated = self.conn.execute(
            "UPDATE projects SET archived = 0, updated_at = datetime('now') WHERE id = ?1 AND user_id = ?2",
            params![project_id, user.id],
        )?;

        Ok(rows_updated > 0)
    }

    /// Archive a research session (soft delete - hides from main list)
    pub fn archive_session(&self, session_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_updated = self.conn.execute(
            "UPDATE research_sessions SET archived = 1, updated_at = datetime('now') WHERE id = ?1 AND user_id = ?2",
            params![session_id, user.id],
        )?;

        Ok(rows_updated > 0)
    }

    /// Unarchive a research session (restore to main list)
    pub fn unarchive_session(&self, session_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let rows_updated = self.conn.execute(
            "UPDATE research_sessions SET archived = 0, updated_at = datetime('now') WHERE id = ?1 AND user_id = ?2",
            params![session_id, user.id],
        )?;

        Ok(rows_updated > 0)
    }

    /// List archived projects (for the Archived section in sidebar)
    pub fn list_archived_projects(&self) -> Result<Vec<ProjectSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT p.id, p.name, p.description, p.created_at, p.updated_at,
                   COALESCE(p.archived, 0) as archived,
                   COUNT(ps.session_id) as session_count
            FROM projects p
            LEFT JOIN project_sessions ps ON p.id = ps.project_id
            WHERE p.user_id = ?1 AND COALESCE(p.archived, 0) = 1
            GROUP BY p.id
            ORDER BY p.updated_at DESC
            "#,
        )?;

        let projects = stmt
            .query_map(params![user.id], |row| {
                Ok(ProjectSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    archived: row.get::<_, i64>(5)? != 0,
                    session_count: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    /// List archived research sessions (for the Archived section in sidebar)
    pub fn list_archived_sessions(&self) -> Result<Vec<ResearchSessionSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        let mut stmt = self.conn.prepare(
            r#"
            SELECT
                rs.id,
                rs.company,
                rs.model,
                rs.manifest_name,
                rs.status,
                rs.current_phase_id,
                (SELECT COUNT(*) FROM phase_outputs WHERE session_id = rs.id) as phase_count,
                COALESCE(rs.archived, 0) as archived,
                rs.created_at,
                rs.updated_at,
                ps.project_id,
                p.name as project_name
            FROM research_sessions rs
            LEFT JOIN project_sessions ps ON rs.id = ps.session_id
            LEFT JOIN projects p ON ps.project_id = p.id
            WHERE rs.user_id = ?1 AND COALESCE(rs.archived, 0) = 1
            ORDER BY rs.updated_at DESC
            "#,
        )?;

        let sessions = stmt
            .query_map(params![user.id], |row| {
                Ok(ResearchSessionSummary {
                    id: row.get(0)?,
                    company: row.get(1)?,
                    model: row.get(2)?,
                    manifest_name: row.get(3)?,
                    status: row.get(4)?,
                    current_phase_id: row.get(5)?,
                    phase_count: row.get(6)?,
                    archived: row.get::<_, i64>(7)? != 0,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                    project_id: row.get(10)?,
                    project_name: row.get(11)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    /// Add a session to a project
    pub fn add_session_to_project(&self, project_id: i64, session_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify both project and session belong to current user
        let project_owned: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![project_id, user.id],
            |row| row.get(0),
        )?;

        if !project_owned {
            return Err(AuthError::Validation("Project not found or access denied".to_string()));
        }

        let session_owned: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?1 AND user_id = ?2)",
            params![session_id, user.id],
            |row| row.get(0),
        )?;

        if !session_owned {
            return Err(AuthError::Validation("Session not found or access denied".to_string()));
        }

        // Insert (ignore if already exists)
        let result = self.conn.execute(
            r#"
            INSERT OR IGNORE INTO project_sessions (project_id, session_id)
            VALUES (?1, ?2)
            "#,
            params![project_id, session_id],
        )?;

        // Update project's updated_at timestamp
        self.conn.execute(
            "UPDATE projects SET updated_at = datetime('now') WHERE id = ?1",
            params![project_id],
        )?;

        Ok(result > 0)
    }

    /// Remove a session from a project
    pub fn remove_session_from_project(&self, project_id: i64, session_id: i64) -> Result<bool, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify project belongs to current user
        let project_owned: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![project_id, user.id],
            |row| row.get(0),
        )?;

        if !project_owned {
            return Err(AuthError::Validation("Project not found or access denied".to_string()));
        }

        let rows_deleted = self.conn.execute(
            "DELETE FROM project_sessions WHERE project_id = ?1 AND session_id = ?2",
            params![project_id, session_id],
        )?;

        Ok(rows_deleted > 0)
    }

    /// Get all sessions in a project (excludes archived sessions)
    pub fn get_project_sessions(&self, project_id: i64) -> Result<Vec<ResearchSessionSummary>, AuthError> {
        let user = self.current_user.as_ref().ok_or(AuthError::NotLoggedIn)?;

        // Verify project belongs to current user
        let project_owned: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id = ?1 AND user_id = ?2)",
            params![project_id, user.id],
            |row| row.get(0),
        )?;

        if !project_owned {
            return Err(AuthError::Validation("Project not found or access denied".to_string()));
        }

        let mut stmt = self.conn.prepare(
            r#"
            SELECT rs.id, rs.company, rs.model, rs.manifest_name, rs.status, rs.current_phase_id,
                   rs.created_at, rs.updated_at,
                   (SELECT COUNT(*) FROM phase_outputs po WHERE po.session_id = rs.id) as phase_count,
                   COALESCE(rs.archived, 0) as archived,
                   ps.project_id,
                   p.name as project_name
            FROM research_sessions rs
            JOIN project_sessions ps ON rs.id = ps.session_id
            JOIN projects p ON ps.project_id = p.id
            WHERE ps.project_id = ?1 AND COALESCE(rs.archived, 0) = 0
            ORDER BY rs.updated_at DESC
            "#,
        )?;

        let sessions = stmt
            .query_map(params![project_id], |row| {
                Ok(ResearchSessionSummary {
                    id: row.get(0)?,
                    company: row.get(1)?,
                    model: row.get(2)?,
                    manifest_name: row.get(3)?,
                    status: row.get(4)?,
                    current_phase_id: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    phase_count: row.get(8)?,
                    archived: row.get::<_, i64>(9)? != 0,
                    project_id: row.get(10)?,
                    project_name: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }
}

// ------------------------------------------------------------------
// Tests
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_manager() -> AuthManager {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        AuthManager::new(&db_path).unwrap()
    }

    #[test]
    fn test_register_and_login() {
        let mut manager = create_test_manager();

        // Register
        let user = manager.register("testuser", "testpass123").unwrap();
        assert_eq!(user.username, "testuser");

        // Login
        let logged_in = manager.login("testuser", "testpass123").unwrap();
        assert_eq!(logged_in.username, "testuser");
        assert!(manager.is_logged_in());

        // Logout
        manager.logout();
        assert!(!manager.is_logged_in());
    }

    #[test]
    fn test_wrong_password() {
        let mut manager = create_test_manager();

        manager.register("testuser", "correctpass").unwrap();

        let result = manager.login("testuser", "wrongpass");
        assert!(matches!(result, Err(AuthError::InvalidCredentials)));
    }

    #[test]
    fn test_duplicate_user() {
        let mut manager = create_test_manager();

        manager.register("testuser", "pass1").unwrap();

        let result = manager.register("testuser", "pass2");
        assert!(matches!(result, Err(AuthError::UserExists)));
    }

    #[test]
    fn test_api_key_storage() {
        let mut manager = create_test_manager();

        manager.register("testuser", "testpass").unwrap();
        manager.login("testuser", "testpass").unwrap();

        // Store API key
        manager.store_api_key(Provider::Anthropic, "sk-ant-test-key-123").unwrap();

        // Retrieve API key
        let key = manager.get_api_key(Provider::Anthropic).unwrap();
        assert_eq!(key, Some("sk-ant-test-key-123".to_string()));

        // Check non-existent key
        let no_key = manager.get_api_key(Provider::DeepSeek).unwrap();
        assert!(no_key.is_none());
    }

    #[test]
    fn test_api_key_list() {
        let mut manager = create_test_manager();

        manager.register("testuser", "testpass").unwrap();
        manager.login("testuser", "testpass").unwrap();

        manager.store_api_key(Provider::Anthropic, "key1").unwrap();
        manager.store_api_key(Provider::Google, "key2").unwrap();

        let keys = manager.list_api_keys().unwrap();

        let anthropic = keys.iter().find(|k| k.provider == "anthropic").unwrap();
        assert!(anthropic.has_key);

        let deepseek = keys.iter().find(|k| k.provider == "deepseek").unwrap();
        assert!(!deepseek.has_key);
    }
}
