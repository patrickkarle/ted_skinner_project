# ULTRATHINK: User Data Accessibility & Session Management
## CDP LODA Enhancement - Phase 1 Analysis

**Date:** 2025-11-28
**Status:** ULTRATHINK In Progress
**Sprint:** User Data Accessibility

---

## 1. Problem Statement

The user has identified interconnected issues around data accessibility:

1. **Manifest Prompt Inaccessibility** - Cannot see what was actually sent to the LLM
2. **Session Restart Blocked** - Paused sessions cannot be resumed because prompt context is lost
3. **Conversation Fragmentation** - Manifest submissions and follow-up chat are disconnected
4. **UI Deficiencies** - Data is there but not exposed cleanly to the user

**Root Cause:** The system was built output-centric, not conversation-centric.

---

## 2. Current Architecture Analysis

### 2.1 What Gets Persisted (Current)

```
research_sessions table
├── id, company, model, manifest_name
├── status ("in_progress", "completed", "failed")
├── current_phase_id (for pause indication)
└── created_at, updated_at

phase_outputs table
├── session_id (FK → research_sessions)
├── phase_id, phase_name
├── status, output, error ← OUTPUT ONLY
└── created_at, updated_at

briefs table (separate concept)
├── company, model, content (final output)
└── conversation_messages → follow-up chat
```

**Gap:** No `prompt_text` or `input_data` stored in phase_outputs.

### 2.2 Agent Execution Flow (agent.rs)

```rust
// Line 208-212 - Prompt created but NOT persisted
let req = LLMRequest {
    system: system_prompt.clone(),    // ← LOST
    user: input_data.clone(),         // ← LOST
    model: model.to_string(),
};

// Line 132 - Only OUTPUT is emitted for persistence
self.emit_phase_output(&phase.id, &phase.name, "completed", Some(&output), None);
```

### 2.3 Frontend Session View (App.tsx)

```typescript
// Line 1278-1305 - Shows phase outputs but NOT prompts
{sessionPhaseOutputs.map((output) => (
  <div className="phase-output-card">
    <pre>{output.output}</pre>  // ← Only output displayed
  </div>
))}
```

---

## 3. Proposed Solution Architecture

### 3.1 Database Schema Changes

```sql
-- Add prompt storage to phase_outputs
ALTER TABLE phase_outputs ADD COLUMN system_prompt TEXT;
ALTER TABLE phase_outputs ADD COLUMN user_input TEXT;

-- Add session-level conversation (not tied to briefs)
CREATE TABLE session_conversations (
    id INTEGER PRIMARY KEY,
    session_id INTEGER NOT NULL REFERENCES research_sessions(id),
    phase_id TEXT,  -- NULL for general follow-up, set for phase-specific
    role TEXT NOT NULL,  -- 'user', 'assistant', 'system'
    content TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### 3.2 Backend Changes (agent.rs)

```rust
// NEW: PhaseOutputPayload includes prompts
struct PhaseOutputPayload {
    session_id: Option<i64>,
    phase_id: String,
    phase_name: String,
    status: String,
    system_prompt: Option<String>,  // NEW
    user_input: Option<String>,     // NEW
    output: Option<String>,
    error: Option<String>,
}

// In execute_phase(), emit prompts at "running" status:
self.emit_phase_output(
    &phase.id,
    &phase.name,
    "running",
    Some(&system_prompt),   // NEW
    Some(&input_data),      // NEW
    None,
    None
);
```

### 3.3 Frontend Changes (App.tsx)

```typescript
// NEW: PhaseOutputRecord includes prompts
type PhaseOutputRecord = {
  // existing fields...
  system_prompt: string | null;  // NEW
  user_input: string | null;     // NEW
};

// NEW: Expandable prompt view in session detail
<div className="phase-output-card">
  <button onClick={() => togglePromptView(output.id)}>
    📝 View Prompt
  </button>
  {showPrompts[output.id] && (
    <div className="prompt-view">
      <h5>System Prompt</h5>
      <pre>{output.system_prompt}</pre>
      <h5>User Input</h5>
      <pre>{output.user_input}</pre>
    </div>
  )}
  <h5>Output</h5>
  <pre>{output.output}</pre>
</div>
```

### 3.4 Session Restart Mechanism

```rust
// NEW: Rust command to resume session
#[tauri::command]
async fn resume_research_session(
    session_id: i64,
    state: State<'_, AppState>
) -> Result<String, String> {
    // 1. Load session and phase_outputs
    // 2. Find last completed phase
    // 3. Rebuild context from outputs
    // 4. Continue from next phase
}
```

```typescript
// NEW: Frontend restart button
{session.status === "in_progress" && (
  <button onClick={() => resumeSession(session.id)}>
    ▶ Resume
  </button>
)}
```

---

## 4. Feature Breakdown

### 4.1 Manifest Prompt Accessibility
| Item | Priority | Complexity |
|------|----------|------------|
| Add prompt columns to DB | HIGH | Low |
| Modify PhaseOutputPayload in Rust | HIGH | Medium |
| Update emit_phase_output() calls | HIGH | Low |
| Add frontend prompt display | HIGH | Medium |
| Add "Copy Prompt" button | MEDIUM | Low |

### 4.2 Session Continuity
| Item | Priority | Complexity |
|------|----------|------------|
| Resume session Rust command | HIGH | High |
| Context reconstruction logic | HIGH | High |
| Frontend resume button | HIGH | Low |
| Session state indicator improvements | MEDIUM | Medium |

### 4.3 Conversation Integration
| Item | Priority | Complexity |
|------|----------|------------|
| session_conversations table | HIGH | Low |
| Link chat to sessions (not briefs) | HIGH | Medium |
| Multi-turn using ChatMessage types | MEDIUM | Medium |
| Conversation history in resume | MEDIUM | Medium |

### 4.4 UI Professionalization
| Item | Priority | Complexity |
|------|----------|------------|
| Session panel redesign | HIGH | High |
| Prompt/output collapsible cards | MEDIUM | Medium |
| Status indicators (running/paused/done) | MEDIUM | Low |
| Session timeline view | LOW | High |

---

## 5. IM Code Allocation

| IM Code | Component | Description |
|---------|-----------|-------------|
| IM-5001 | PhaseOutputPayload.system_prompt | Prompt field in event payload |
| IM-5002 | PhaseOutputPayload.user_input | Input field in event payload |
| IM-5003 | emit_phase_output_with_prompts | Updated emit method |
| IM-5010 | save_phase_output_with_prompts | Updated persistence command |
| IM-5011 | get_phase_outputs_with_prompts | Updated retrieval command |
| IM-5020 | resume_research_session | Session restart command |
| IM-5021 | reconstruct_session_context | Context rebuilding logic |
| IM-5030 | session_conversations table | New DB table |
| IM-5031 | add_session_message | New conversation command |
| IM-5032 | get_session_conversation | Retrieve session chat |
| IM-5040 | SessionDetailPanel | New UI component |
| IM-5041 | PromptViewCard | Collapsible prompt display |
| IM-5042 | ResumeSessionButton | Resume action component |

---

## 6. Dependencies

```
IM-5001, IM-5002 (payload fields)
    ↓
IM-5003 (emit method)
    ↓
IM-5010 (DB persistence)
    ↓
IM-5011 (DB retrieval)
    ↓
IM-5040, IM-5041 (UI components)

IM-5010, IM-5011 (phase data)
    ↓
IM-5021 (context reconstruction)
    ↓
IM-5020 (resume command)
    ↓
IM-5042 (UI button)

IM-5030 (conversations table)
    ↓
IM-5031, IM-5032 (CRUD)
    ↓
Multi-turn integration (uses IM-4020)
```

---

## 7. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Schema migration on existing data | Medium | Medium | Migration script preserves existing, adds nullable columns |
| Large prompts causing UI lag | Low | Medium | Virtualized scrolling, lazy loading |
| Session resume data corruption | Medium | High | Validation before resume, rollback on error |
| Multi-turn token limits | Medium | Medium | Use sliding window from IM-4003 |

---

## 8. Next Steps

1. **PHASE 2: RESEARCH** - Review Tauri 2.0 migration patterns for DB changes
2. **PHASE 3: NOTES** - Document schema migration approach
3. **PHASE 4: PLAN** - Detailed technical plan with IM code specs
4. **PHASE 5: PRE-CODE** - ICD for new data contracts

---

**Document Version:** 1.0
**Created:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint)
**Status:** ULTRATHINK COMPLETE - Ready for RESEARCH
