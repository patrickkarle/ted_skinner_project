# RESEARCH: User Data Accessibility & Session Management
## CDP LODA Sprint 2 - Phase 2 Findings

**Date:** 2025-11-28
**Status:** RESEARCH Complete
**Sprint:** User Data Accessibility

---

## 1. Research Objectives

From ULTRATHINK open questions (L4-MANIFEST Section 7):

| ID | Question | Priority |
|----|----------|----------|
| Q-01 | Should prompts be stored compressed? | Medium |
| Q-02 | Max conversation history for resume? | High |
| Q-03 | Should session_conversations replace briefs.conversations? | High |

Additional research needed:
- SQLite migration patterns for Tauri applications
- Existing schema structure and relationships

---

## 2. Codebase Investigation

### 2.1 Current Schema Analysis (auth.rs:270-350)

**Tables Discovered:**

```
briefs (auth.rs:270-279)
├── id INTEGER PRIMARY KEY
├── user_id INTEGER NOT NULL (FK → users)
├── company TEXT NOT NULL
├── model TEXT NOT NULL
├── manifest_name TEXT
├── content TEXT NOT NULL              ← Final brief content only
└── created_at TEXT

conversations (auth.rs:288-295)
├── id INTEGER PRIMARY KEY
├── brief_id INTEGER NOT NULL (FK → briefs)  ← Linked to BRIEFS
├── role TEXT NOT NULL ('user', 'assistant')
├── content TEXT NOT NULL
└── created_at TEXT

research_sessions (auth.rs:318-329)
├── id INTEGER PRIMARY KEY
├── user_id INTEGER NOT NULL (FK → users)
├── company, model, manifest_name TEXT
├── status TEXT ('running', 'completed', 'failed')
├── current_phase_id TEXT
└── created_at, updated_at TEXT

phase_outputs (auth.rs:338-350)
├── id INTEGER PRIMARY KEY
├── session_id INTEGER NOT NULL (FK → research_sessions)
├── phase_id, phase_name TEXT NOT NULL
├── status TEXT ('running', 'completed', 'failed')
├── output TEXT                        ← OUTPUT ONLY - NO PROMPTS
├── error TEXT
└── created_at, updated_at TEXT
```

### 2.2 Key Finding: Two Separate Conversation Systems

```
BRIEF CONVERSATION FLOW (Existing):
┌─────────────────────────────────────────────────────────────┐
│ research_sessions → completes → briefs → conversations      │
│                                                              │
│ Used for: Follow-up Q&A AFTER research is complete          │
│ Pattern: User asks questions about the generated brief      │
└─────────────────────────────────────────────────────────────┘

SESSION CONVERSATION FLOW (Proposed):
┌─────────────────────────────────────────────────────────────┐
│ research_sessions → session_conversations                    │
│                                                              │
│ Used for: Interaction DURING research (manifest prompts)    │
│ Pattern: User views prompts, resumes sessions, multi-turn   │
└─────────────────────────────────────────────────────────────┘
```

**Conclusion:** These are COMPLEMENTARY, not redundant.

### 2.3 Migration Pattern Analysis (auth.rs:360-430)

Two patterns observed:

**Pattern 1: ALTER TABLE ADD COLUMN (Safe)**
```rust
// auth.rs:422-425 - Adding nullable columns
let _ = self.conn.execute("ALTER TABLE users ADD COLUMN first_name TEXT", []);
let _ = self.conn.execute("ALTER TABLE users ADD COLUMN last_name TEXT", []);
```
- **Use when:** Adding nullable columns
- **Advantage:** Preserves existing data
- **Risk:** Low

**Pattern 2: DROP and Recreate (Destructive)**
```rust
// auth.rs:387-388 - Schema changes requiring table recreation
self.conn.execute("DROP TABLE IF EXISTS phase_outputs", [])?;
self.conn.execute("DROP TABLE IF EXISTS research_sessions", [])?;
```
- **Use when:** Changing column constraints or types
- **Advantage:** Clean schema
- **Risk:** Data loss (only use for new/empty tables)

---

## 3. Q-01 Answer: Should Prompts Be Stored Compressed?

### Investigation

| Factor | Analysis |
|--------|----------|
| **Typical Prompt Size** | System prompts: 500-2000 chars, User inputs: 100-5000 chars |
| **SQLite TEXT Limit** | Up to 1 billion bytes (effectively unlimited) |
| **Storage Growth** | 10 sessions × 5 phases × 3KB = 150KB (minimal) |
| **Compression Overhead** | Rust flate2 crate: ~5KB binary size, CPU cost per read/write |

### Recommendation: NO COMPRESSION

**Rationale:**
1. Storage cost is negligible (modern SSDs, SQLite handles TEXT efficiently)
2. Compression adds complexity (serialize → compress → store → decompress → deserialize)
3. Searchability lost (can't query compressed content)
4. Retrieval latency increased (~1-5ms for decompression)

**Decision:** Store prompts as plain TEXT. Revisit if storage exceeds 100MB.

---

## 4. Q-02 Answer: Max Conversation History for Resume?

### Provider Context Windows

**Source:** L4-MANIFEST-MultiTurnCaching.md Section 4 states:
> "Anthropic: 200K token context, OpenAI/DeepSeek: 64K-128K tokens, Gemini: 128K-1M tokens"

| Provider | Context Window | Safe History Limit |
|----------|---------------|-------------------|
| Claude 3.5 Sonnet | 200K tokens | ~80 message pairs |
| GPT-4o | 128K tokens | ~50 message pairs |
| DeepSeek | 64K tokens | ~25 message pairs |
| Gemini 1.5 | 128K-1M tokens | ~50-400 message pairs |

### Current Implementation

```rust
// llm.rs:482, 1435, 1599, 1643
"max_tokens": 4096  // OUTPUT limit, not input
```

The 4096 is output tokens. Input context windows are much larger.

### Recommendation: SLIDING WINDOW WITH PROVIDER DETECTION

**Implementation Strategy:**
```
┌─────────────────────────────────────────────────────────────┐
│ Session Resume Flow:                                         │
│                                                              │
│ 1. Load all phase_outputs for session                        │
│ 2. Detect provider from model string                         │
│ 3. Apply sliding window based on provider limits:            │
│    - anthropic: Last 80 message pairs                       │
│    - openai/deepseek: Last 50 message pairs                 │
│    - gemini: Last 100 message pairs                         │
│ 4. Build MultiTurnRequest with history                       │
│ 5. Continue from next phase                                  │
└─────────────────────────────────────────────────────────────┘
```

**Safe Default:** 25 message pairs (~50 messages total)
- Works for all providers including DeepSeek's 64K window
- **Assumption:** ~50K tokens estimated at 1K tokens/message pair (to be validated during testing)

**Decision:** Implement sliding window with provider-specific limits. Use 25 pairs as default.

---

## 5. Q-03 Answer: Should session_conversations Replace briefs.conversations?

### Analysis

| Table | Purpose | Lifecycle | FK Target |
|-------|---------|-----------|-----------|
| `conversations` | Brief follow-up Q&A | After research | briefs.id |
| `session_conversations` | Session prompts/responses | During research | research_sessions.id |

### User Journey

```
1. User starts research session
   └── session_conversations tracks manifest prompts (NEW)

2. Research completes → Brief generated
   └── Brief stored in briefs table

3. User asks follow-up questions about brief
   └── conversations table stores Q&A (EXISTING)

4. User wants to resume paused session
   └── session_conversations provides history for multi-turn (NEW)
```

### Recommendation: DO NOT REPLACE - KEEP BOTH

**Rationale:**
1. **Different purposes:** Brief Q&A vs session interaction
2. **Different lifecycles:** Post-completion vs during-execution
3. **Different FK relationships:** brief_id vs session_id
4. **Future flexibility:** Sessions may not always produce briefs

**Potential Enhancement (Future Backlog):**
- Add `session_id` column to `briefs` table for traceability
- This allows linking: session → brief → conversations

**Decision:** Create new `session_conversations` table. Keep existing `conversations` table unchanged.

---

## 6. Migration Strategy

### Phase 1: Add Prompt Columns to phase_outputs (Non-breaking)

**Approach:** Use ALTER TABLE ADD COLUMN pattern for nullable columns.
- **Pattern Reference:** auth.rs:422-425 (existing profile migration)
- **Columns to Add:** `system_prompt TEXT`, `user_input TEXT`
- **Safety:** Nullable columns preserve existing data
- **Detection:** Check if column exists before attempting migration

*Implementation details to be specified in PRE-CODE phase (IM-5010).*

### Phase 2: Create session_conversations Table (Additive)

**Approach:** Use CREATE TABLE IF NOT EXISTS pattern.
- **Table Purpose:** Store session-level conversation during research
- **Key Fields:** session_id (FK), phase_id (optional), role, content
- **Constraints:** Role limited to user/assistant/system (like existing conversations table)
- **Indexing:** Index on session_id for efficient retrieval

*Full schema and implementation to be specified in PRE-CODE phase (IM-5030).*

---

## 7. Research Summary

| Question | Answer | Impact |
|----------|--------|--------|
| Q-01: Compress prompts? | **NO** | Store as plain TEXT |
| Q-02: Max history? | **25 message pairs** | Sliding window, provider-aware |
| Q-03: Replace conversations? | **NO** | Keep both tables, separate purposes |

### Key Technical Decisions

1. **Migration:** Use `ALTER TABLE ADD COLUMN` pattern (safe, preserves data)
2. **Storage:** Plain TEXT, no compression
3. **History:** Sliding window with 25 pair default, provider-specific maximums
4. **Tables:** Additive - new session_conversations, modify phase_outputs

---

## 8. Dependencies for PHASE 3: NOTES

Research findings feed into:
- Schema migration scripts
- IM-5001/5002 implementation details
- IM-5020 resume logic (sliding window)
- IM-5030 table creation

---

**Document Version:** 1.0
**Created:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint 2)
**Status:** RESEARCH COMPLETE - Ready for NOTES

---

## 9. Source References

| Source | Location | Purpose |
|--------|----------|---------|
| briefs schema | auth.rs:270-279 | Table structure |
| conversations schema | auth.rs:288-295 | Table structure |
| research_sessions schema | auth.rs:318-329 | Table structure |
| phase_outputs schema | auth.rs:338-350 | Modification target |
| Migration pattern 1 | auth.rs:422-425 | ALTER TABLE ADD COLUMN |
| Migration pattern 2 | auth.rs:387-388 | DROP and recreate |
| add_conversation_message | auth.rs:773-801 | Conversation pattern |
| Multi-turn implementation | L4-MANIFEST-MultiTurnCaching.md | Sprint 1 reference |
