# TESTING PLAN: User Data Accessibility & Session Management
## CDP LODA Sprint 2 - Phase 6 Test Specifications

**Date:** 2025-11-28
**Status:** TESTING PLAN In Progress
**Sprint:** User Data Accessibility
**Source:** USER_DATA_ACCESSIBILITY_PRECODE.md (14 ICD Contracts)

---

## 1. Purpose

This document specifies test cases derived FROM the PRE-CODE ICD contracts. Every test traces to a specific IM code and ICD contract, ensuring 100% test-to-code alignment.

**Key Principle:** Tests designed from manifest prevent the API incompatibility issues seen in Sprint 3 (dbPath vs sqliteDbPath).

---

## 2. Test Coverage Requirements

### 2.1 Coverage Targets

| Layer | Target | Priority | Rationale |
|-------|--------|----------|-----------|
| Critical Path (IM-5020) | 100% | CRITICAL | Session resume is core requirement |
| Data Preservation (COALESCE) | 100% | CRITICAL | Data loss = mission failure |
| Error Handling | 100% | CRITICAL | User-facing error messages |
| Backend Commands | 80% | IMPORTANT | Tauri command coverage |
| Frontend Components | 60% | OPTIONAL | UI can be manually tested |

### 2.2 Test Prioritization

```
PRIORITY 1 (CRITICAL - Must pass before merge):
├── TC-5020-* (resume_research_session)
├── TC-5010-COALESCE-* (data preservation)
└── TC-5021-* (sliding window boundary)

PRIORITY 2 (IMPORTANT - Should pass):
├── TC-5001-* through TC-5011-* (backend)
├── TC-5030-* through TC-5032-* (database)
└── TC-EVT-* (event handling)

PRIORITY 3 (OPTIONAL - Nice to have):
├── TC-5040-* through TC-5042-* (frontend components)
└── TC-UI-* (visual regression)
```

---

## 3. Unit Test Specifications

### 3.1 ICD-001: PhaseOutputPayload Extension (IM-5001, IM-5002)

**Test Case: TC-5001-01 - Payload Serialization**
```
ID: TC-5001-01
IM Codes: IM-5001, IM-5002
ICD Contract: ICD-001
Priority: IMPORTANT

Preconditions:
- PhaseOutputPayload struct updated with system_prompt, user_input fields

Test Input:
PhaseOutputPayload {
    session_id: Some(123),
    phase_id: "phase_1".to_string(),
    phase_name: "Research".to_string(),
    status: "running".to_string(),
    system_prompt: Some("You are a research assistant...".to_string()),
    user_input: Some("Analyze the following topic...".to_string()),
    output: None,
    error: None,
}

Expected Output (JSON):
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

Verification:
- serde_json::to_string() succeeds
- All fields present in JSON output
- null for None values (not missing)
```

**Test Case: TC-5001-02 - Payload with None Prompts**
```
ID: TC-5001-02
IM Codes: IM-5001, IM-5002
ICD Contract: ICD-001
Priority: IMPORTANT

Test Input:
PhaseOutputPayload {
    session_id: Some(123),
    phase_id: "phase_1".to_string(),
    phase_name: "Research".to_string(),
    status: "completed".to_string(),
    system_prompt: None,
    user_input: None,
    output: Some("Analysis complete...".to_string()),
    error: None,
}

Expected Output:
- system_prompt serializes as null
- user_input serializes as null
- Backward compatibility maintained
```

---

### 3.2 ICD-002: emit_phase_output_with_prompts (IM-5003)

**Test Case: TC-5003-01 - Emit Running Event**
```
ID: TC-5003-01
IM Code: IM-5003
ICD Contract: ICD-002
Priority: CRITICAL

Preconditions:
- Agent with valid app_handle
- session_id = Some(123)

Test Input:
emit_phase_output_with_prompts(
    "phase_1",
    "Research",
    "running",
    Some("System prompt text"),
    Some("User input text"),
    None,
    None
)

Expected Behavior:
- app.emit("phase-output", payload) called
- Payload contains system_prompt = "System prompt text"
- Payload contains user_input = "User input text"
- Console log: "[AGENT-EMIT] Phase output: phase_1 -> running (session: Some(123))"

Verification:
- Mock app_handle captures emitted event
- Event name = "phase-output"
- Payload matches expected structure
```

**Test Case: TC-5003-02 - Emit Completed Event**
```
ID: TC-5003-02
IM Code: IM-5003
ICD Contract: ICD-002
Priority: CRITICAL

Test Input:
emit_phase_output_with_prompts(
    "phase_1",
    "Research",
    "completed",
    None,  // Prompts not sent on completion
    None,
    Some("Analysis output..."),
    None
)

Expected Behavior:
- Payload system_prompt = null
- Payload user_input = null
- Payload output = "Analysis output..."
- COALESCE in database preserves earlier prompts
```

**Test Case: TC-5003-03 - No App Handle**
```
ID: TC-5003-03
IM Code: IM-5003
ICD Contract: ICD-002
Priority: IMPORTANT

Preconditions:
- Agent with app_handle = None

Expected Behavior:
- Function completes without error
- No emit attempted
- No panic or crash
```

---

### 3.3 ICD-004: save_phase_output COALESCE Logic (IM-5010)

**Test Case: TC-5010-COALESCE-01 - Running Then Completed**
```
ID: TC-5010-COALESCE-01
IM Code: IM-5010
ICD Contract: ICD-004
Priority: CRITICAL

Preconditions:
- phase_outputs table with migration complete
- Session ID 123 exists in research_sessions

Test Sequence:
1. INSERT/UPSERT with:
   - session_id: 123, phase_id: "phase_1"
   - status: "running"
   - system_prompt: "System prompt A"
   - user_input: "User input B"
   - output: NULL

2. UPSERT with:
   - session_id: 123, phase_id: "phase_1"
   - status: "completed"
   - system_prompt: NULL
   - user_input: NULL
   - output: "Output C"

Expected State After Step 2:
- status = "completed"
- system_prompt = "System prompt A" (PRESERVED via COALESCE)
- user_input = "User input B" (PRESERVED via COALESCE)
- output = "Output C"

Verification Query:
SELECT status, system_prompt, user_input, output
FROM phase_outputs
WHERE session_id = 123 AND phase_id = 'phase_1';
```

**Test Case: TC-5010-COALESCE-02 - Output Never Overwritten with NULL**
```
ID: TC-5010-COALESCE-02
IM Code: IM-5010
ICD Contract: ICD-004
Priority: CRITICAL

Test Sequence:
1. UPSERT: output = "First output"
2. UPSERT: output = NULL

Expected:
- output = "First output" (PRESERVED)

Rationale: COALESCE(excluded.output, phase_outputs.output)
```

**Test Case: TC-5010-COALESCE-03 - Error Always Overwrites**
```
ID: TC-5010-COALESCE-03
IM Code: IM-5010
ICD Contract: ICD-004
Priority: IMPORTANT

Test Sequence:
1. UPSERT: error = "First error"
2. UPSERT: error = NULL

Expected:
- error = NULL (REPLACED, not preserved)

Rationale: error = excluded.error (no COALESCE)
```

---

### 3.4 ICD-005: Database Migration (IM-5001, IM-5002)

**Test Case: TC-5001-MIGRATE-01 - Idempotent Migration**
```
ID: TC-5001-MIGRATE-01
IM Codes: IM-5001, IM-5002
ICD Contract: ICD-005
Priority: CRITICAL

Test Sequence:
1. Run migration (ALTER TABLE ADD COLUMN system_prompt TEXT)
2. Run migration again (same statement)
3. Run migration third time

Expected:
- First run: Column added
- Second run: No error (column exists, ignored via let _ =)
- Third run: No error
- All existing data preserved

Verification:
PRAGMA table_info(phase_outputs);
-- Should show system_prompt and user_input columns
```

**Test Case: TC-5001-MIGRATE-02 - Existing Data Preserved**
```
ID: TC-5001-MIGRATE-02
IM Codes: IM-5001, IM-5002
ICD Contract: ICD-005
Priority: CRITICAL

Preconditions:
- phase_outputs table has 10 existing rows

Test Sequence:
1. Run migration

Expected:
- All 10 rows still exist
- system_prompt = NULL for all existing rows
- user_input = NULL for all existing rows
- No data loss
```

---

### 3.5 ICD-006: session_conversations Table (IM-5030)

**Test Case: TC-5030-01 - Table Creation**
```
ID: TC-5030-01
IM Code: IM-5030
ICD Contract: ICD-006
Priority: IMPORTANT

Test:
- Execute CREATE TABLE IF NOT EXISTS session_conversations...
- Execute CREATE INDEX IF NOT EXISTS...

Verification:
SELECT name FROM sqlite_master WHERE type='table' AND name='session_conversations';
SELECT name FROM sqlite_master WHERE type='index' AND name='idx_session_conversations_session_id';

Expected:
- Table exists
- Index exists
```

**Test Case: TC-5030-02 - Role CHECK Constraint**
```
ID: TC-5030-02
IM Code: IM-5030
ICD Contract: ICD-006
Priority: IMPORTANT

Test Input:
INSERT INTO session_conversations (session_id, role, content)
VALUES (1, 'invalid_role', 'content');

Expected:
- CHECK constraint violation
- Insert fails
- Error message indicates role constraint
```

**Test Case: TC-5030-03 - Foreign Key CASCADE**
```
ID: TC-5030-03
IM Code: IM-5030
ICD Contract: ICD-006
Priority: IMPORTANT

Test Sequence:
1. Create research_session with id=1
2. Insert 3 messages into session_conversations for session_id=1
3. DELETE FROM research_sessions WHERE id=1

Expected:
- All 3 messages in session_conversations deleted (CASCADE)
- No orphan records
```

---

### 3.6 ICD-007: add_session_message (IM-5031)

**Test Case: TC-5031-01 - Valid User Message**
```
ID: TC-5031-01
IM Code: IM-5031
ICD Contract: ICD-007
Priority: IMPORTANT

Test Input:
add_session_message(
    session_id: 123,
    phase_id: Some("phase_1"),
    role: "user",
    content: "What is machine learning?"
)

Expected:
- Returns Ok(message_id) where message_id > 0
- Row inserted with correct values
- created_at populated automatically
```

**Test Case: TC-5031-02 - Invalid Role Rejected**
```
ID: TC-5031-02
IM Code: IM-5031
ICD Contract: ICD-007
Priority: CRITICAL

Test Input:
add_session_message(
    session_id: 123,
    phase_id: None,
    role: "moderator",  // Invalid
    content: "content"
)

Expected:
- Returns Err("Invalid role: moderator. Must be 'user', 'assistant', or 'system'")
- No row inserted
```

**Test Case: TC-5031-03 - All Valid Roles**
```
ID: TC-5031-03
IM Code: IM-5031
ICD Contract: ICD-007
Priority: IMPORTANT

Test:
- role: "user" → Success
- role: "assistant" → Success
- role: "system" → Success
```

---

### 3.7 ICD-008: get_session_conversation (IM-5032)

**Test Case: TC-5032-01 - Retrieve All Messages**
```
ID: TC-5032-01
IM Code: IM-5032
ICD Contract: ICD-008
Priority: IMPORTANT

Preconditions:
- session_id=123 has 5 messages

Test Input:
get_session_conversation(session_id: 123, phase_id: None)

Expected:
- Returns Vec<SessionMessage> with 5 elements
- Ordered by created_at ASC
- All fields populated correctly
```

**Test Case: TC-5032-02 - Filter by Phase**
```
ID: TC-5032-02
IM Code: IM-5032
ICD Contract: ICD-008
Priority: IMPORTANT

Preconditions:
- session_id=123 has 5 messages
- 2 messages have phase_id="phase_1"
- 3 messages have phase_id="phase_2"

Test Input:
get_session_conversation(session_id: 123, phase_id: Some("phase_1"))

Expected:
- Returns Vec<SessionMessage> with 2 elements
- Only phase_1 messages included
```

**Test Case: TC-5032-03 - Empty Session**
```
ID: TC-5032-03
IM Code: IM-5032
ICD Contract: ICD-008
Priority: IMPORTANT

Test Input:
get_session_conversation(session_id: 999, phase_id: None)

Expected:
- Returns Ok(Vec::new()) - empty vector
- No error for non-existent session
```

---

### 3.8 ICD-009: get_phase_outputs Extended (IM-5011)

**Test Case: TC-5011-01 - Retrieve with Prompt Fields**
```
ID: TC-5011-01
IM Code: IM-5011
ICD Contract: ICD-009
Priority: IMPORTANT

Preconditions:
- session_id=123 has 3 phase outputs
- Phase 1: system_prompt="SP1", user_input="UI1", output="O1"

Test Input:
get_phase_outputs(session_id: 123)

Expected:
- Returns Vec<PhaseOutputRecord> with 3 elements
- Each record includes system_prompt and user_input fields
- Fields match stored values
```

---

### 3.9 ICD-010: reconstruct_session_context (IM-5021)

**Test Case: TC-5021-01 - Under Limit**
```
ID: TC-5021-01
IM Code: IM-5021
ICD Contract: ICD-010
Priority: CRITICAL

Test Input:
- phase_outputs: 10 completed phases with prompts
- max_pairs: 25

Expected:
- Returns 20 ChatMessages (10 pairs * 2)
- All pairs included (under limit)
- Order: oldest first
```

**Test Case: TC-5021-02 - At Limit**
```
ID: TC-5021-02
IM Code: IM-5021
ICD Contract: ICD-010
Priority: CRITICAL

Test Input:
- phase_outputs: 25 completed phases with prompts
- max_pairs: 25

Expected:
- Returns 50 ChatMessages (25 pairs * 2)
- All pairs included (exactly at limit)
```

**Test Case: TC-5021-03 - Over Limit (Sliding Window)**
```
ID: TC-5021-03
IM Code: IM-5021
ICD Contract: ICD-010
Priority: CRITICAL

Test Input:
- phase_outputs: 50 completed phases with prompts (named phase_1 through phase_50)
- max_pairs: 25

Expected:
- Returns 50 ChatMessages (last 25 pairs * 2)
- First message is from phase_26 (oldest in window)
- Last message is from phase_50 (newest)
- Phases 1-25 excluded (sliding window)
```

**Test Case: TC-5021-04 - Empty Input**
```
ID: TC-5021-04
IM Code: IM-5021
ICD Contract: ICD-010
Priority: IMPORTANT

Test Input:
- phase_outputs: empty vector
- max_pairs: 25

Expected:
- Returns empty Vec<ChatMessage>
- No panic or error
```

**Test Case: TC-5021-05 - Filter Non-Completed**
```
ID: TC-5021-05
IM Code: IM-5021
ICD Contract: ICD-010
Priority: CRITICAL

Test Input:
- phase_outputs: [completed, failed, running, completed]
- max_pairs: 25

Expected:
- Returns 4 ChatMessages (2 completed phases * 2)
- Failed and running phases excluded
```

---

### 3.10 ICD-011: resume_research_session (IM-5020)

**Test Case: TC-5020-01 - Successful Resume**
```
ID: TC-5020-01
IM Code: IM-5020
ICD Contract: ICD-011
Priority: CRITICAL

Preconditions:
- Session 123 exists with status="in_progress"
- 3 completed phases (phase_1, phase_2, phase_3)
- phase_4 is running

Test Input:
resume_research_session(session_id: 123)

Expected:
- Returns Ok(ResumeSessionResult)
- result.session.id = 123
- result.next_phase_id = "phase_4" (or next after last completed)
- result.context.history contains 6 messages (3 pairs)
- result.context.last_completed_phase = "phase_3"
- result.context.completed_phases = 3
```

**Test Case: TC-5020-ERR-01 - Session Not Found**
```
ID: TC-5020-ERR-01
IM Code: IM-5020
ICD Contract: ICD-011
Priority: CRITICAL

Test Input:
resume_research_session(session_id: 999)

Expected:
- Returns Err("Session 999 not found: ...")
```

**Test Case: TC-5020-ERR-02 - Session Already Completed**
```
ID: TC-5020-ERR-02
IM Code: IM-5020
ICD Contract: ICD-011
Priority: CRITICAL

Preconditions:
- Session 123 exists with status="completed"

Test Input:
resume_research_session(session_id: 123)

Expected:
- Returns Err("Session 123 already completed")
```

**Test Case: TC-5020-ERR-03 - Session Failed**
```
ID: TC-5020-ERR-03
IM Code: IM-5020
ICD Contract: ICD-011
Priority: CRITICAL

Preconditions:
- Session 123 exists with status="failed"

Test Input:
resume_research_session(session_id: 123)

Expected:
- Returns Err("Session 123 failed and cannot be resumed")
```

**Test Case: TC-5020-ERR-04 - Invalid Status**
```
ID: TC-5020-ERR-04
IM Code: IM-5020
ICD Contract: ICD-011
Priority: IMPORTANT

Preconditions:
- Session 123 exists with status="paused" (invalid)

Test Input:
resume_research_session(session_id: 123)

Expected:
- Returns Err("Session 123 has invalid status: paused")
```

**Test Case: TC-5020-ERR-05 - No Completed Phases**
```
ID: TC-5020-ERR-05
IM Code: IM-5020
ICD Contract: ICD-011
Priority: CRITICAL

Preconditions:
- Session 123 exists with status="in_progress"
- No completed phases (only running/failed)

Test Input:
resume_research_session(session_id: 123)

Expected:
- Returns Err("Session 123 has no completed phases to resume from")
```

**Test Case: TC-5020-ERR-06 - Lock Failure**
```
ID: TC-5020-ERR-06
IM Code: IM-5020
ICD Contract: ICD-011
Priority: IMPORTANT

Preconditions:
- auth_state.manager.lock() poisoned

Test Input:
resume_research_session(session_id: 123)

Expected:
- Returns Err("Failed to lock auth state")
```

---

### 3.11 ICD-013: Event Listener (Frontend)

**Test Case: TC-EVT-01 - Parameter Mapping**
```
ID: TC-EVT-01
IM Codes: IM-5001, IM-5002, IM-5003
ICD Contract: ICD-013
Priority: CRITICAL

Test:
Verify snake_case to camelCase mapping in invoke():

| Event Payload | Invoke Parameter |
|--------------|------------------|
| session_id | sessionId |
| phase_id | phaseId |
| phase_name | phaseName |
| system_prompt | systemPrompt |
| user_input | userInput |

Verification:
- Mock invoke() captures parameters
- All 7 parameters mapped correctly
```

**Test Case: TC-EVT-02 - Null Session ID Handling**
```
ID: TC-EVT-02
IM Code: IM-5003
ICD Contract: ICD-013
Priority: IMPORTANT

Test Input:
Event with session_id: null

Expected:
- invoke() NOT called
- No save_phase_output attempt
- No error thrown
```

---

## 4. Integration Test Specifications

### 4.1 Full Data Flow

**Test Case: TC-INT-01 - Emit to Save to Retrieve**
```
ID: TC-INT-01
IM Codes: IM-5003, IM-5010, IM-5011
Priority: CRITICAL

Test Sequence:
1. Agent emits phase-output with system_prompt and user_input
2. Frontend listener receives event
3. Frontend invokes save_phase_output
4. Backend persists to database
5. Frontend invokes get_phase_outputs
6. Verify prompts in returned data

Expected:
- End-to-end data integrity
- No field loss or corruption
- Correct ordering
```

### 4.2 Resume Flow

**Test Case: TC-INT-02 - Pause and Resume**
```
ID: TC-INT-02
IM Codes: IM-5020, IM-5021
Priority: CRITICAL

Test Sequence:
1. Start session with 3 phases
2. Complete phases 1 and 2
3. Pause (stop execution)
4. Call resume_research_session
5. Verify context reconstruction
6. Continue from phase 3

Expected:
- History contains phases 1 and 2
- next_phase_id = "phase_3"
- Agent can continue with context
```

### 4.3 Conversation Thread

**Test Case: TC-INT-03 - Add and Retrieve Messages**
```
ID: TC-INT-03
IM Codes: IM-5031, IM-5032
Priority: IMPORTANT

Test Sequence:
1. Add 5 messages to session
2. Retrieve all messages
3. Retrieve filtered by phase_id

Expected:
- All messages returned in order
- Filter correctly applied
- IDs sequential
```

---

## 5. Test Execution Order

```
PHASE 1: Unit Tests - Database Layer
├── TC-5001-MIGRATE-01, TC-5001-MIGRATE-02 (migration)
├── TC-5030-01, TC-5030-02, TC-5030-03 (session_conversations)
└── TC-5010-COALESCE-01, TC-5010-COALESCE-02, TC-5010-COALESCE-03 (COALESCE)

PHASE 2: Unit Tests - Backend Commands
├── TC-5001-01, TC-5001-02 (serialization)
├── TC-5003-01, TC-5003-02, TC-5003-03 (emit)
├── TC-5031-01, TC-5031-02, TC-5031-03 (add_session_message)
├── TC-5032-01, TC-5032-02, TC-5032-03 (get_session_conversation)
└── TC-5011-01 (get_phase_outputs)

PHASE 3: Unit Tests - Session Resume (CRITICAL PATH)
├── TC-5021-01 through TC-5021-05 (sliding window)
└── TC-5020-01, TC-5020-ERR-01 through TC-5020-ERR-06 (resume)

PHASE 4: Unit Tests - Frontend
└── TC-EVT-01, TC-EVT-02 (event listener)

PHASE 5: Integration Tests
├── TC-INT-01 (full data flow)
├── TC-INT-02 (pause and resume)
└── TC-INT-03 (conversation thread)
```

---

## 6. Test Infrastructure Requirements

### 6.1 Test Database Setup

```rust
// Test fixture for database tests
fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    // Run all migrations
    ensure_tables(&conn);
    // Create test research_session
    conn.execute(
        "INSERT INTO research_sessions (id, manifest_name, status) VALUES (123, 'Test', 'in_progress')",
        [],
    ).unwrap();
    conn
}
```

### 6.2 Mock App Handle

```rust
// Mock for Tauri app handle in emit tests
struct MockAppHandle {
    emitted_events: Vec<(String, serde_json::Value)>,
}

impl MockAppHandle {
    fn emit<T: Serialize>(&mut self, event: &str, payload: T) -> Result<(), ()> {
        self.emitted_events.push((
            event.to_string(),
            serde_json::to_value(payload).unwrap(),
        ));
        Ok(())
    }
}
```

### 6.3 Frontend Test Setup

```typescript
// Mock invoke for frontend tests
const mockInvoke = jest.fn();
jest.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

// Reset between tests
beforeEach(() => {
  mockInvoke.mockClear();
});
```

---

## 7. Traceability Matrix

| Test Case | IM Code | ICD Contract | Priority |
|-----------|---------|--------------|----------|
| TC-5001-01 | IM-5001, IM-5002 | ICD-001 | IMPORTANT |
| TC-5001-02 | IM-5001, IM-5002 | ICD-001 | IMPORTANT |
| TC-5003-01 | IM-5003 | ICD-002 | CRITICAL |
| TC-5003-02 | IM-5003 | ICD-002 | CRITICAL |
| TC-5003-03 | IM-5003 | ICD-002 | IMPORTANT |
| TC-5010-COALESCE-01 | IM-5010 | ICD-004 | CRITICAL |
| TC-5010-COALESCE-02 | IM-5010 | ICD-004 | CRITICAL |
| TC-5010-COALESCE-03 | IM-5010 | ICD-004 | IMPORTANT |
| TC-5001-MIGRATE-01 | IM-5001, IM-5002 | ICD-005 | CRITICAL |
| TC-5001-MIGRATE-02 | IM-5001, IM-5002 | ICD-005 | CRITICAL |
| TC-5030-01 | IM-5030 | ICD-006 | IMPORTANT |
| TC-5030-02 | IM-5030 | ICD-006 | IMPORTANT |
| TC-5030-03 | IM-5030 | ICD-006 | IMPORTANT |
| TC-5031-01 | IM-5031 | ICD-007 | IMPORTANT |
| TC-5031-02 | IM-5031 | ICD-007 | CRITICAL |
| TC-5031-03 | IM-5031 | ICD-007 | IMPORTANT |
| TC-5032-01 | IM-5032 | ICD-008 | IMPORTANT |
| TC-5032-02 | IM-5032 | ICD-008 | IMPORTANT |
| TC-5032-03 | IM-5032 | ICD-008 | IMPORTANT |
| TC-5011-01 | IM-5011 | ICD-009 | IMPORTANT |
| TC-5021-01 | IM-5021 | ICD-010 | CRITICAL |
| TC-5021-02 | IM-5021 | ICD-010 | CRITICAL |
| TC-5021-03 | IM-5021 | ICD-010 | CRITICAL |
| TC-5021-04 | IM-5021 | ICD-010 | IMPORTANT |
| TC-5021-05 | IM-5021 | ICD-010 | CRITICAL |
| TC-5020-01 | IM-5020 | ICD-011 | CRITICAL |
| TC-5020-ERR-01 | IM-5020 | ICD-011 | CRITICAL |
| TC-5020-ERR-02 | IM-5020 | ICD-011 | CRITICAL |
| TC-5020-ERR-03 | IM-5020 | ICD-011 | CRITICAL |
| TC-5020-ERR-04 | IM-5020 | ICD-011 | IMPORTANT |
| TC-5020-ERR-05 | IM-5020 | ICD-011 | CRITICAL |
| TC-5020-ERR-06 | IM-5020 | ICD-011 | IMPORTANT |
| TC-EVT-01 | IM-5001, IM-5002, IM-5003 | ICD-013 | CRITICAL |
| TC-EVT-02 | IM-5003 | ICD-013 | IMPORTANT |
| TC-INT-01 | IM-5003, IM-5010, IM-5011 | Multiple | CRITICAL |
| TC-INT-02 | IM-5020, IM-5021 | Multiple | CRITICAL |
| TC-INT-03 | IM-5031, IM-5032 | Multiple | IMPORTANT |

**Total Test Cases:** 37
**CRITICAL Priority:** 19 (100% required)
**IMPORTANT Priority:** 15 (80% required)
**OPTIONAL Priority:** 3 (best effort)

---

## 8. Coverage Summary

| IM Code | Test Cases | Coverage |
|---------|------------|----------|
| IM-5001 | 7 | 100% |
| IM-5002 | 7 | 100% |
| IM-5003 | 5 | 100% |
| IM-5010 | 3 | 100% |
| IM-5011 | 1 | 100% |
| IM-5020 | 7 | 100% |
| IM-5021 | 5 | 100% |
| IM-5030 | 3 | 100% |
| IM-5031 | 3 | 100% |
| IM-5032 | 3 | 100% |
| IM-5040 | - | Deferred (UI) |
| IM-5041 | - | Deferred (UI) |
| IM-5042 | - | Deferred (UI) |

**Backend Coverage:** 100% (10/10 IM codes)
**Frontend Components:** Deferred to manual testing

---

**Document Version:** 1.0
**Created:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint 2)
**Status:** TESTING PLAN COMPLETE - Ready for MICROGATE Review

---

*This document provides test specifications derived from PRE-CODE ICD contracts.*
*CDP LODA Sprint 2: User Data Accessibility & Session Management*
