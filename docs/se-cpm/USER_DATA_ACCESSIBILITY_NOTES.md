# NOTES: User Data Accessibility & Session Management
## CDP LODA Sprint 2 - Phase 3 Structured Notes

**Date:** 2025-11-28
**Status:** NOTES In Progress
**Sprint:** User Data Accessibility
**Source:** USER_DATA_ACCESSIBILITY_RESEARCH.md

---

## 1. Purpose

These notes distill RESEARCH findings into actionable items for the PLAN phase.
All decisions have been made; these notes document the "why" for future reference.

---

## 2. Decision Registry

### 2.1 Q-01: Prompt Storage Format

| Aspect | Decision |
|--------|----------|
| **Question** | Should prompts be stored compressed? |
| **Answer** | **NO** - Store as plain TEXT |
| **Rationale** | See Section 3.1 below |
| **IM Codes Affected** | IM-5001, IM-5002, IM-5010 |
| **Revisit Trigger** | If storage exceeds 100MB (~33,000 sessions) |

### 2.2 Q-02: Conversation History Limit

| Aspect | Decision |
|--------|----------|
| **Question** | Max conversation history for session resume? |
| **Answer** | **25 message pairs** (sliding window, provider-aware) |
| **Rationale** | See Section 3.2 below |
| **IM Codes Affected** | IM-5020, IM-5021, IM-4003, IM-4020 |
| **Revisit Trigger** | If token estimation assumption proves inaccurate in Phase 9 |

### 2.3 Q-03: Table Strategy

| Aspect | Decision |
|--------|----------|
| **Question** | Should session_conversations replace briefs.conversations? |
| **Answer** | **NO** - Keep both tables (complementary purposes) |
| **Rationale** | See Section 3.3 below |
| **IM Codes Affected** | IM-5030, IM-5031, IM-5032 |
| **Revisit Trigger** | If user feedback indicates confusion between table purposes |

---

## 3. Decision Rationale (Permanent Record)

### 3.1 Plain TEXT Storage Decision

**4-Factor Analysis:**

| Factor | Finding | Impact |
|--------|---------|--------|
| **Typical Prompt Size** | System: 500-2000 chars, User: 100-5000 chars | Minimal storage |
| **SQLite TEXT Limit** | Up to 1 billion bytes | No constraint |
| **Storage Growth** | 10 sessions x 5 phases x 3KB = 150KB | Negligible |
| **Compression Overhead** | flate2 crate: 5KB binary, CPU per read/write | Unnecessary cost |

**Trade-off Rejected:** Compression adds complexity (serialize/compress/store/decompress/deserialize) without meaningful benefit. Searchability lost, latency increased (~1-5ms per operation).

**Revisit Trigger:** If storage exceeds 100MB (approximately 33,000 sessions).

### 3.2 Sliding Window Decision

**Provider Constraint Analysis:**

| Provider | Context Window | Safe Limit | Used |
|----------|---------------|-----------|------|
| Claude 3.5 Sonnet | 200K tokens | 80 pairs | - |
| GPT-4o | 128K tokens | 50 pairs | - |
| DeepSeek | 64K tokens | 25 pairs | **Default** |
| Gemini 1.5 | 128K-1M tokens | 50-400 pairs | - |

**Conservative Default Chosen:** 25 message pairs.
- Works for ALL providers including the most constrained (DeepSeek 64K)

> **ASSUMPTION (To be validated in Phase 9: EXECUTE TESTS):**
> Token estimation of ~1K tokens per message pair (~50K tokens for 25 pairs).
> Validation method: Measure actual token counts during test session resumption.

**Future Enhancement:** Provider-specific maximums could be implemented after default proves stable.

### 3.3 Dual-Table Decision

**Lifecycle Analysis:**

| Table | Purpose | When Used | FK Target |
|-------|---------|-----------|-----------|
| `conversations` | Brief follow-up Q&A | AFTER research complete | briefs.id |
| `session_conversations` | Session prompts/responses | DURING research | research_sessions.id |

**User Journey:**
1. User starts research session -> `session_conversations` tracks manifest prompts (NEW)
2. Research completes -> Brief generated and stored in `briefs`
3. User asks follow-up questions -> `conversations` stores Q&A (EXISTING)
4. User resumes paused session -> `session_conversations` provides history (NEW)

**Why Not Replace:** Different purposes, different lifecycles, different FK relationships. Future sessions may not always produce briefs (e.g., abandoned sessions, experiments).

---

## 4. Migration Pattern Notes

### 4.1 Selected Pattern: ALTER TABLE ADD COLUMN

**Pattern Reference:** auth.rs:422-425 (existing profile migration)

```
Existing Pattern:
let _ = self.conn.execute("ALTER TABLE users ADD COLUMN first_name TEXT", []);
let _ = self.conn.execute("ALTER TABLE users ADD COLUMN last_name TEXT", []);
```

**Key Characteristics:**
- Uses `let _ =` to ignore result (column may already exist)
- Nullable columns (no NOT NULL constraint)
- Preserves all existing data
- Idempotent (safe to run multiple times)

**Risk Mitigation:** Migration runs on app startup; check column existence before attempting.

### 4.2 Rejected Pattern: DROP and Recreate

**Pattern Reference:** auth.rs:387-388

**Why Rejected:** Destroys existing phase_outputs data. Only acceptable for new/empty tables.

---

## 5. Schema Requirements for PLAN Phase

### 5.1 phase_outputs Modifications

**Requirement:** Add two nullable TEXT columns to existing phase_outputs table.

| Column | IM Code | Purpose | Constraint |
|--------|---------|---------|------------|
| system_prompt | IM-5001 | Store system prompt sent to LLM | TEXT NULL (backward compat) |
| user_input | IM-5002 | Store user input sent to LLM | TEXT NULL (backward compat) |

**Migration Approach:** Use ALTER TABLE ADD COLUMN pattern (per Section 4.1).

*Full SQL specifications to be defined in PLAN phase (IM-5010).*

### 5.2 session_conversations New Table

**Requirement:** Create new table for session-level conversation tracking.

| Requirement | Description |
|-------------|-------------|
| **Purpose** | Store prompts/responses during research (not after) |
| **FK Relationship** | Links to research_sessions.id (not briefs.id) |
| **Role Constraint** | 'user', 'assistant', 'system' (matches existing pattern) |
| **Indexing** | Index on session_id for efficient retrieval |
| **Phase Linking** | Optional phase_id to link messages to specific phases |

*Full CREATE TABLE and INDEX specifications to be defined in PLAN phase (IM-5030).*

---

## 6. Event Flow Notes

### 6.1 Current Flow (agent.rs → App.tsx → auth.rs)

```
agent.rs:132          App.tsx:295              auth.rs:1185
emit_phase_output() → listen<Payload>() → invoke("save_phase_output")
```

**Gap:** PhaseOutputPayload missing system_prompt and user_input fields.

### 6.2 Required Modifications

The following components require modification to close the prompt persistence gap:

| Component | File | IM Code | Modification Required |
|-----------|------|---------|----------------------|
| PhaseOutputPayload struct | agent.rs | IM-5001, IM-5002 | Add system_prompt and user_input fields |
| PhaseOutputPayload type | App.tsx | IM-5001, IM-5002 | Mirror Rust struct changes in TypeScript |
| Event listener | App.tsx:295 | IM-5003 | Update destructuring to include new fields |
| save_phase_output command | auth.rs:1185 | IM-5010 | Accept and persist new fields |

**Data Flow After Modification:**
```
agent.rs                    App.tsx                      auth.rs
emit_phase_output()    →    listen<Payload>()       →    save_phase_output()
[system_prompt, user_input] [destructure new fields]     [INSERT with new columns]
```

*Exact struct/type signatures to be specified in PRE-CODE phase (ICD contracts).*

---

## 7. Integration Dependencies

### 7.1 Sprint 1 Reuse (Multi-Turn Caching)

| IM Code | Component | Reuse Pattern |
|---------|-----------|---------------|
| IM-4001 | ChatMessage | Model for session_conversations |
| IM-4002 | ChatRole | Enum for role field validation |
| IM-4003 | MultiTurnRequest | Container for resume history |
| IM-4020 | generate_multi_turn() | Resume uses existing multi-turn |

### 7.2 Sprint 2 Layer Dependencies

```
Layer 1 (Data):       IM-5001, IM-5002, IM-5030
                              ↓
Layer 2 (Backend):    IM-5003, IM-5010, IM-5011, IM-5021, IM-5031, IM-5032
                              ↓
Layer 3 (Commands):   IM-5020 (resume_research_session)
                              ↓
Layer 4 (Frontend):   IM-5040, IM-5041, IM-5042
```

---

## 8. Risk Mitigations Confirmed

| Risk ID | Risk | Mitigation Strategy |
|---------|------|---------------------|
| R-01 | Schema migration breaks existing data | Nullable columns only (no NOT NULL) |
| R-02 | Large prompts cause UI lag | Lazy load prompts, collapse by default |
| R-03 | Resume fails from corrupted context | Validate phase_outputs before resume, rollback on error |
| R-04 | Multi-turn token overflow | Use 25-pair sliding window (conservative) |

---

## 9. PLAN Phase Inputs

The following must be specified in PLAN phase:

### 9.1 Backend Specifications
- [ ] IM-5003: emit_phase_output_with_prompts() - Full function signature
- [ ] IM-5010: save_phase_output_with_prompts - Tauri command signature
- [ ] IM-5011: get_phase_outputs_with_prompts - Tauri command signature
- [ ] IM-5020: resume_research_session - Complete algorithm
- [ ] IM-5021: reconstruct_session_context - Context building logic

### 9.2 Database Specifications
- [ ] IM-5030: session_conversations - CREATE TABLE statement
- [ ] IM-5031: add_session_message - Insert command
- [ ] IM-5032: get_session_conversation - Query command
- [ ] Migration script with idempotent checks

### 9.3 Frontend Specifications
- [ ] IM-5040: SessionDetailPanel - Props and state
- [ ] IM-5041: PromptViewCard - Collapsible component spec
- [ ] IM-5042: ResumeSessionButton - Click handler spec
- [ ] TypeScript type updates

---

## 10. Source References

| Topic | Source | Lines |
|-------|--------|-------|
| briefs schema | auth.rs | 270-279 |
| conversations schema | auth.rs | 288-295 |
| research_sessions schema | auth.rs | 318-329 |
| phase_outputs schema | auth.rs | 338-350 |
| ALTER TABLE pattern | auth.rs | 422-425 |
| DROP/recreate pattern | auth.rs | 387-388 |
| emit_phase_output | agent.rs | 132 |
| LLMRequest creation | agent.rs | 208-212 |
| Event listener | App.tsx | 295 |
| Phase output display | App.tsx | 1278-1305 |
| Provider context windows | L4-MANIFEST-MultiTurnCaching.md | Section 4 |

---

**Document Version:** 1.1 (Remediated - P0/P1 fixes applied)
**Created:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint 2)
**Status:** NOTES COMPLETE - Ready for PLAN

**Remediation History:**
| Version | Changes |
|---------|---------|
| 1.0 | Initial draft - FAIL (91/100) |
| 1.1 | P0: Removed implementation code from Sections 5 & 6. P1: Added Revisit Triggers, ASSUMPTION callouts |

---

*This document captures structured notes from RESEARCH phase for PLAN phase consumption.*
*CDP LODA Sprint 2: User Data Accessibility & Session Management*
