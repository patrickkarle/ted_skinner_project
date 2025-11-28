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
}

// ------------------------------------------------------------------
// Data Structures
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    pub provider: String,
    pub has_key: bool,  // Don't expose actual key, just presence
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
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
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
            "#
        )?;

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
        })
    }

    /// Login an existing user
    pub fn login(&mut self, username: &str, password: &str) -> Result<User, AuthError> {
        let username = username.trim();

        // Fetch user and password hash
        let (id, stored_hash): (i64, String) = self.conn.query_row(
            "SELECT id, password_hash FROM users WHERE username = ?1",
            params![username],
            |row| Ok((row.get(0)?, row.get(1)?)),
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
