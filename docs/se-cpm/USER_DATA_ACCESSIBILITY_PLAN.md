# PLAN: User Data Accessibility & Session Management
## CDP LODA Sprint 2 - Phase 4 Technical Specifications

**Date:** 2025-11-28
**Status:** PLAN In Progress
**Sprint:** User Data Accessibility
**Source:** USER_DATA_ACCESSIBILITY_NOTES.md

---

## 1. Purpose

This document provides detailed technical specifications for all IM codes defined in Sprint 2.
These specifications enable mechanical translation during the IMPLEMENT phase.

**Inputs:** NOTES Section 9 (PLAN Phase Inputs)
**Outputs:** Complete specifications for Backend, Database, and Frontend components

---

## 2. Backend Specifications

### 2.1 IM-5001 & IM-5002: PhaseOutputPayload Fields

**Location:** agent.rs (PhaseOutputPayload struct)

**Current Definition (agent.rs):**
```rust
#[derive(Clone, Serialize)]
struct PhaseOutputPayload {
    session_id: Option<i64>,
    phase_id: String,
    phase_name: String,
    status: String,
    output: Option<String>,
    error: Option<String>,
}
```

**Modified Definition:**
```rust
#[derive(Clone, Serialize)]
struct PhaseOutputPayload {
    session_id: Option<i64>,
    phase_id: String,
    phase_name: String,
    status: String,
    system_prompt: Option<String>,  // IM-5001: ADD
    user_input: Option<String>,     // IM-5002: ADD
    output: Option<String>,
    error: Option<String>,
}
```

**Field Specifications:**

| Field | Type | Description | When Populated |
|-------|------|-------------|----------------|
| system_prompt | Option<String> | System prompt sent to LLM | status="running" |
| user_input | Option<String> | User input/manifest data sent to LLM | status="running" |

---

### 2.2 IM-5003: emit_phase_output_with_prompts()

**Location:** agent.rs (Agent impl)

**Function Signature:**
```rust
fn emit_phase_output_with_prompts(
    &self,
    phase_id: &str,
    phase_name: &str,
    status: &str,
    system_prompt: Option<&str>,
    user_input: Option<&str>,
    output: Option<&str>,
    error: Option<&str>,
) -> Result<(), String>
```

**Implementation Notes:**
- Replaces current `emit_phase_output()` method
- Emits "phase-output" event via Tauri app handle
- Includes system_prompt and user_input only when status="running"

**Call Sites to Update:**
1. `execute_phase()` at LLMRequest creation (status="running")
2. `execute_phase()` at completion (status="completed", prompts=None)
3. `execute_phase()` at error (status="failed", prompts=None)

---

### 2.3 IM-5010: save_phase_output_with_prompts

**Location:** auth.rs (Tauri command)

**Command Signature:**
```rust
#[tauri::command]
pub async fn save_phase_output(
    session_id: i64,
    phase_id: String,
    phase_name: String,
    status: String,
    system_prompt: Option<String>,  // ADD
    user_input: Option<String>,     // ADD
    output: Option<String>,
    error: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String>
```

**SQL Update:**
```sql
INSERT INTO phase_outputs (
    session_id, phase_id, phase_name, status,
    system_prompt, user_input,  -- ADD
    output, error, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
ON CONFLICT (session_id, phase_id) DO UPDATE SET
    status = excluded.status,
    system_prompt = COALESCE(excluded.system_prompt, phase_outputs.system_prompt),
    user_input = COALESCE(excluded.user_input, phase_outputs.user_input),
    output = COALESCE(excluded.output, phase_outputs.output),
    error = excluded.error,
    updated_at = datetime('now')
```

**COALESCE Logic:** Preserves prompts from "running" status when "completed" update arrives with None.

---

### 2.4 IM-5011: get_phase_outputs_with_prompts

**Location:** auth.rs (Tauri command)

**Command Signature:**
```rust
#[tauri::command]
pub async fn get_phase_outputs(
    session_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<PhaseOutputRecord>, String>
```

**PhaseOutputRecord:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct PhaseOutputRecord {
    pub id: i64,
    pub session_id: i64,
    pub phase_id: String,
    pub phase_name: String,
    pub status: String,
    pub system_prompt: Option<String>,  // ADD
    pub user_input: Option<String>,     // ADD
    pub output: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
```

**SQL Query:**
```sql
SELECT id, session_id, phase_id, phase_name, status,
       system_prompt, user_input,  -- ADD
       output, error, created_at, updated_at
FROM phase_outputs
WHERE session_id = ?
ORDER BY created_at ASC
```

---

### 2.5 IM-5020: resume_research_session

**Location:** auth.rs (new Tauri command)

**Command Signature:**
```rust
#[tauri::command]
pub async fn resume_research_session(
    session_id: i64,
    state: State<'_, AppState>,
) -> Result<ResumeSessionResult, String>
```

**ResumeSessionResult:**
```rust
#[derive(Debug, Serialize)]
pub struct ResumeSessionResult {
    pub session: ResearchSession,
    pub next_phase_id: String,
    pub context: SessionContext,
}

#[derive(Debug, Serialize)]
pub struct SessionContext {
    pub history: Vec<ChatMessage>,  // Reuses IM-4001
    pub last_completed_phase: String,
    pub total_phases: usize,
    pub completed_phases: usize,
}
```

**Algorithm:**
```
1. Load session from research_sessions WHERE id = session_id
2. Verify status = 'in_progress' (cannot resume completed/failed)
3. Load phase_outputs for session, ordered by created_at
4. Find last completed phase (status = 'completed')
5. Determine next_phase_id from manifest phase sequence
6. Call reconstruct_session_context(phase_outputs)
7. Return ResumeSessionResult
```

**Error Cases:**
| Condition | Error Message |
|-----------|---------------|
| Session not found | "Session {id} not found" |
| Session completed | "Session {id} already completed" |
| Session failed | "Session {id} failed and cannot be resumed" |
| No completed phases | "Session {id} has no completed phases to resume from" |

---

### 2.6 IM-5021: reconstruct_session_context

**Location:** agent.rs (Agent impl)

**Function Signature:**
```rust
fn reconstruct_session_context(
    &self,
    phase_outputs: Vec<PhaseOutputRecord>,
    max_pairs: usize,  // Default: 25 (from NOTES Section 3.2)
) -> Vec<ChatMessage>
```

**Algorithm:**
```
1. Filter phase_outputs where status = 'completed' AND system_prompt IS NOT NULL
2. For each phase_output, create message pairs:
   - ChatMessage { role: "user", content: user_input }
   - ChatMessage { role: "assistant", content: output }
3. Apply sliding window: Take last max_pairs pairs
4. Return Vec<ChatMessage> for MultiTurnRequest
```

**Sliding Window Implementation:**
```rust
let pairs: Vec<(ChatMessage, ChatMessage)> = phase_outputs
    .iter()
    .filter(|p| p.status == "completed" && p.system_prompt.is_some())
    .map(|p| (
        ChatMessage { role: ChatRole::User, content: p.user_input.clone().unwrap_or_default() },
        ChatMessage { role: ChatRole::Assistant, content: p.output.clone().unwrap_or_default() },
    ))
    .collect();

// Apply sliding window (last N pairs)
let window_start = pairs.len().saturating_sub(max_pairs);
pairs[window_start..].to_vec().into_iter().flat_map(|(u, a)| vec![u, a]).collect()
```

---

## 3. Database Specifications

### 3.1 IM-5001/IM-5002: phase_outputs Migration

**Migration Script (idempotent):**
```rust
// In AuthManager::ensure_tables()
// Pattern: auth.rs:422-425

// Add prompt columns if they don't exist
let _ = self.conn.execute(
    "ALTER TABLE phase_outputs ADD COLUMN system_prompt TEXT",
    [],
);
let _ = self.conn.execute(
    "ALTER TABLE phase_outputs ADD COLUMN user_input TEXT",
    [],
);
```

**Verification Query:**
```sql
PRAGMA table_info(phase_outputs);
-- Should show system_prompt and user_input columns
```

---

### 3.2 IM-5030: session_conversations Table

**CREATE TABLE Statement:**
```sql
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

**Column Specifications:**

| Column | Type | Constraint | Description |
|--------|------|------------|-------------|
| id | INTEGER | PRIMARY KEY | Auto-increment ID |
| session_id | INTEGER | NOT NULL, FK | Links to research_sessions |
| phase_id | TEXT | NULL | Optional phase association |
| role | TEXT | NOT NULL, CHECK | 'user', 'assistant', or 'system' |
| content | TEXT | NOT NULL | Message content |
| created_at | TEXT | NOT NULL | ISO8601 timestamp |

---

### 3.3 IM-5031: add_session_message

**Command Signature:**
```rust
#[tauri::command]
pub async fn add_session_message(
    session_id: i64,
    phase_id: Option<String>,
    role: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<i64, String>
```

**Validation:**
```rust
if !["user", "assistant", "system"].contains(&role.as_str()) {
    return Err(format!("Invalid role: {}. Must be user, assistant, or system", role));
}
```

**SQL:**
```sql
INSERT INTO session_conversations (session_id, phase_id, role, content, created_at)
VALUES (?, ?, ?, ?, datetime('now'))
RETURNING id
```

**Return:** ID of inserted message

---

### 3.4 IM-5032: get_session_conversation

**Command Signature:**
```rust
#[tauri::command]
pub async fn get_session_conversation(
    session_id: i64,
    phase_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SessionMessage>, String>
```

**SessionMessage:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionMessage {
    pub id: i64,
    pub session_id: i64,
    pub phase_id: Option<String>,
    pub role: String,
    pub content: String,
    pub created_at: String,
}
```

**SQL (with optional phase filter):**
```sql
SELECT id, session_id, phase_id, role, content, created_at
FROM session_conversations
WHERE session_id = ?
  AND (? IS NULL OR phase_id = ?)
ORDER BY created_at ASC
```

---

## 4. Frontend Specifications

### 4.1 TypeScript Type Updates

**PhaseOutputPayload (App.tsx):**
```typescript
interface PhaseOutputPayload {
  session_id: number | null;
  phase_id: string;
  phase_name: string;
  status: string;
  system_prompt: string | null;  // ADD
  user_input: string | null;     // ADD
  output: string | null;
  error: string | null;
}
```

**PhaseOutputRecord (App.tsx):**
```typescript
interface PhaseOutputRecord {
  id: number;
  session_id: number;
  phase_id: string;
  phase_name: string;
  status: string;
  system_prompt: string | null;  // ADD
  user_input: string | null;     // ADD
  output: string | null;
  error: string | null;
  created_at: string;
  updated_at: string;
}
```

**SessionMessage (new):**
```typescript
interface SessionMessage {
  id: number;
  session_id: number;
  phase_id: string | null;
  role: 'user' | 'assistant' | 'system';
  content: string;
  created_at: string;
}
```

**ResumeSessionResult (new):**
```typescript
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

### 4.2 IM-5040: SessionDetailPanel

**Component Signature:**
```typescript
interface SessionDetailPanelProps {
  session: ResearchSession;
  phaseOutputs: PhaseOutputRecord[];
  onResume: (sessionId: number) => void;
  onClose: () => void;
}

const SessionDetailPanel: React.FC<SessionDetailPanelProps> = ({
  session,
  phaseOutputs,
  onResume,
  onClose,
}) => { ... }
```

**State:**
```typescript
const [expandedPrompts, setExpandedPrompts] = useState<Set<number>>(new Set());
const [isResuming, setIsResuming] = useState(false);
```

**Render Structure:**
```
<div className="session-detail-panel">
  <header>
    <h2>{session.manifest_name}</h2>
    <span className="status-badge">{session.status}</span>
    <button onClick={onClose}>Close</button>
  </header>

  <section className="phase-outputs">
    {phaseOutputs.map(output => (
      <PromptViewCard
        key={output.id}
        output={output}
        expanded={expandedPrompts.has(output.id)}
        onToggle={() => togglePrompt(output.id)}
      />
    ))}
  </section>

  <footer>
    {session.status === 'in_progress' && (
      <ResumeSessionButton
        sessionId={session.id}
        onResume={onResume}
        isLoading={isResuming}
      />
    )}
  </footer>
</div>
```

---

### 4.3 IM-5041: PromptViewCard

**Component Signature:**
```typescript
interface PromptViewCardProps {
  output: PhaseOutputRecord;
  expanded: boolean;
  onToggle: () => void;
}

const PromptViewCard: React.FC<PromptViewCardProps> = ({
  output,
  expanded,
  onToggle,
}) => { ... }
```

**Render Structure:**
```
<div className="prompt-view-card">
  <header onClick={onToggle}>
    <span className="phase-name">{output.phase_name}</span>
    <span className="status">{output.status}</span>
    <span className="chevron">{expanded ? '▼' : '▶'}</span>
  </header>

  {expanded && (
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

  <div className="output-section">
    <h4>Output</h4>
    <pre>{output.output}</pre>
  </div>
</div>
```

**Lazy Loading:** System prompt and user input content only rendered when expanded=true.

---

### 4.4 IM-5042: ResumeSessionButton

**Component Signature:**
```typescript
interface ResumeSessionButtonProps {
  sessionId: number;
  onResume: (sessionId: number) => void;
  isLoading: boolean;
}

const ResumeSessionButton: React.FC<ResumeSessionButtonProps> = ({
  sessionId,
  onResume,
  isLoading,
}) => { ... }
```

**Click Handler:**
```typescript
const handleClick = async () => {
  try {
    const result = await invoke<ResumeSessionResult>('resume_research_session', {
      sessionId,
    });
    onResume(sessionId);
    // Agent continues from result.next_phase_id with result.context.history
  } catch (error) {
    console.error('Failed to resume session:', error);
    // Show error toast/notification
  }
};
```

**Render:**
```
<button
  className="resume-button"
  onClick={handleClick}
  disabled={isLoading}
>
  {isLoading ? 'Resuming...' : '▶ Resume Session'}
</button>
```

---

### 4.5 Event Listener Update (App.tsx:295)

**Current:**
```typescript
const unlistenPhaseOutput = listen<PhaseOutputPayload>("phase-output", async (event) => {
  const { session_id, phase_id, phase_name, status, output, error } = event.payload;
  if (session_id !== null) {
    await invoke("save_phase_output", { sessionId, phaseId, phaseName, status, output, error });
  }
});
```

**Updated:**
```typescript
const unlistenPhaseOutput = listen<PhaseOutputPayload>("phase-output", async (event) => {
  const {
    session_id, phase_id, phase_name, status,
    system_prompt, user_input,  // ADD
    output, error
  } = event.payload;

  if (session_id !== null) {
    await invoke("save_phase_output", {
      sessionId: session_id,
      phaseId: phase_id,
      phaseName: phase_name,
      status,
      systemPrompt: system_prompt,  // ADD
      userInput: user_input,        // ADD
      output,
      error
    });
  }
});
```

---

## 5. Implementation Order

Based on NOTES Section 7.2 Layer Dependencies:

```
PHASE 1: Database Schema (Day 1)
├── Run phase_outputs migration (IM-5001, IM-5002)
├── Create session_conversations table (IM-5030)
└── Verify with PRAGMA table_info

PHASE 2: Backend - Data Layer (Day 1-2)
├── Update PhaseOutputPayload struct
├── Update emit_phase_output_with_prompts (IM-5003)
├── Update save_phase_output command (IM-5010)
└── Update get_phase_outputs command (IM-5011)

PHASE 3: Backend - Session Commands (Day 2)
├── Implement add_session_message (IM-5031)
├── Implement get_session_conversation (IM-5032)
├── Implement reconstruct_session_context (IM-5021)
└── Implement resume_research_session (IM-5020)

PHASE 4: Frontend - Types & Listener (Day 3)
├── Update TypeScript types
├── Update event listener (App.tsx:295)
└── Verify data flow with console logging

PHASE 5: Frontend - Components (Day 3-4)
├── Create PromptViewCard (IM-5041)
├── Create ResumeSessionButton (IM-5042)
├── Create SessionDetailPanel (IM-5040)
└── Integrate into existing UI
```

---

## 6. Risk Mitigations Applied

| Risk | Mitigation in Specifications |
|------|------------------------------|
| R-01: Migration breaks data | COALESCE in UPSERT preserves existing prompts |
| R-02: Large prompts UI lag | Lazy loading in PromptViewCard (expanded flag) |
| R-03: Resume corruption | Validation in resume_research_session with specific errors |
| R-04: Token overflow | max_pairs=25 default in reconstruct_session_context |

---

## 7. Test Specifications (for TESTING PLAN phase)

### Unit Tests Required
- [ ] PhaseOutputPayload serialization with new fields
- [ ] emit_phase_output_with_prompts emits correct event
- [ ] save_phase_output COALESCE logic preserves prompts
- [ ] get_phase_outputs returns new fields
- [ ] add_session_message validates role
- [ ] get_session_conversation filters by phase_id
- [ ] reconstruct_session_context sliding window at boundary (0, 25, 50 pairs)
- [ ] resume_research_session error cases

### Integration Tests Required
- [ ] Full flow: emit → save → get (with prompts)
- [ ] Resume flow: pause → resume → continue
- [ ] Conversation: add messages → retrieve → resume with history

---

## 8. Source References

| Specification | Source | Lines |
|---------------|--------|-------|
| PhaseOutputPayload current | agent.rs | (to be located) |
| emit_phase_output current | agent.rs | 132 |
| save_phase_output current | auth.rs | 1185 |
| get_phase_outputs current | auth.rs | 1242 |
| ALTER TABLE pattern | auth.rs | 422-425 |
| ChatMessage type | IM-4001 (Sprint 1) | llm.rs |
| MultiTurnRequest | IM-4003 (Sprint 1) | llm.rs |

---

**Document Version:** 1.0
**Created:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint 2)
**Status:** PLAN COMPLETE - Ready for PRE-CODE

---

*This document provides mechanical translation guidance for IMPLEMENT phase.*
*CDP LODA Sprint 2: User Data Accessibility & Session Management*
