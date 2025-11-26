# PHASE 3: NOTES - Architectural Decisions

**Project:** Fullintel Sales Intelligence Generator
**Date:** 2025-11-19
**Status:** Completed

---

## Key Architectural Decisions

### Decision 1: Tool Registry Pattern

**Problem:** Manifest declares tools (`search_tool`, `finance_api`, etc.) but no execution layer exists.

**Decision:** Implement Rust trait-based tool registry

**Rationale:**
- Type-safe tool interface
- Easy to add new tools without modifying agent
- Tools can be mocked for testing
- Aligns with manifest-driven architecture

**Implementation:**
```rust
pub trait Tool: Send + Sync {
    async fn execute(&self, args: serde_json::Value) -> Result<String>;
    fn name(&self) -> &str;
    fn schema(&self) -> ToolSchema;
}

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>
}
```

---

### Decision 2: SQLite for State Persistence

**Problem:** Agent state lost on crash, no save/resume capability

**Decision:** Embedded SQLite database for session persistence

**Rationale:**
- Local-first (no server dependency)
- ACID compliance
- Fast queries for session history
- Cross-platform (works on Windows/Mac/Linux)
- Tauri supports SQLite plugins

**Schema:**
```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    company TEXT NOT NULL,
    status TEXT, -- 'running', 'completed', 'failed'
    current_phase TEXT,
    created_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE phase_outputs (
    id INTEGER PRIMARY KEY,
    session_id TEXT NOT NULL,
    phase_id TEXT NOT NULL,
    output_json TEXT,
    completed_at INTEGER,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

CREATE TABLE llm_calls (
    id INTEGER PRIMARY KEY,
    session_id TEXT,
    phase_id TEXT,
    provider TEXT,
    model TEXT,
    tokens_in INTEGER,
    tokens_out INTEGER,
    cost_usd REAL,
    latency_ms INTEGER,
    timestamp INTEGER
);
```

---

### Decision 3: Quality Gate Validators as Separate Module

**Problem:** Quality gates defined in YAML but not enforced in code

**Decision:** Create `quality_gates.rs` module with validator implementations

**Rationale:**
- Separation of concerns (orchestration vs validation)
- Gates are business logic, not agent logic
- Easy to test independently
- Can be extended without modifying agent

**Interface:**
```rust
pub struct QualityGateValidator {
    gates: Vec<QualityGate>
}

impl QualityGateValidator {
    pub fn validate(&self, phase_id: &str, output: &str) -> ValidationResult {
        // Find gates for this phase
        // Execute regex/heuristic checks
        // Return PASS/FAIL with detailed reason
    }
}
```

**Example Checks:**
- Coverage quantification: Regex for numbers + "articles"
- Generic text detection: Check for "placeholder", "[insert", "TODO"
- ROI calculations: Verify presence of dollar amounts and percentages
- Case study presence: Check for specific client names from database

---

### Decision 4: Progressive Disclosure UI Pattern

**Problem:** Overwhelming amount of information during 5-phase workflow

**Decision:** Step-by-step wizard UI with collapsible detail panels

**Rationale:**
- Reduces cognitive load
- Clear progress indication
- User can focus on current step
- Results easily accessible later

**Screen Flow:**
```
Setup Screen
  ├─ Company input field
  ├─ API key configuration
  └─ Start button

    ↓

Progress Screen (during execution)
  ├─ Phase progress bar (1/5, 2/5, etc.)
  ├─ Current phase name + description
  ├─ Live log output (collapsible)
  └─ Cancel button

    ↓

Results Screen
  ├─ Full markdown brief (preview)
  ├─ Copy to clipboard button
  ├─ Export PDF button
  ├─ Save for later button
  └─ Start new research button
```

---

### Decision 5: MVP Tool Stack

**Problem:** Many potential tool integrations, need to prioritize

**Decision:** Phase 1 MVP tools

| Tool Need | MVP Solution | Cost | Fallback |
|-----------|--------------|------|----------|
| Web Search | Tavily API | $0.001/search | LLM knowledge |
| Company Data | LLM extraction from search | Free | Manual input |
| News Search | NewsAPI.org | Free tier | Google News RSS |
| LinkedIn Contacts | Manual input prompt | Free | Apollo.io later |
| Case Studies | Local JSON file | Free | N/A |

**Rationale:**
- Minimize upfront costs
- Validate product-market fit first
- Easy to upgrade tools later
- Graceful degradation if APIs fail

---

### Decision 6: LLM Provider Strategy

**Problem:** Which LLM(s) to use for different phases?

**Decision:** Phase-specific model selection with cost optimization

| Phase | Model | Rationale | Cost/Run |
|-------|-------|-----------|----------|
| 1-3   | DeepSeek | Cheap, good for extraction | $0.001 |
| 4     | Local logic | No LLM needed (rule-based) | $0 |
| 5     | Claude Sonnet | Best instruction following | $0.025 |

**Total estimated cost:** ~$0.03 per brief (vs. $0.04 if all Claude)

**Fallback:** If DeepSeek fails → use Claude for all phases

---

### Decision 7: Security Architecture

**Problem:** API keys, user data, compliance

**Decisions:**
1. **API Key Storage:** Windows Credential Manager (not plaintext JSON)
2. **Data Retention:** Auto-delete sessions older than 90 days (GDPR compliance)
3. **Input Sanitization:** Escape special chars before LLM calls
4. **CSP Policy:** Define strict Content Security Policy in tauri.conf.json
5. **Rate Limiting:** Max 10 LLM calls/minute per user

---

### Decision 8: Error Handling Strategy

**Problem:** Many failure points (API rate limits, network issues, bad LLM outputs)

**Decision:** Graceful degradation with user feedback

**Strategy:**
```
Tool API fails
  → Retry 3x with exponential backoff
  → If still fails, show user error
  → Offer manual input alternative

LLM API fails
  → Try fallback provider (Claude → DeepSeek)
  → If all fail, save state and notify user
  → Allow resume later

Quality gate fails
  → Show specific failure reason
  → Offer to regenerate phase
  → Or allow manual editing of output
```

---

## Open Questions for Ted

1. **Budget:** What's acceptable cost per research brief?
   - Current estimate: $0.03-0.05 per brief (LLM only)
   - With paid tools: $0.10-0.50 per brief

2. **Data Access:** Does Fullintel have:
   - Internal media monitoring API we can use?
   - Existing company databases?
   - Case study database in structured format?

3. **Deployment:**
   - Desktop app only or web version later?
   - Single user or team collaboration features?
   - Cloud sync needed?

4. **Compliance:**
   - Data retention policy preferences?
   - GDPR/CCPA considerations?
   - LinkedIn scraping legal review status?

5. **Integration:**
   - Which CRM does sales team use?
   - Export format preferences (PDF, Word, both)?
   - Email integration needed?

---

## Risk Mitigation Strategies

### Technical Risks

**Risk:** API rate limits hit during demo
**Mitigation:** Pre-cache common company data, show cached results first

**Risk:** LLM generates poor quality briefs
**Mitigation:** Quality gates BLOCK completion, force regeneration with stricter prompts

**Risk:** Tool APIs change/break
**Mitigation:** Abstract tool interface, easy to swap implementations

### Business Risks

**Risk:** Cost per brief too high for ROI
**Mitigation:** Use cheaper models for phases 1-3, optimize prompts to reduce tokens

**Risk:** LinkedIn contact data unavailable
**Mitigation:** Make contact discovery optional, provide manual input UI

**Risk:** Generated briefs lack personalization
**Mitigation:** Add template customization, allow users to tune prompts

---

## Next Steps

**Completed:**
- ✅ ULTRATHINK (7 perspectives)
- ✅ RESEARCH (codebase + tools analysis)
- ✅ NOTES (architectural decisions)

**Next:**
- 📋 PLAN - Create L1-SAD (System Architecture Document)
- 📋 PRE-CODE - Design component specs (L3-CDD)
- 📋 MANIFEST - Build integration inventory (L4)
- 📋 TESTING PLAN - Define test specifications

---

**Document Status:** Complete
**Next Action:** Create L1-SAD in `docs/se-cpm/L1-SAD.md`
