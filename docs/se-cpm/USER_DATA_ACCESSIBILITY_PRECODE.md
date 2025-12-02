# PRE-CODE: User Data Accessibility & Session Management
## CDP LODA Sprint 2 - Phase 5 Interface Control Documents (ICD)

**Date:** 2025-11-28
**Status:** PRE-CODE In Progress
**Sprint:** User Data Accessibility
**Source:** USER_DATA_ACCESSIBILITY_PLAN.md

---

## 1. Purpose

This document provides exact Interface Control Documents (ICDs) for mechanical translation during IMPLEMENT phase. All line numbers verified against actual source code.

**Key Principle:** Implementation should require ZERO decisions - every field, type, and signature is specified exactly.

---

## 2. Verified Source Locations

### 2.1 Rust Backend Sources

| Component | File | Verified Lines | Last Verified |
|-----------|------|----------------|---------------|
| PhaseOutputPayload struct | src-tauri/src/agent.rs | 33-40 | 2025-11-28 |
| emit_phase_output fn | src-tauri/src/agent.rs | 157-177 | 2025-11-28 |
| save_phase_output Tauri cmd | src-tauri/src/main.rs | 476-497 | 2025-11-28 |
| save_phase_output impl | src-tauri/src/auth.rs | 1185-1219 | 2025-11-28 |
| ALTER TABLE pattern | src-tauri/src/auth.rs | 422-425 | 2025-11-28 |

### 2.2 Frontend Sources

| Component | File | Verified Lines | Last Verified |
|-----------|------|----------------|---------------|
| PhaseOutputPayload type | src/App.tsx | 70-78 | 2025-11-28 |
| PhaseOutputRecord type | src/App.tsx | 92-102 | 2025-11-28 |
| Event listener | src/App.tsx | 295-310 | 2025-11-28 |

---

## 3. ICD-001: PhaseOutputPayload Extension

### 3.1 Contract ID
**ICD-5001-5002**: PhaseOutputPayload struct extension for system_prompt and user_input fields

### 3.2 Current State (agent.rs:33-40)

```rust
/// Payload for phase completion with output - enables frontend session persistence
#[derive(Clone, Serialize)]
struct PhaseOutputPayload {
    session_id: Option<i64>,
    phase_id: String,
    phase_name: String,
    status: String, // "running", "completed", "failed"
    output: Option<String>,
    error: Option<String>,
}
```

### 3.3 Target State

```rust
/// Payload for phase completion with output - enables frontend session persistence
/// Extended for user data accessibility (IM-5001, IM-5002)
#[derive(Clone, Serialize)]
struct PhaseOutputPayload {
    session_id: Option<i64>,
    phase_id: String,
    phase_name: String,
    status: String, // "running", "completed", "failed"
    system_prompt: Option<String>,  // IM-5001: System prompt sent to LLM
    user_input: Option<String>,     // IM-5002: User input/manifest data sent to LLM
    output: Option<String>,
    error: Option<String>,
}
```

### 3.4 Field Contract

| Field | IM Code | Type | Nullable | Populated When |
|-------|---------|------|----------|----------------|
| system_prompt | IM-5001 | Option<String> | Yes | status="running" |
| user_input | IM-5002 | Option<String> | Yes | status="running" |

### 3.5 Serialization Contract

```json
{
  "session_id": 123,
  "phase_id": "phase_1",
  "phase_name": "Research",
  "status": "running",
  "system_prompt": "You are a research assistant...",
  "user_input": "Analyze the following topic...",
  "output": null,
  "error": null
}
```

---

## 4. ICD-002: emit_phase_output_with_prompts

### 4.1 Contract ID
**ICD-5003**: Agent method to emit phase output events with prompt data

### 4.2 Current State (agent.rs:157-177)

```rust
fn emit_phase_output(
    &self,
    phase_id: &str,
    phase_name: &str,
    status: &str,
    output: Option<&str>,
    error: Option<&str>,
) {
    if let Some(app) = &self.app_handle {
        match app.emit("phase-output", PhaseOutputPayload {
            session_id: self.session_id,
            phase_id: phase_id.to_string(),
            phase_name: phase_name.to_string(),
            status: status.to_string(),
            output: output.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
        }) {
            Ok(_) => println!("[AGENT-EMIT] Phase output: {} -> {} (session: {:?})", phase_id, status, self.session_id),
            Err(e) => eprintln!("[AGENT-EMIT-ERROR] Failed to emit phase-output: {}", e),
        }
    }
}
```

### 4.3 Target State

```rust
/// Emit phase output event with optional prompt data for persistence
/// IM-5003: Extended for user data accessibility
fn emit_phase_output_with_prompts(
    &self,
    phase_id: &str,
    phase_name: &str,
    status: &str,
    system_prompt: Option<&str>,
    user_input: Option<&str>,
    output: Option<&str>,
    error: Option<&str>,
) {
    if let Some(app) = &self.app_handle {
        match app.emit("phase-output", PhaseOutputPayload {
            session_id: self.session_id,
            phase_id: phase_id.to_string(),
            phase_name: phase_name.to_string(),
            status: status.to_string(),
            system_prompt: system_prompt.map(|s| s.to_string()),
            user_input: user_input.map(|s| s.to_string()),
            output: output.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
        }) {
            Ok(_) => println!("[AGENT-EMIT] Phase output: {} -> {} (session: {:?})", phase_id, status, self.session_id),
            Err(e) => eprintln!("[AGENT-EMIT-ERROR] Failed to emit phase-output: {}", e),
        }
    }
}
```

### 4.4 Call Site Updates

| Location | Current Call | Updated Call |
|----------|--------------|--------------|
| execute_phase (running) | `emit_phase_output(id, name, "running", None, None)` | `emit_phase_output_with_prompts(id, name, "running", Some(&system_prompt), Some(&user_input), None, None)` |
| execute_phase (completed) | `emit_phase_output(id, name, "completed", Some(&output), None)` | `emit_phase_output_with_prompts(id, name, "completed", None, None, Some(&output), None)` |
| execute_phase (failed) | `emit_phase_output(id, name, "failed", None, Some(&error))` | `emit_phase_output_with_prompts(id, name, "failed", None, None, None, Some(&error))` |

### 4.5 Backward Compatibility

**Option A (Recommended):** Replace `emit_phase_output` with `emit_phase_output_with_prompts` and update all call sites.

**Option B (Alternative):** Keep both methods, with `emit_phase_output` calling `emit_phase_output_with_prompts` with None for prompts.

---

## 5. ICD-003: save_phase_output Extension

### 5.1 Contract ID
**ICD-5010**: Tauri command to persist phase output with prompt data

### 5.2 Current State (main.rs:476-497)

```rust
#[tauri::command]
async fn save_phase_output(
    session_id: i64,
    phase_id: String,
    phase_name: String,
    status: String,
    output: Option<String>,
    error: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .save_phase_output(
            session_id,
            &phase_id,
            &phase_name,
            &status,
            output.as_deref(),
            error.as_deref(),
        )
        .map_err(|e| e.to_string())
}
```

### 5.3 Target State

```rust
/// Persist phase output with optional prompt data
/// IM-5010: Extended for user data accessibility
#[tauri::command]
async fn save_phase_output(
    session_id: i64,
    phase_id: String,
    phase_name: String,
    status: String,
    system_prompt: Option<String>,  // IM-5001
    user_input: Option<String>,     // IM-5002
    output: Option<String>,
    error: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .save_phase_output(
            session_id,
            &phase_id,
            &phase_name,
            &status,
            system_prompt.as_deref(),
            user_input.as_deref(),
            output.as_deref(),
            error.as_deref(),
        )
        .map_err(|e| e.to_string())
}
```

### 5.4 Parameter Contract

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| session_id | i64 | Yes | FK to research_sessions.id |
| phase_id | String | Yes | Phase identifier (e.g., "phase_1") |
| phase_name | String | Yes | Human-readable phase name |
| status | String | Yes | "running", "completed", or "failed" |
| system_prompt | Option<String> | No | System prompt sent to LLM |
| user_input | Option<String> | No | User input sent to LLM |
| output | Option<String> | No | LLM response output |
| error | Option<String> | No | Error message if failed |

---

## 6. ICD-004: save_phase_output SQL Implementation

### 6.1 Contract ID
**ICD-5010-SQL**: SQL implementation with COALESCE preservation

### 6.2 Current State (auth.rs:1185-1219)

```rust
pub fn save_phase_output(
    &self,
    session_id: i64,
    phase_id: &str,
    phase_name: &str,
    status: &str,
    output: Option<&str>,
    error: Option<&str>,
) -> Result<i64, AuthError> {
    // ...validation...
    self.conn.execute(
        r#"
        INSERT INTO phase_outputs (session_id, phase_id, phase_name, status, output, error)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(session_id, phase_id) DO UPDATE SET
            status = excluded.status,
            output = COALESCE(excluded.output, phase_outputs.output),
            error = excluded.error,
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![session_id, phase_id, phase_name, status, output, error],
    )?;
    // ...
}
```

### 6.3 Target State

```rust
/// Save phase output with prompt preservation via COALESCE
/// IM-5010: Extended for user data accessibility
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
    // Validate session exists
    let session_exists: bool = self.conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM research_sessions WHERE id = ?)",
        [session_id],
        |row| row.get(0),
    )?;

    if !session_exists {
        return Err(AuthError::NotFound(format!("Session {} not found", session_id)));
    }

    self.conn.execute(
        r#"
        INSERT INTO phase_outputs (
            session_id, phase_id, phase_name, status,
            system_prompt, user_input,
            output, error, created_at, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        ON CONFLICT(session_id, phase_id) DO UPDATE SET
            status = excluded.status,
            system_prompt = COALESCE(excluded.system_prompt, phase_outputs.system_prompt),
            user_input = COALESCE(excluded.user_input, phase_outputs.user_input),
            output = COALESCE(excluded.output, phase_outputs.output),
            error = excluded.error,
            updated_at = CURRENT_TIMESTAMP
        "#,
        params![
            session_id, phase_id, phase_name, status,
            system_prompt, user_input,
            output, error
        ],
    )?;

    Ok(self.conn.last_insert_rowid())
}
```

### 6.4 COALESCE Preservation Logic

```
Scenario: "running" event followed by "completed" event

Event 1 (running):
  system_prompt = "You are..."
  user_input = "Analyze..."
  output = NULL

Event 2 (completed):
  system_prompt = NULL
  user_input = NULL
  output = "The analysis shows..."

Result after COALESCE:
  system_prompt = "You are..."     <- Preserved from Event 1
  user_input = "Analyze..."        <- Preserved from Event 1
  output = "The analysis shows..." <- From Event 2
```

---

## 7. ICD-005: Database Schema Migration

### 7.1 Contract ID
**ICD-5001-5002-MIGRATE**: Idempotent schema migration for phase_outputs

### 7.2 Migration Script (auth.rs ensure_tables)

```rust
// Location: auth.rs, within ensure_tables() method
// Pattern reference: auth.rs:422-425

// IM-5001: Add system_prompt column
let _ = self.conn.execute(
    "ALTER TABLE phase_outputs ADD COLUMN system_prompt TEXT",
    [],
);

// IM-5002: Add user_input column
let _ = self.conn.execute(
    "ALTER TABLE phase_outputs ADD COLUMN user_input TEXT",
    [],
);
```

### 7.3 Idempotency Contract

- `let _ =` ignores errors (column may already exist)
- NULL default allows backward compatibility
- No data loss - existing rows get NULL for new columns
- Safe to run on every app startup

### 7.4 Verification Query

```sql
-- Run after migration to verify
PRAGMA table_info(phase_outputs);

-- Expected output includes:
-- ...
-- system_prompt | TEXT | 0 | NULL | 0
-- user_input    | TEXT | 0 | NULL | 0
```

---

## 8. ICD-006: session_conversations Table

### 8.1 Contract ID
**ICD-5030**: New table for session-level conversation tracking

### 8.2 CREATE TABLE Statement

```sql
-- Location: auth.rs, within ensure_tables() method
-- IM-5030: Session-level conversation tracking

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
```

### 8.3 Column Contract

| Column | Type | Constraint | Nullable | Description |
|--------|------|------------|----------|-------------|
| id | INTEGER | PRIMARY KEY AUTOINCREMENT | No | Unique message ID |
| session_id | INTEGER | FK → research_sessions(id) | No | Parent session |
| phase_id | TEXT | None | Yes | Optional phase association |
| role | TEXT | CHECK (IN ('user','assistant','system')) | No | Message role |
| content | TEXT | None | No | Message content |
| created_at | TEXT | DEFAULT datetime('now') | No | ISO8601 timestamp |

### 8.4 Rust Implementation

```rust
// In ensure_tables()
self.conn.execute(
    r#"
    CREATE TABLE IF NOT EXISTS session_conversations (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        session_id INTEGER NOT NULL,
        phase_id TEXT,
        role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
        content TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now')),
        FOREIGN KEY (session_id) REFERENCES research_sessions(id) ON DELETE CASCADE
    )
    "#,
    [],
)?;

self.conn.execute(
    "CREATE INDEX IF NOT EXISTS idx_session_conversations_session_id ON session_conversations(session_id)",
    [],
)?;
```

---

## 9. ICD-007: add_session_message Command

### 9.1 Contract ID
**ICD-5031**: Tauri command to add messages to session conversation

### 9.2 Command Signature

```rust
/// Add a message to session conversation history
/// IM-5031: Session message persistence
#[tauri::command]
pub async fn add_session_message(
    session_id: i64,
    phase_id: Option<String>,
    role: String,
    content: String,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    // Validate role
    if !["user", "assistant", "system"].contains(&role.as_str()) {
        return Err(format!("Invalid role: {}. Must be 'user', 'assistant', or 'system'", role));
    }

    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .add_session_message(session_id, phase_id.as_deref(), &role, &content)
        .map_err(|e| e.to_string())
}
```

### 9.3 SQL Implementation

```rust
/// Insert message into session_conversations
pub fn add_session_message(
    &self,
    session_id: i64,
    phase_id: Option<&str>,
    role: &str,
    content: &str,
) -> Result<i64, AuthError> {
    self.conn.execute(
        r#"
        INSERT INTO session_conversations (session_id, phase_id, role, content, created_at)
        VALUES (?1, ?2, ?3, ?4, datetime('now'))
        "#,
        params![session_id, phase_id, role, content],
    )?;

    Ok(self.conn.last_insert_rowid())
}
```

### 9.4 Error Contract

| Condition | Error Type | Message |
|-----------|------------|---------|
| Invalid role | String | "Invalid role: {role}. Must be 'user', 'assistant', or 'system'" |
| Session not found | AuthError | FK constraint violation |
| Lock failure | String | "Failed to lock auth state" |

---

## 10. ICD-008: get_session_conversation Command

### 10.1 Contract ID
**ICD-5032**: Tauri command to retrieve session conversation history

### 10.2 SessionMessage Struct

```rust
/// Message in session conversation
/// IM-5032: Session conversation retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: i64,
    pub session_id: i64,
    pub phase_id: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
```

### 10.3 Command Signature

```rust
/// Get conversation history for a session
/// IM-5032: Optional phase_id filter
#[tauri::command]
pub async fn get_session_conversation(
    session_id: i64,
    phase_id: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<Vec<SessionMessage>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .get_session_conversation(session_id, phase_id.as_deref())
        .map_err(|e| e.to_string())
}
```

### 10.4 SQL Implementation

```rust
/// Retrieve session conversation with optional phase filter
pub fn get_session_conversation(
    &self,
    session_id: i64,
    phase_id: Option<&str>,
) -> Result<Vec<SessionMessage>, AuthError> {
    let mut stmt = self.conn.prepare(
        r#"
        SELECT id, session_id, phase_id, role, content, created_at
        FROM session_conversations
        WHERE session_id = ?1
          AND (?2 IS NULL OR phase_id = ?2)
        ORDER BY created_at ASC
        "#,
    )?;

    let messages = stmt.query_map(params![session_id, phase_id], |row| {
        Ok(SessionMessage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            phase_id: row.get(2)?,
            role: row.get(3)?,
            content: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}
```

---

## 11. ICD-009: get_phase_outputs Extension

### 11.1 Contract ID
**ICD-5011**: Extended phase outputs retrieval with prompt data

### 11.2 PhaseOutputRecord Struct

```rust
/// Phase output record with prompt data
/// IM-5011: Extended for user data accessibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseOutputRecord {
    pub id: i64,
    pub session_id: i64,
    pub phase_id: String,
    pub phase_name: String,
    pub status: String,
    pub system_prompt: Option<String>,  // IM-5001
    pub user_input: Option<String>,     // IM-5002
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

### 11.3 SQL Query

```rust
pub fn get_phase_outputs(&self, session_id: i64) -> Result<Vec<PhaseOutputRecord>, AuthError> {
    let mut stmt = self.conn.prepare(
        r#"
        SELECT id, session_id, phase_id, phase_name, status,
               system_prompt, user_input,
               output, error, created_at, updated_at
        FROM phase_outputs
        WHERE session_id = ?
        ORDER BY created_at ASC
        "#,
    )?;

    let outputs = stmt.query_map([session_id], |row| {
        Ok(PhaseOutputRecord {
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
    .collect::<Result<Vec<_>, _>>()?;

    Ok(outputs)
}
```

---

## 12. ICD-010: reconstruct_session_context

### 12.1 Contract ID
**ICD-5021**: Agent method to reconstruct conversation context for session resume

### 12.2 Function Signature

```rust
/// Reconstruct conversation context from phase outputs for session resume
/// IM-5021: Uses sliding window (default 25 pairs)
fn reconstruct_session_context(
    phase_outputs: &[PhaseOutputRecord],
    max_pairs: usize,
) -> Vec<ChatMessage> {
    // Filter completed phases with prompts
    let pairs: Vec<(ChatMessage, ChatMessage)> = phase_outputs
        .iter()
        .filter(|p| p.status == "completed" && p.user_input.is_some())
        .map(|p| (
            ChatMessage {
                role: ChatRole::User,
                content: p.user_input.clone().unwrap_or_default(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: p.output.clone().unwrap_or_default(),
            },
        ))
        .collect();

    // Apply sliding window
    let window_start = pairs.len().saturating_sub(max_pairs);
    pairs[window_start..]
        .iter()
        .flat_map(|(user, assistant)| vec![user.clone(), assistant.clone()])
        .collect()
}
```

### 12.3 Sliding Window Contract

| Scenario | Input Pairs | max_pairs | Output Messages |
|----------|-------------|-----------|-----------------|
| Under limit | 10 pairs | 25 | 20 messages (all) |
| At limit | 25 pairs | 25 | 50 messages (all) |
| Over limit | 50 pairs | 25 | 50 messages (last 25 pairs) |
| Empty | 0 pairs | 25 | 0 messages |

### 12.4 Token Estimation

- Assumption: ~1K tokens per message pair
- 25 pairs ≈ 50K tokens
- Safe for DeepSeek (64K), GPT-4o (128K), Claude (200K)

---

## 13. ICD-011: resume_research_session Command

### 13.1 Contract ID
**ICD-5020**: Tauri command to resume paused research session

### 13.2 Result Types

```rust
/// Result of session resume operation
/// IM-5020: Session resume with context reconstruction
#[derive(Debug, Serialize)]
pub struct ResumeSessionResult {
    pub session: ResearchSession,
    pub next_phase_id: String,
    pub context: SessionContext,
}

#[derive(Debug, Serialize)]
pub struct SessionContext {
    pub history: Vec<ChatMessage>,
    pub last_completed_phase: String,
    pub total_phases: usize,
    pub completed_phases: usize,
}
```

### 13.3 Command Signature

```rust
/// Resume a paused research session with full context
/// IM-5020: Requires IM-5011, IM-5021
#[tauri::command]
pub async fn resume_research_session(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<ResumeSessionResult, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;

    // 1. Load session
    let session = manager.get_research_session(session_id)
        .map_err(|e| format!("Session {} not found: {}", session_id, e))?;

    // 2. Validate status
    match session.status.as_str() {
        "completed" => return Err(format!("Session {} already completed", session_id)),
        "failed" => return Err(format!("Session {} failed and cannot be resumed", session_id)),
        "in_progress" => {}, // OK to resume
        _ => return Err(format!("Session {} has invalid status: {}", session_id, session.status)),
    }

    // 3. Load phase outputs
    let phase_outputs = manager.get_phase_outputs(session_id)
        .map_err(|e| e.to_string())?;

    // 4. Find last completed phase
    let completed_phases: Vec<_> = phase_outputs.iter()
        .filter(|p| p.status == "completed")
        .collect();

    if completed_phases.is_empty() {
        return Err(format!("Session {} has no completed phases to resume from", session_id));
    }

    let last_completed = completed_phases.last().unwrap();

    // 5. Determine next phase (manifest-dependent)
    let next_phase_id = determine_next_phase(&last_completed.phase_id);

    // 6. Reconstruct context
    let history = reconstruct_session_context(&phase_outputs, 25);

    Ok(ResumeSessionResult {
        session,
        next_phase_id,
        context: SessionContext {
            history,
            last_completed_phase: last_completed.phase_id.clone(),
            total_phases: 7, // Manifest-specific
            completed_phases: completed_phases.len(),
        },
    })
}
```

### 13.4 Error Contract

| Condition | Error Message |
|-----------|---------------|
| Session not found | "Session {id} not found: {error}" |
| Session completed | "Session {id} already completed" |
| Session failed | "Session {id} failed and cannot be resumed" |
| Invalid status | "Session {id} has invalid status: {status}" |
| No completed phases | "Session {id} has no completed phases to resume from" |
| Lock failure | "Failed to lock auth state" |

---

## 14. ICD-012: TypeScript Types

### 14.1 Contract ID
**ICD-TS-001**: Frontend TypeScript type definitions

### 14.2 PhaseOutputPayload (App.tsx:70-78 update)

```typescript
// Current location: src/App.tsx:70-78
// IM-5001, IM-5002: Extended for prompt data

interface PhaseOutputPayload {
  session_id: number | null;
  phase_id: string;
  phase_name: string;
  status: 'running' | 'completed' | 'failed';
  system_prompt: string | null;  // IM-5001
  user_input: string | null;     // IM-5002
  output: string | null;
  error: string | null;
}
```

### 14.3 PhaseOutputRecord (App.tsx:92-102 update)

```typescript
// Current location: src/App.tsx:92-102
// IM-5011: Extended for prompt retrieval

interface PhaseOutputRecord {
  id: number;
  session_id: number;
  phase_id: string;
  phase_name: string;
  status: string;
  system_prompt: string | null;  // IM-5001
  user_input: string | null;     // IM-5002
  output: string | null;
  error: string | null;
  created_at: string;
  updated_at: string;
}
```

### 14.4 SessionMessage (new type)

```typescript
// IM-5032: Session conversation message

interface SessionMessage {
  id: number;
  session_id: number;
  phase_id: string | null;
  role: 'user' | 'assistant' | 'system';
  content: string;
  created_at: string;
}
```

### 14.5 ResumeSessionResult (new type)

```typescript
// IM-5020: Session resume result

interface ResumeSessionResult {
  session: ResearchSession;
  next_phase_id: string;
  context: {
    history: ChatMessage[];
    last_completed_phase: string;
    total_phases: number;
    completed_phases: number;
  };
}
```

---

## 15. ICD-013: Event Listener Update

### 15.1 Contract ID
**ICD-EVT-001**: Phase output event listener with prompt handling

### 15.2 Current State (App.tsx:295-310)

```typescript
const unlistenPhaseOutput = listen<PhaseOutputPayload>("phase-output", async (event) => {
  const { session_id, phase_id, phase_name, status, output, error } = event.payload;
  console.log("[DEBUG] phase-output received:", { session_id, phase_id, status });

  if (session_id !== null) {
    try {
      await invoke("save_phase_output", {
        sessionId: session_id,
        phaseId: phase_id,
        phaseName: phase_name,
        status,
        output,
        error,
      });
    } catch (err) {
      console.error("[ERROR] Failed to save phase output:", err);
    }
  }
});
```

### 15.3 Target State

```typescript
// IM-5003: Extended event listener with prompt data
const unlistenPhaseOutput = listen<PhaseOutputPayload>("phase-output", async (event) => {
  const {
    session_id,
    phase_id,
    phase_name,
    status,
    system_prompt,  // IM-5001
    user_input,     // IM-5002
    output,
    error
  } = event.payload;

  console.log("[DEBUG] phase-output received:", { session_id, phase_id, status });

  if (session_id !== null) {
    try {
      await invoke("save_phase_output", {
        sessionId: session_id,
        phaseId: phase_id,
        phaseName: phase_name,
        status,
        systemPrompt: system_prompt,  // IM-5001
        userInput: user_input,        // IM-5002
        output,
        error,
      });
    } catch (err) {
      console.error("[ERROR] Failed to save phase output:", err);
    }
  }
});
```

### 15.4 Parameter Naming Contract

| Event Payload (snake_case) | Invoke Parameter (camelCase) |
|---------------------------|------------------------------|
| session_id | sessionId |
| phase_id | phaseId |
| phase_name | phaseName |
| system_prompt | systemPrompt |
| user_input | userInput |

---

## 16. ICD-014: Frontend Components

### 16.1 PromptViewCard (IM-5041)

```typescript
// Location: src/components/PromptViewCard.tsx (new file)

interface PromptViewCardProps {
  output: PhaseOutputRecord;
  expanded: boolean;
  onToggle: () => void;
}

const PromptViewCard: React.FC<PromptViewCardProps> = ({
  output,
  expanded,
  onToggle,
}) => (
  <div className="prompt-view-card">
    <header onClick={onToggle} className="prompt-header">
      <span className="phase-name">{output.phase_name}</span>
      <span className={`status status-${output.status}`}>{output.status}</span>
      <span className="chevron">{expanded ? '▼' : '▶'}</span>
    </header>

    {expanded && (output.system_prompt || output.user_input) && (
      <div className="prompt-content">
        {output.system_prompt && (
          <div className="system-prompt">
            <h4>System Prompt</h4>
            <pre>{output.system_prompt}</pre>
          </div>
        )}
        {output.user_input && (
          <div className="user-input">
            <h4>User Input</h4>
            <pre>{output.user_input}</pre>
          </div>
        )}
      </div>
    )}

    {output.output && (
      <div className="output-section">
        <h4>Output</h4>
        <pre>{output.output}</pre>
      </div>
    )}

    {output.error && (
      <div className="error-section">
        <h4>Error</h4>
        <pre className="error">{output.error}</pre>
      </div>
    )}
  </div>
);
```

### 16.2 ResumeSessionButton (IM-5042)

```typescript
// Location: src/components/ResumeSessionButton.tsx (new file)

interface ResumeSessionButtonProps {
  sessionId: number;
  onResume: (result: ResumeSessionResult) => void;
  disabled?: boolean;
}

const ResumeSessionButton: React.FC<ResumeSessionButtonProps> = ({
  sessionId,
  onResume,
  disabled = false,
}) => {
  const [isLoading, setIsLoading] = useState(false);

  const handleClick = async () => {
    setIsLoading(true);
    try {
      const result = await invoke<ResumeSessionResult>('resume_research_session', {
        sessionId,
      });
      onResume(result);
    } catch (error) {
      console.error('Failed to resume session:', error);
      // Show error notification
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <button
      className="resume-button"
      onClick={handleClick}
      disabled={disabled || isLoading}
    >
      {isLoading ? 'Resuming...' : '▶ Resume Session'}
    </button>
  );
};
```

### 16.3 SessionDetailPanel (IM-5040)

```typescript
// Location: src/components/SessionDetailPanel.tsx (new file)

interface SessionDetailPanelProps {
  session: ResearchSession;
  phaseOutputs: PhaseOutputRecord[];
  onResume: (result: ResumeSessionResult) => void;
  onClose: () => void;
}

const SessionDetailPanel: React.FC<SessionDetailPanelProps> = ({
  session,
  phaseOutputs,
  onResume,
  onClose,
}) => {
  const [expandedPrompts, setExpandedPrompts] = useState<Set<number>>(new Set());

  const togglePrompt = (id: number) => {
    setExpandedPrompts(prev => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  return (
    <div className="session-detail-panel">
      <header className="panel-header">
        <h2>{session.manifest_name}</h2>
        <span className={`status-badge status-${session.status}`}>
          {session.status}
        </span>
        <button className="close-button" onClick={onClose}>×</button>
      </header>

      <section className="phase-outputs">
        <h3>Phase Outputs ({phaseOutputs.length})</h3>
        {phaseOutputs.map(output => (
          <PromptViewCard
            key={output.id}
            output={output}
            expanded={expandedPrompts.has(output.id)}
            onToggle={() => togglePrompt(output.id)}
          />
        ))}
      </section>

      {session.status === 'in_progress' && (
        <footer className="panel-footer">
          <ResumeSessionButton
            sessionId={session.id}
            onResume={onResume}
          />
        </footer>
      )}
    </div>
  );
};
```

---

## 17. Implementation Order

```
PHASE 1: Database Schema (Prerequisite)
├── IM-5001, IM-5002: ALTER TABLE phase_outputs (ICD-005)
└── IM-5030: CREATE TABLE session_conversations (ICD-006)

PHASE 2: Backend - Structs & Payloads
├── IM-5001, IM-5002: PhaseOutputPayload extension (ICD-001)
├── IM-5011: PhaseOutputRecord struct (ICD-009)
└── IM-5032: SessionMessage struct (ICD-008)

PHASE 3: Backend - Core Commands
├── IM-5003: emit_phase_output_with_prompts (ICD-002)
├── IM-5010: save_phase_output update (ICD-003, ICD-004)
├── IM-5011: get_phase_outputs update (ICD-009)
├── IM-5031: add_session_message (ICD-007)
└── IM-5032: get_session_conversation (ICD-008)

PHASE 4: Backend - Session Resume
├── IM-5021: reconstruct_session_context (ICD-010)
└── IM-5020: resume_research_session (ICD-011)

PHASE 5: Frontend - Types
├── PhaseOutputPayload type (ICD-012)
├── PhaseOutputRecord type (ICD-012)
├── SessionMessage type (ICD-012)
└── ResumeSessionResult type (ICD-012)

PHASE 6: Frontend - Event Handler
└── Event listener update (ICD-013)

PHASE 7: Frontend - Components
├── IM-5041: PromptViewCard (ICD-014)
├── IM-5042: ResumeSessionButton (ICD-014)
└── IM-5040: SessionDetailPanel (ICD-014)
```

---

## 18. Traceability Matrix

| IM Code | ICD Contract | Status |
|---------|--------------|--------|
| IM-5001 | ICD-001, ICD-004, ICD-005, ICD-012 | Specified |
| IM-5002 | ICD-001, ICD-004, ICD-005, ICD-012 | Specified |
| IM-5003 | ICD-002 | Specified |
| IM-5010 | ICD-003, ICD-004 | Specified |
| IM-5011 | ICD-009 | Specified |
| IM-5020 | ICD-011 | Specified |
| IM-5021 | ICD-010 | Specified |
| IM-5030 | ICD-006 | Specified |
| IM-5031 | ICD-007 | Specified |
| IM-5032 | ICD-008 | Specified |
| IM-5040 | ICD-014 | Specified |
| IM-5041 | ICD-014 | Specified |
| IM-5042 | ICD-014 | Specified |

**Coverage:** 13/13 IM codes (100%)

---

## 19. P1 Remediation (from PLAN review)

**Issue:** PLAN Section 8 Source References showed "(to be located)" for PhaseOutputPayload.

**Resolution:** Verified as `agent.rs:33-40` during PRE-CODE phase source verification.

---

**Document Version:** 1.0
**Created:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint 2)
**Status:** PRE-CODE COMPLETE - Ready for MICROGATE Review

---

*This document provides Interface Control Documents for mechanical translation during IMPLEMENT phase.*
*CDP LODA Sprint 2: User Data Accessibility & Session Management*
