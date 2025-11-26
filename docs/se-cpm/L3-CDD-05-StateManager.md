# L3-CDD-05: StateManager Component Design Document

**Document ID:** L3-CDD-STATE-001
**Component Name:** StateManager
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L2-ICD-03-ComponentInterfaces.md
**Traceability:** L1-SAD REQ-SYS-005 (State Persistence), SR-009 (Crash Recovery)

---

## 1. Component Overview

### 1.1 Purpose
Provides persistent storage for workflow session state, enabling crash recovery, session history, and resume-from-last-phase functionality using embedded SQLite database.

### 1.2 Responsibilities
- Store session metadata (company, status, timestamps, costs)
- Persist phase completion checkpoints
- Save workflow context (intermediate outputs, LLM responses)
- Enable resume after application crash
- Provide session history queries
- Track cost accumulation across sessions
- Manage database schema migrations

### 1.3 Integration Points
| Component | Interface | Direction |
|-----------|-----------|-----------|
| AgentOrchestrator | `save_phase_completion()`, `resume_session()` | ← Receives persistence requests |
| Tauri IPC | `get_session_history()`, `get_session_output()` | ← Receives query requests |
| SQLite Database | SQL queries | → Stores/retrieves data |
| File System | Database file (fullintel.db) | → Reads/writes |

---

## 2. File Structure

```
src-tauri/src/
├── state/
│   ├── mod.rs                    # Public API + StateManager
│   ├── schema.rs                 # Database schema + migrations
│   ├── models.rs                 # Rust structs for DB rows
│   ├── queries.rs                # SQL query builders
│   └── migrations/
│       ├── v1_initial_schema.sql
│       └── v2_add_cost_tracking.sql (future)
```

### 2.1 Module Dependencies
```rust
// state/mod.rs
mod schema;
mod models;
mod queries;

pub use models::{Session, PhaseCompletion, SessionSummary};
pub use self::StateManager;
```

---

## 3. Data Structures

### 3.1 Core Types

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID (UUID)
    pub session_id: String,

    /// Target company name
    pub company: String,

    /// Session status
    pub status: SessionStatus,

    /// Creation timestamp (Unix ms)
    pub created_at: u64,

    /// Completion timestamp (Unix ms, optional)
    pub completed_at: Option<u64>,

    /// Total workflow duration (ms)
    pub duration_ms: Option<u64>,

    /// Total cost in USD
    pub cost_usd: f64,

    /// Final markdown output (null until phase_5 complete)
    pub markdown_output: Option<String>,

    /// Error message (if failed)
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    /// Currently executing
    Running,

    /// Completed successfully
    Completed,

    /// Failed with error
    Failed,

    /// Paused (for future resume)
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseCompletion {
    /// Session ID (foreign key)
    pub session_id: String,

    /// Phase ID (e.g., "phase_1")
    pub phase_id: String,

    /// Phase completion timestamp
    pub completed_at: u64,

    /// Phase duration (ms)
    pub duration_ms: u64,

    /// Phase output (JSON-encoded)
    pub output: String,

    /// Cost incurred during phase
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub company: String,
    pub created_at: u64,
    pub status: SessionStatus,
    pub duration_ms: Option<u64>,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowContext {
    /// Key-value pairs for workflow state
    pub data: HashMap<String, serde_json::Value>,

    /// Last completed phase
    pub last_completed_phase: Option<String>,

    /// Total cost so far
    pub accumulated_cost: f64,
}
```

---

## 4. Database Schema

### 4.1 Sessions Table

```sql
-- state/migrations/v1_initial_schema.sql
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,
    company TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('Running', 'Completed', 'Failed', 'Paused')),
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    duration_ms INTEGER,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    markdown_output TEXT,
    error_message TEXT
);

CREATE INDEX idx_sessions_created_at ON sessions(created_at DESC);
CREATE INDEX idx_sessions_status ON sessions(status);
```

### 4.2 Phase Completions Table

```sql
CREATE TABLE IF NOT EXISTS phase_completions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    phase_id TEXT NOT NULL,
    completed_at INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    output TEXT NOT NULL,
    cost_usd REAL NOT NULL DEFAULT 0.0,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
    UNIQUE(session_id, phase_id)
);

CREATE INDEX idx_phase_completions_session ON phase_completions(session_id);
```

### 4.3 Workflow Context Table

```sql
CREATE TABLE IF NOT EXISTS workflow_context (
    session_id TEXT PRIMARY KEY,
    context_json TEXT NOT NULL,
    last_updated INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
);
```

---

## 5. StateManager Implementation

### 5.1 Main Struct

```rust
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};

pub struct StateManager {
    /// SQLite connection (thread-safe)
    conn: Arc<Mutex<Connection>>,

    /// Database file path
    db_path: String,
}
```

### 5.2 Constructor

```rust
impl StateManager {
    /// Create new StateManager with database at specified path
    pub fn new(db_path: &str) -> Result<Self> {
        let conn = Connection::open(db_path)
            .context("Failed to open SQLite database")?;

        // Enable WAL mode for better concurrency
        conn.execute("PRAGMA journal_mode=WAL", [])
            .context("Failed to enable WAL mode")?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys=ON", [])
            .context("Failed to enable foreign keys")?;

        let manager = Self {
            conn: Arc::new(Mutex::new(conn)),
            db_path: db_path.to_string(),
        };

        // Run migrations
        manager.run_migrations()?;

        Ok(manager)
    }

    /// Run database migrations
    fn run_migrations(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Create migrations table if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Check current version
        let current_version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Apply migrations
        if current_version < 1 {
            self.apply_migration_v1(&conn)?;
        }

        Ok(())
    }

    fn apply_migration_v1(&self, conn: &Connection) -> Result<()> {
        // Sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                session_id TEXT PRIMARY KEY,
                company TEXT NOT NULL,
                status TEXT NOT NULL CHECK(status IN ('Running', 'Completed', 'Failed', 'Paused')),
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                duration_ms INTEGER,
                cost_usd REAL NOT NULL DEFAULT 0.0,
                markdown_output TEXT,
                error_message TEXT
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_created_at ON sessions(created_at DESC)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)",
            [],
        )?;

        // Phase completions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS phase_completions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                phase_id TEXT NOT NULL,
                completed_at INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                output TEXT NOT NULL,
                cost_usd REAL NOT NULL DEFAULT 0.0,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE,
                UNIQUE(session_id, phase_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_phase_completions_session ON phase_completions(session_id)",
            [],
        )?;

        // Workflow context table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS workflow_context (
                session_id TEXT PRIMARY KEY,
                context_json TEXT NOT NULL,
                last_updated INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(session_id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Record migration
        conn.execute(
            "INSERT INTO schema_migrations (version, applied_at) VALUES (?, ?)",
            params![1, Self::current_timestamp_ms()],
        )?;

        Ok(())
    }
}
```

### 5.3 Session Management

```rust
impl StateManager {
    /// Create new session
    pub fn create_session(&self, company: &str) -> Result<String> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let created_at = Self::current_timestamp_ms();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (session_id, company, status, created_at, cost_usd)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, company, "Running", created_at, 0.0],
        )?;

        Ok(session_id)
    }

    /// Update session status
    pub fn update_session_status(
        &self,
        session_id: &str,
        status: SessionStatus,
        error_message: Option<String>,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        let status_str = match status {
            SessionStatus::Running => "Running",
            SessionStatus::Completed => "Completed",
            SessionStatus::Failed => "Failed",
            SessionStatus::Paused => "Paused",
        };

        if status == SessionStatus::Completed || status == SessionStatus::Failed {
            // Set completion timestamp
            let completed_at = Self::current_timestamp_ms();

            conn.execute(
                "UPDATE sessions
                 SET status = ?1, completed_at = ?2, error_message = ?3
                 WHERE session_id = ?4",
                params![status_str, completed_at, error_message, session_id],
            )?;
        } else {
            conn.execute(
                "UPDATE sessions
                 SET status = ?1, error_message = ?2
                 WHERE session_id = ?3",
                params![status_str, error_message, session_id],
            )?;
        }

        Ok(())
    }

    /// Save final markdown output
    pub fn save_markdown_output(&self, session_id: &str, markdown: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE sessions SET markdown_output = ?1 WHERE session_id = ?2",
            params![markdown, session_id],
        )?;
        Ok(())
    }

    /// Get session by ID
    pub fn get_session(&self, session_id: &str) -> Result<Option<Session>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT session_id, company, status, created_at, completed_at,
                    duration_ms, cost_usd, markdown_output, error_message
             FROM sessions
             WHERE session_id = ?"
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            Ok(Session {
                session_id: row.get(0)?,
                company: row.get(1)?,
                status: Self::parse_status(row.get::<_, String>(2)?),
                created_at: row.get(3)?,
                completed_at: row.get(4)?,
                duration_ms: row.get(5)?,
                cost_usd: row.get(6)?,
                markdown_output: row.get(7)?,
                error_message: row.get(8)?,
            })
        });

        match result {
            Ok(session) => Ok(Some(session)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get session history (most recent first)
    pub fn get_session_history(&self, limit: usize) -> Result<Vec<SessionSummary>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT session_id, company, created_at, status, duration_ms, cost_usd
             FROM sessions
             ORDER BY created_at DESC
             LIMIT ?"
        )?;

        let sessions = stmt.query_map(params![limit], |row| {
            Ok(SessionSummary {
                session_id: row.get(0)?,
                company: row.get(1)?,
                created_at: row.get(2)?,
                status: Self::parse_status(row.get::<_, String>(3)?),
                duration_ms: row.get(4)?,
                cost_usd: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    fn parse_status(s: String) -> SessionStatus {
        match s.as_str() {
            "Running" => SessionStatus::Running,
            "Completed" => SessionStatus::Completed,
            "Failed" => SessionStatus::Failed,
            "Paused" => SessionStatus::Paused,
            _ => SessionStatus::Failed,
        }
    }
}
```

### 5.4 Phase Completion Tracking

```rust
impl StateManager {
    /// Save phase completion
    pub fn save_phase_completion(
        &self,
        session_id: &str,
        phase_id: &str,
        output: &serde_json::Value,
        duration_ms: u64,
        cost_usd: f64,
    ) -> Result<()> {
        let completed_at = Self::current_timestamp_ms();
        let output_json = serde_json::to_string(output)?;

        let conn = self.conn.lock().unwrap();

        // Insert phase completion
        conn.execute(
            "INSERT OR REPLACE INTO phase_completions
             (session_id, phase_id, completed_at, duration_ms, output, cost_usd)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, phase_id, completed_at, duration_ms, output_json, cost_usd],
        )?;

        // Update session cost
        conn.execute(
            "UPDATE sessions
             SET cost_usd = cost_usd + ?1
             WHERE session_id = ?2",
            params![cost_usd, session_id],
        )?;

        Ok(())
    }

    /// Get completed phases for session
    pub fn get_completed_phases(&self, session_id: &str) -> Result<Vec<PhaseCompletion>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT session_id, phase_id, completed_at, duration_ms, output, cost_usd
             FROM phase_completions
             WHERE session_id = ?
             ORDER BY completed_at ASC"
        )?;

        let phases = stmt.query_map(params![session_id], |row| {
            Ok(PhaseCompletion {
                session_id: row.get(0)?,
                phase_id: row.get(1)?,
                completed_at: row.get(2)?,
                duration_ms: row.get(3)?,
                output: row.get(4)?,
                cost_usd: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(phases)
    }

    /// Check if phase is completed
    pub fn is_phase_completed(&self, session_id: &str, phase_id: &str) -> Result<bool> {
        let conn = self.conn.lock().unwrap();

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM phase_completions
             WHERE session_id = ? AND phase_id = ?",
            params![session_id, phase_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }
}
```

### 5.5 Workflow Context Management

```rust
impl StateManager {
    /// Save workflow context (for crash recovery)
    pub fn save_context(
        &self,
        session_id: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        let context_json = serde_json::to_string(context)?;
        let last_updated = Self::current_timestamp_ms();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO workflow_context
             (session_id, context_json, last_updated)
             VALUES (?1, ?2, ?3)",
            params![session_id, context_json, last_updated],
        )?;

        Ok(())
    }

    /// Load workflow context (for resume)
    pub fn load_context(&self, session_id: &str) -> Result<Option<WorkflowContext>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT context_json FROM workflow_context WHERE session_id = ?"
        )?;

        let result = stmt.query_row(params![session_id], |row| {
            let json: String = row.get(0)?;
            Ok(json)
        });

        match result {
            Ok(json) => {
                let data: HashMap<String, serde_json::Value> = serde_json::from_str(&json)?;

                // Get last completed phase
                let phases = self.get_completed_phases(session_id)?;
                let last_completed_phase = phases.last().map(|p| p.phase_id.clone());

                // Get accumulated cost
                let session = self.get_session(session_id)?
                    .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

                Ok(Some(WorkflowContext {
                    data,
                    last_completed_phase,
                    accumulated_cost: session.cost_usd,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Resume session from last completed phase
    pub fn resume_session(&self, session_id: &str) -> Result<Option<WorkflowContext>> {
        // Check session exists and is resumable
        let session = self.get_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if session.status == SessionStatus::Completed {
            return Err(anyhow::anyhow!("Session already completed"));
        }

        // Load context
        self.load_context(session_id)
    }
}
```

### 5.6 Utility Methods

```rust
impl StateManager {
    /// Calculate total workflow duration
    pub fn calculate_duration(&self, session_id: &str) -> Result<u64> {
        let session = self.get_session(session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session not found"))?;

        if let Some(completed_at) = session.completed_at {
            Ok(completed_at - session.created_at)
        } else {
            Ok(Self::current_timestamp_ms() - session.created_at)
        }
    }

    /// Get total sessions count
    pub fn get_total_sessions(&self) -> Result<usize> {
        let conn = self.conn.lock().unwrap();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        )?;
        Ok(count as usize)
    }

    /// Get aggregate statistics
    pub fn get_statistics(&self) -> Result<SessionStatistics> {
        let conn = self.conn.lock().unwrap();

        let total: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0),
        )?;

        let completed: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'Completed'",
            [],
            |row| row.get(0),
        )?;

        let failed: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE status = 'Failed'",
            [],
            |row| row.get(0),
        )?;

        let avg_cost: f64 = conn.query_row(
            "SELECT AVG(cost_usd) FROM sessions WHERE status = 'Completed'",
            [],
            |row| row.get(0),
        ).unwrap_or(0.0);

        let avg_duration: f64 = conn.query_row(
            "SELECT AVG(duration_ms) FROM sessions WHERE status = 'Completed'",
            [],
            |row| row.get(0),
        ).unwrap_or(0.0);

        Ok(SessionStatistics {
            total_sessions: total as usize,
            completed_sessions: completed as usize,
            failed_sessions: failed as usize,
            avg_cost_usd: avg_cost,
            avg_duration_ms: avg_duration as u64,
        })
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatistics {
    pub total_sessions: usize,
    pub completed_sessions: usize,
    pub failed_sessions: usize,
    pub avg_cost_usd: f64,
    pub avg_duration_ms: u64,
}
```

---

## 6. Error Handling

### 6.1 Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] rusqlite::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Session already completed")]
    SessionCompleted,

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Migration failed: {0}")]
    MigrationError(String),
}
```

### 6.2 Recovery Strategy

| Error Type | Recovery Action |
|-----------|----------------|
| DatabaseError | Retry with exponential backoff (3 attempts) |
| SessionNotFound | Return None (valid scenario) |
| SessionCompleted | Fail immediately (invalid operation) |
| SerializationError | Fail immediately (data corruption) |
| MigrationError | Fail startup (require manual intervention) |

---

## 7. Testing Requirements

### 7.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_create_and_retrieve_session() {
        let temp_db = NamedTempFile::new().unwrap();
        let manager = StateManager::new(temp_db.path().to_str().unwrap()).unwrap();

        let session_id = manager.create_session("TechCorp").unwrap();
        let session = manager.get_session(&session_id).unwrap().unwrap();

        assert_eq!(session.company, "TechCorp");
        assert_eq!(session.status, SessionStatus::Running);
        assert_eq!(session.cost_usd, 0.0);
    }

    #[test]
    fn test_phase_completion_tracking() {
        let temp_db = NamedTempFile::new().unwrap();
        let manager = StateManager::new(temp_db.path().to_str().unwrap()).unwrap();

        let session_id = manager.create_session("TechCorp").unwrap();

        let output = serde_json::json!({"result": "Phase 1 complete"});
        manager.save_phase_completion(&session_id, "phase_1", &output, 5000, 0.05).unwrap();

        assert!(manager.is_phase_completed(&session_id, "phase_1").unwrap());
        assert!(!manager.is_phase_completed(&session_id, "phase_2").unwrap());

        let phases = manager.get_completed_phases(&session_id).unwrap();
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].phase_id, "phase_1");
    }

    #[test]
    fn test_context_save_and_resume() {
        let temp_db = NamedTempFile::new().unwrap();
        let manager = StateManager::new(temp_db.path().to_str().unwrap()).unwrap();

        let session_id = manager.create_session("TechCorp").unwrap();

        let mut context = HashMap::new();
        context.insert("target_company".to_string(), serde_json::json!("TechCorp"));
        context.insert("phase_1_output".to_string(), serde_json::json!({"data": "result"}));

        manager.save_context(&session_id, &context).unwrap();

        let loaded = manager.load_context(&session_id).unwrap().unwrap();
        assert_eq!(loaded.data.len(), 2);
        assert_eq!(loaded.data["target_company"], "TechCorp");
    }

    #[test]
    fn test_cost_accumulation() {
        let temp_db = NamedTempFile::new().unwrap();
        let manager = StateManager::new(temp_db.path().to_str().unwrap()).unwrap();

        let session_id = manager.create_session("TechCorp").unwrap();

        manager.save_phase_completion(&session_id, "phase_1", &serde_json::json!({}), 1000, 0.02).unwrap();
        manager.save_phase_completion(&session_id, "phase_2", &serde_json::json!({}), 1000, 0.03).unwrap();
        manager.save_phase_completion(&session_id, "phase_3", &serde_json::json!({}), 1000, 0.01).unwrap();

        let session = manager.get_session(&session_id).unwrap().unwrap();
        assert_eq!(session.cost_usd, 0.06);
    }
}
```

### 7.2 Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_crash_recovery_workflow() {
        let temp_db = NamedTempFile::new().unwrap();
        let db_path = temp_db.path().to_str().unwrap();

        // Simulate workflow execution
        {
            let manager = StateManager::new(db_path).unwrap();
            let session_id = manager.create_session("TechCorp").unwrap();

            // Complete phase 1
            manager.save_phase_completion(
                &session_id,
                "phase_1",
                &serde_json::json!({"profile": "data"}),
                5000,
                0.02,
            ).unwrap();

            // Save context
            let mut context = HashMap::new();
            context.insert("target_company".to_string(), serde_json::json!("TechCorp"));
            context.insert("phase_1_output".to_string(), serde_json::json!({"profile": "data"}));
            manager.save_context(&session_id, &context).unwrap();

            // Simulate crash (drop manager)
        }

        // Resume after crash
        {
            let manager = StateManager::new(db_path).unwrap();
            let sessions = manager.get_session_history(10).unwrap();
            assert_eq!(sessions.len(), 1);

            let session_id = &sessions[0].session_id;
            let context = manager.resume_session(session_id).unwrap().unwrap();

            assert_eq!(context.last_completed_phase, Some("phase_1".to_string()));
            assert_eq!(context.data.len(), 2);
            assert_eq!(context.accumulated_cost, 0.02);
        }
    }
}
```

---

## 8. Performance Requirements

| Metric | Target | Validation Method |
|--------|--------|------------------|
| **Session Creation** | < 10ms | Measure 1000 creates |
| **Phase Save** | < 20ms | Measure 1000 saves |
| **Context Load** | < 50ms | Measure 100 loads |
| **History Query** | < 100ms | Query 500 sessions |
| **Database Size** | < 100MB per 1000 sessions | Measure actual size |
| **WAL Checkpoint** | < 1s | Monitor background checkpoints |

---

## 9. Security Requirements

### 9.1 Data Protection
- **File Permissions**: Database file owned by application user only (chmod 600)
- **No Sensitive Data**: API keys NOT stored in database (environment only)
- **SQL Injection**: All queries use parameterized statements
- **Backup Strategy**: Daily backups to separate directory

### 9.2 Data Retention
```rust
impl StateManager {
    /// Delete sessions older than specified days
    pub fn cleanup_old_sessions(&self, days: u32) -> Result<usize> {
        let cutoff = Self::current_timestamp_ms() - (days as u64 * 86400 * 1000);

        let conn = self.conn.lock().unwrap();
        let deleted = conn.execute(
            "DELETE FROM sessions WHERE created_at < ? AND status IN ('Completed', 'Failed')",
            params![cutoff],
        )?;

        Ok(deleted)
    }
}
```

---

## 10. Database Maintenance

### 10.1 Vacuum and Optimize

```rust
impl StateManager {
    /// Vacuum database to reclaim space
    pub fn vacuum(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("VACUUM", [])?;
        Ok(())
    }

    /// Optimize database (analyze tables)
    pub fn optimize(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("ANALYZE", [])?;
        Ok(())
    }

    /// Checkpoint WAL file
    pub fn checkpoint(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", [])?;
        Ok(())
    }
}
```

---

## 11. Traceability Matrix

| L2 Interface Requirement | Implementation Element | Validation |
|-------------------------|----------------------|------------|
| ICD-03: Session schema | `Session` struct + SQL schema | Unit test serialization |
| ICD-03: save_phase_completion | `save_phase_completion()` method | Integration test |
| ICD-03: resume_session | `resume_session()` method | Crash recovery test |
| L1-SAD REQ-SYS-005 | SQLite persistence | Performance test |
| L1-SAD SR-009 (95% recovery) | Context save/load | Crash simulation test |

---

## 12. Future Enhancements

### 12.1 Export/Import Sessions
```rust
pub fn export_session(&self, session_id: &str, path: &str) -> Result<()> {
    // Export session + phases to JSON file
    unimplemented!("Session export - post-MVP")
}

pub fn import_session(&self, path: &str) -> Result<String> {
    // Import session from JSON file
    unimplemented!("Session import - post-MVP")
}
```

### 12.2 Analytics Queries
```rust
pub fn get_cost_trends(&self, days: u32) -> Result<Vec<(u64, f64)>> {
    // Return (timestamp, avg_cost) per day
    unimplemented!("Cost analytics - post-MVP")
}
```

---

**Document Status:** Complete - Ready for L3-CDD-06-FrontendComponents
**Next Document:** L3-CDD-06-FrontendComponents.md
