# PHASE 3: NOTES - Architectural Decisions

**Document Classification:** DOC-NOTES-001
**Project:** Fullintel Sales Intelligence Generator
**Date:** 2025-11-20
**Status:** Completed (Taxonomy v3.0 Compliant)
**Parent Document:** DOC-RESEARCH-001
**Traceability Chain:** L0-REQ → L1-SAD-1.1 → DOC-RESEARCH-001 → DOC-NOTES-001

---

## Taxonomy Compliance

### Level 1: Document Classification
- **DOC-NOTES-001** - Architectural Notes document

### Level 2: Component Classifications (Preliminary Skeleton)

**Implementation Components:**
- **NOTES-001-IMPL-001** - Tool Registry Pattern (trait-based architecture)
- **NOTES-001-IMPL-002** - Quality Gate Validator Module
- **NOTES-001-IMPL-003** - LLM Provider Routing Logic
- **NOTES-001-IMPL-004** - Error Recovery Strategy

**Database Components:**
- **NOTES-001-DB-001** - SQLite State Persistence Schema
- **NOTES-001-DB-002** - Session Management Tables
- **NOTES-001-DB-003** - LLM Call Tracking

**UI Components:**
- **NOTES-001-UI-001** - Progressive Disclosure UI Pattern
- **NOTES-001-UI-002** - Setup Screen Component
- **NOTES-001-UI-003** - Progress Screen Component
- **NOTES-001-UI-004** - Results Screen Component

**Integration Components:**
- **NOTES-001-INTEG-001** - Tavily API Integration
- **NOTES-001-INTEG-002** - NewsAPI Integration
- **NOTES-001-INTEG-003** - Anthropic Claude API
- **NOTES-001-INTEG-004** - Google Gemini API
- **NOTES-001-INTEG-005** - DeepSeek API

**Security Components:**
- **NOTES-001-SEC-001** - Windows Credential Manager Integration
- **NOTES-001-SEC-002** - Input Sanitization Layer
- **NOTES-001-SEC-003** - CSP Policy Configuration
- **NOTES-001-SEC-004** - Rate Limiting Middleware

**Error Handling Components:**
- **NOTES-001-ERR-001** - Exponential Backoff Retry Logic
- **NOTES-001-ERR-002** - Fallback Provider Chain
- **NOTES-001-ERR-003** - State Recovery After Crash

**Data Transformation Components:**
- **NOTES-001-TRANSFORM-001** - YAML to Rust Struct Deserialization
- **NOTES-001-TRANSFORM-002** - LLM Response to Phase Output Mapping

### Level 3: Technical Tags (Preliminary)

```rust
/**
 * @taxonomy FOC-22    # Asynchronous Functions - trait Tool execution
 * @taxonomy DMC-08    # Database Operations - SQLite persistence
 * @taxonomy SRC-02    # Encryption - API key storage
 * @taxonomy SRC-06    # Rate Limiting - LLM call throttling
 * @taxonomy DMC-05    # Hash Maps - context state management
 * @taxonomy CSE-05    # Control Flow - error recovery branching
 * @taxonomy FOC-06    # Class/Struct - Manifest, Agent, LLMClient
 * @taxonomy TVC-01    # Unit Testing - quality gate validators
 */
```

---

## Traceability to DOC-RESEARCH-001

This document builds upon findings from DOC-RESEARCH-001:

| Component-ID | Research Finding | Architectural Decision |
|--------------|------------------|------------------------|
| NOTES-001-IMPL-001 | RESEARCH-001-IMPL-002 (Agent::execute_phase) | Tool Registry Pattern needed |
| NOTES-001-DB-001 | RESEARCH-001-FUNC-001 (No persistence layer) | SQLite for state persistence |
| NOTES-001-IMPL-002 | RESEARCH-001-CLASS-003 (QualityGate struct) | Separate validator module |
| NOTES-001-UI-001 | RESEARCH-001-API-003 (Tauri Window events) | Progressive disclosure pattern |
| NOTES-001-IMPL-003 | RESEARCH-001-IMPL-001 (LLMClient::generate) | Phase-specific model selection |
| NOTES-001-SEC-001 | RESEARCH-001-API-001 (set_api_key command) | Windows Credential Manager |
| NOTES-001-ERR-001 | RESEARCH-001-IMPL-002 (Error propagation) | Exponential backoff strategy |

---

## Key Architectural Decisions

### Decision 1: Tool Registry Pattern
**Component-ID:** NOTES-001-IMPL-001
**Confidence:** HIGH
**Traceability:** RESEARCH-001-IMPL-002 (Agent::execute_phase comment "REAL IMPLEMENTATION SWITCH")

**Problem:** Manifest declares tools (`search_tool`, `finance_api`, etc.) but no execution layer exists in current codebase.

**Decision:** Implement Rust trait-based tool registry

**Rationale:**
- Type-safe tool interface leveraging Rust's trait system
- Easy to add new tools without modifying agent orchestration logic
- Tools can be mocked for testing (dependency injection)
- Aligns with manifest-driven architecture principles

**Implementation Contract:**
```rust
// @taxonomy FOC-22 (Async Functions)
// @taxonomy FOC-06 (Traits)
pub trait Tool: Send + Sync {
    async fn execute(&self, args: serde_json::Value) -> Result<String>;
    fn name(&self) -> &str;
    fn schema(&self) -> ToolSchema;
}

// @taxonomy DMC-05 (Hash Maps)
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>
}
```

**Unknowns:**
- **UNKNOWN-INTEG-001** (Confidence: MEDIUM): Tavily API authentication method (API key header vs. query param)
- **UNKNOWN-INTEG-002** (Confidence: LOW): NewsAPI rate limits on free tier (unknown if sufficient for production)

---

### Decision 2: SQLite for State Persistence
**Component-IDs:** NOTES-001-DB-001, NOTES-001-DB-002, NOTES-001-DB-003
**Confidence:** HIGH
**Traceability:** RESEARCH-001-FUNC-001 (No persistence layer found in research)

**Problem:** Agent state lost on crash, no save/resume capability as required by L1-SAD SR-009 (Crash Recovery).

**Decision:** Embedded SQLite database for session persistence

**Rationale:**
- Local-first architecture (no server dependency, meets MC-003)
- ACID compliance guarantees data integrity
- Fast queries for session history (<50ms target per L2-ICD-01)
- Cross-platform compatibility (Windows/Mac/Linux)
- Tauri v2 supports SQLite plugins via `tauri-plugin-sql`

**Database Schema:**
```sql
-- @taxonomy DMC-08 (Database Operations)
-- Session tracking table
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,              -- UUID format
    company TEXT NOT NULL,            -- Target company name
    status TEXT CHECK(status IN ('running', 'completed', 'failed')),
    current_phase TEXT,               -- phase_1, phase_2, etc.
    created_at INTEGER NOT NULL,      -- Unix timestamp ms
    updated_at INTEGER NOT NULL       -- Unix timestamp ms
);

-- Phase outputs for resume capability
CREATE TABLE phase_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    phase_id TEXT NOT NULL,           -- phase_1, phase_2, etc.
    output_json TEXT,                 -- Serialized phase output
    completed_at INTEGER,             -- Unix timestamp ms
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

-- Cost tracking per L1-SAD MO-003 (< $0.10 per brief)
CREATE TABLE llm_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    phase_id TEXT,
    provider TEXT,                    -- anthropic, google, deepseek
    model TEXT,                       -- claude-3-5-sonnet, etc.
    tokens_in INTEGER,
    tokens_out INTEGER,
    cost_usd REAL,                    -- Calculated cost
    latency_ms INTEGER,
    timestamp INTEGER,
    FOREIGN KEY(session_id) REFERENCES sessions(id)
);

-- Index for fast session history queries (L2-ICD-01: < 100ms)
CREATE INDEX idx_sessions_created ON sessions(created_at DESC);
CREATE INDEX idx_phase_outputs_session ON phase_outputs(session_id, phase_id);
CREATE INDEX idx_llm_calls_session ON llm_calls(session_id);
```

**Unknowns:**
- **UNKNOWN-DB-001** (Confidence: MEDIUM): SQLite WAL mode performance on network drives (if users store app data on NAS)
- **UNKNOWN-DB-002** (Confidence: HIGH): Data retention policy automation (90-day auto-delete requires background task)

**Additional Dependencies Required:**
```toml
# Cargo.toml addition needed
tauri-plugin-sql = { version = "2.0", features = ["sqlite"] }
rusqlite = { version = "0.31", features = ["bundled"] }
```

---

### Decision 3: Quality Gate Validators as Separate Module
**Component-ID:** NOTES-001-IMPL-002
**Confidence:** HIGH
**Traceability:** RESEARCH-001-CLASS-003 (QualityGate struct in manifest.rs)

**Problem:** Quality gates defined in YAML manifest but not enforced in code. Current agent.rs has no validation logic.

**Decision:** Create `quality_gates.rs` module with validator implementations

**Rationale:**
- Separation of concerns (orchestration logic vs validation logic)
- Gates represent business rules, not agent workflow logic
- Easy to test independently with unit tests
- Can be extended without modifying agent orchestration
- Aligns with L1-SAD MO-002 (Quality Standardization: 90%+ pass rate)

**Module Interface:**
```rust
// @taxonomy FOC-06 (Struct)
// @taxonomy TVC-01 (Unit Testing - testable validators)
pub struct QualityGateValidator {
    gates: Vec<QualityGate>
}

impl QualityGateValidator {
    /// Validate phase output against all configured gates for that phase
    /// Returns detailed pass/fail with specific gate that failed
    pub fn validate(&self, phase_id: &str, output: &str) -> ValidationResult {
        // Find gates for this phase
        let phase_gates = self.gates.iter()
            .filter(|g| g.phase_id == phase_id)
            .collect::<Vec<_>>();

        // Execute regex/heuristic checks
        for gate in phase_gates {
            match self.check_gate(gate, output) {
                GateResult::Pass => continue,
                GateResult::Fail(reason) => {
                    return ValidationResult::Failed {
                        gate_name: gate.name.clone(),
                        reason,
                    };
                }
            }
        }

        ValidationResult::Passed
    }
}
```

**Example Quality Gate Checks:**
1. **Coverage Quantification** (Phase 1): Regex for numbers + "articles" (e.g., "43 articles analyzed")
2. **Generic Text Detection** (All Phases): Check for "placeholder", "[insert", "TODO", "TBD"
3. **ROI Calculations** (Phase 4): Verify presence of dollar amounts ($) and percentages (%)
4. **Case Study Presence** (Phase 4): Check for specific client names from database

**Unknowns:**
- **UNKNOWN-IMPL-001** (Confidence: MEDIUM): How to handle subjective quality gates (e.g., "sounds professional")
- **UNKNOWN-IMPL-002** (Confidence: HIGH): Should gates block workflow or just warn user?

---

### Decision 4: Progressive Disclosure UI Pattern
**Component-IDs:** NOTES-001-UI-001, NOTES-001-UI-002, NOTES-001-UI-003, NOTES-001-UI-004
**Confidence:** HIGH
**Traceability:** RESEARCH-001-API-003 (Tauri Window events for progress updates)

**Problem:** 5-phase workflow generates overwhelming amount of information. Users need clear progress indication without cognitive overload.

**Decision:** Step-by-step wizard UI with collapsible detail panels

**Rationale:**
- Reduces cognitive load during lengthy workflow
- Clear progress indication builds user trust
- User can focus on current step, not entire workflow
- Results easily accessible later via session history
- Aligns with L1-SAD Success Criterion: "Ease of Use" (< 10 min learning curve)

**Screen Flow:**
```
Setup Screen (NOTES-001-UI-002)
  ├─ Company input field (validation: 1-200 chars)
  ├─ API key configuration status indicator
  └─ Start Research button (disabled until keys configured)

    ↓ (invoke run_research command)

Progress Screen (NOTES-001-UI-003) - during execution
  ├─ Phase progress bar (1/5, 2/5, etc.)
  ├─ Current phase name + description from manifest
  ├─ Live log output (collapsible, auto-scroll)
  ├─ Estimated time remaining (based on historical data)
  └─ Cancel button (graceful shutdown, save state)

    ↓ (listen to workflow_completed event)

Results Screen (NOTES-001-UI-004)
  ├─ Full markdown brief (preview with syntax highlighting)
  ├─ Session metadata (duration, cost, timestamp)
  ├─ Copy to clipboard button
  ├─ Export PDF button (invoke export_to_pdf command)
  ├─ Save for later button (mark session as favorite)
  └─ Start new research button (return to Setup)
```

**Event Listeners Required:**
```typescript
// @taxonomy FOC-22 (Async Functions)
import { listen } from '@tauri-apps/api/event';

// Phase progress updates (every 5 seconds per L2-ICD-01)
listen('phase_progress', (event) => {
  const { phase_id, message, progress_percent } = event.payload;
  updateProgressUI(phase_id, message, progress_percent);
});

// Quality gate failures
listen('quality_gate_failed', (event) => {
  const { phase_id, gate_name, reason, retry_attempt } = event.payload;
  showQualityGateError(gate_name, reason, retry_attempt);
});

// Workflow completion
listen('workflow_completed', (event) => {
  const { success, duration_ms, cost_usd } = event.payload;
  navigateToResults(event.payload.session_id);
});
```

**Unknowns:**
- **UNKNOWN-UI-001** (Confidence: LOW): Should app support multiple concurrent research workflows?

---

### Decision 5: MVP Tool Stack
**Component-IDs:** NOTES-001-INTEG-001, NOTES-001-INTEG-002
**Confidence:** MEDIUM
**Traceability:** L1-SAD MO-003 (Cost < $0.10 per brief)

**Problem:** Many potential tool integrations, need to prioritize based on cost and value.

**Decision:** Phase 1 MVP tools with graceful degradation

| Tool Need | MVP Solution | Cost | Fallback | Component-ID |
|-----------|--------------|------|----------|--------------|
| Web Search | Tavily API | $0.001/search | LLM knowledge cutoff | NOTES-001-INTEG-001 |
| Company Data | LLM extraction from search | Free | Manual input prompt | N/A |
| News Search | NewsAPI.org | Free tier (100 req/day) | Google News RSS scraping | NOTES-001-INTEG-002 |
| LinkedIn Contacts | Manual input prompt | Free | Apollo.io (future) | UNKNOWN-INTEG-003 |
| Case Studies | Local JSON file | Free | N/A | N/A |

**Rationale:**
- Minimize upfront costs to validate product-market fit
- Easy to upgrade tools later (trait-based registry supports swapping)
- Graceful degradation if APIs fail (don't block workflow)
- Tavily chosen over SerpAPI (better JSON structure, lower cost)
- NewsAPI free tier sufficient for MVP (5 users × 20 briefs/day = 100 req/day)

**Estimated Cost Breakdown:**
```
Tavily:  5 searches/brief × $0.001 = $0.005
NewsAPI: Free tier                 = $0.000
LLM:     See Decision 6            = $0.030
TOTAL:                               $0.035 per brief ✅ (under $0.10 target)
```

**Unknowns:**
- **UNKNOWN-INTEG-003** (Confidence: LOW): Apollo.io API pricing for contact data (alternative to manual input)
- **UNKNOWN-INTEG-004** (Confidence: MEDIUM): LinkedIn scraping legal status (future if automation needed)

**Additional Dependencies Required:**
```toml
# Cargo.toml addition needed
reqwest = { version = "0.11", features = ["json"] }  # Already present
serde_json = "1.0"                                   # Already present
```

---

### Decision 6: LLM Provider Strategy
**Component-IDs:** NOTES-001-IMPL-003, NOTES-001-INTEG-003, NOTES-001-INTEG-004, NOTES-001-INTEG-005
**Confidence:** HIGH
**Traceability:** RESEARCH-001-IMPL-001 (LLMClient::generate supports multi-provider)

**Problem:** Which LLM(s) to use for different phases? Need to balance cost and quality.

**Decision:** Phase-specific model selection with cost optimization

| Phase | Model | Rationale | Cost/Run | Component-ID |
|-------|-------|-----------|----------|--------------|
| 1: Context Research | DeepSeek v3 | Cheap, good for extraction | $0.001 | NOTES-001-INTEG-005 |
| 2: Situation Analysis | DeepSeek v3 | Cheap, good for analysis | $0.001 | NOTES-001-INTEG-005 |
| 3: Comms Strategy | DeepSeek v3 | Cheap, good for structured output | $0.001 | NOTES-001-INTEG-005 |
| 4: Solution Mapping | Local logic | No LLM needed (rule-based matching) | $0 | N/A |
| 5: Brief Generation | Claude Sonnet 3.5 | Best instruction following, long-form | $0.025 | NOTES-001-INTEG-003 |

**Total Estimated Cost:** ~$0.028 per brief (vs. $0.040 if all Claude)

**Fallback Strategy:**
```rust
// @taxonomy CSE-05 (Control Flow - provider fallback)
async fn generate_with_fallback(req: LLMRequest) -> Result<String> {
    // Try primary provider
    match llm_client.generate(req.clone()).await {
        Ok(output) => Ok(output),
        Err(e) if is_rate_limit_error(&e) => {
            // Try fallback: DeepSeek → Claude
            warn!("Primary provider rate limited, trying fallback");
            let fallback_req = req.with_model("claude-3-5-sonnet");
            llm_client.generate(fallback_req).await
        },
        Err(e) => Err(e),
    }
}
```

**Model Configuration in AppConfig:**
```rust
// @taxonomy FOC-06 (Struct - configuration)
pub struct ModelPreferences {
    pub phase_1_model: String,  // Default: "deepseek-chat"
    pub phase_2_model: String,  // Default: "deepseek-chat"
    pub phase_3_model: String,  // Default: "deepseek-chat"
    pub phase_4_model: Option<String>, // None (logic-based, no LLM)
    pub phase_5_model: String,  // Default: "claude-3-5-sonnet"
}
```

**Unknowns:**
- **UNKNOWN-IMPL-003** (Confidence: MEDIUM): DeepSeek API stability (new provider, unknown uptime SLA)
- **UNKNOWN-COST-001** (Confidence: LOW): Future pricing changes (all LLM providers subject to price updates)

---

### Decision 7: Security Architecture
**Component-IDs:** NOTES-001-SEC-001, NOTES-001-SEC-002, NOTES-001-SEC-003, NOTES-001-SEC-004
**Confidence:** HIGH
**Traceability:** L1-SAD MC-005 (API keys encrypted), SR-007 (API security)

**Problem:** API keys, user data, compliance requirements (GDPR, client confidentiality).

**Security Decisions:**

**1. API Key Storage (NOTES-001-SEC-001):**
- **Solution:** Windows Credential Manager (not plaintext JSON)
- **Rust Crate:** `keyring = "2.0"` for cross-platform credential storage
- **Why:** OS-level encryption, meets L1-SAD MC-005 requirement
```rust
// @taxonomy SRC-02 (Encryption - credential storage)
use keyring::Entry;

async fn save_api_key(provider: &str, key: String) -> Result<()> {
    let entry = Entry::new("fullintel-app", provider)?;
    entry.set_password(&key)?;
    Ok(())
}
```

**2. Data Retention (NOTES-001-SEC-002):**
- **Solution:** Auto-delete sessions older than 90 days (GDPR compliance)
- **Implementation:** Background task in service launcher
- **Why:** Minimize data exposure, regulatory compliance
```sql
-- Scheduled cleanup query (runs daily)
DELETE FROM sessions
WHERE created_at < (unixepoch() * 1000) - (90 * 24 * 60 * 60 * 1000);
```

**3. Input Sanitization (NOTES-001-SEC-003):**
- **Solution:** Escape special chars before LLM calls
- **Rust Crate:** Use `html_escape` or custom validator
- **Why:** Prevent prompt injection attacks
```rust
// @taxonomy SRC-08 (Input Validation)
fn sanitize_company_name(input: &str) -> Result<String> {
    if input.len() > 200 {
        return Err(anyhow!("Company name too long (max 200 chars)"));
    }

    // Remove control characters, null bytes
    let sanitized = input.chars()
        .filter(|c| !c.is_control())
        .collect::<String>();

    Ok(sanitized)
}
```

**4. CSP Policy (NOTES-001-SEC-004):**
- **Solution:** Strict Content Security Policy in tauri.conf.json
- **Configuration:**
```json
{
  "tauri": {
    "security": {
      "csp": "default-src 'self'; connect-src 'self' https://api.anthropic.com https://generativelanguage.googleapis.com https://api.deepseek.com https://api.tavily.com https://newsapi.org"
    }
  }
}
```

**5. Rate Limiting (NOTES-001-SEC-004):**
- **Solution:** Max 10 LLM calls/minute per user
- **Implementation:** Token bucket algorithm in LLMClient
- **Why:** Prevent API key abuse, cost control
```rust
// @taxonomy SRC-06 (Rate Limiting)
use governor::{Quota, RateLimiter};

pub struct LLMClient {
    client: Client,
    api_key: String,
    rate_limiter: RateLimiter<...>,  // 10 requests/minute
}
```

**Unknowns:**
- **UNKNOWN-SEC-001** (Confidence: HIGH): macOS/Linux equivalent to Windows Credential Manager (keyring crate should handle, needs testing)
- **UNKNOWN-SEC-002** (Confidence: MEDIUM): GDPR compliance for session metadata (is company name considered PII?)

**Additional Dependencies Required:**
```toml
# Cargo.toml additions needed
keyring = "2.0"                           # Secure credential storage
governor = "0.6"                          # Rate limiting
html-escape = "0.2"                       # Input sanitization
```

---

### Decision 8: Error Handling Strategy
**Component-IDs:** NOTES-001-ERR-001, NOTES-001-ERR-002, NOTES-001-ERR-003
**Confidence:** HIGH
**Traceability:** RESEARCH-001-IMPL-002 (Agent error propagation), L2-ICD-01 Error Conditions

**Problem:** Many failure points (API rate limits, network issues, bad LLM outputs). Need graceful degradation.

**Error Handling Strategy:**

**1. Tool API Failures (NOTES-001-ERR-001):**
```rust
// @taxonomy CSE-05 (Control Flow - retry logic)
async fn execute_tool_with_retry(tool: &dyn Tool, args: Value) -> Result<String> {
    let mut backoff_ms = 1000; // Start with 1 second
    let max_retries = 3;

    for attempt in 0..max_retries {
        match tool.execute(args.clone()).await {
            Ok(result) => return Ok(result),
            Err(e) if is_rate_limit(&e) => {
                warn!("Tool rate limited, retry {}/{}", attempt + 1, max_retries);
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2; // Exponential backoff
            },
            Err(e) => {
                // Non-retryable error, offer manual input
                return Err(e);
            }
        }
    }

    Err(anyhow!("Tool failed after {} retries", max_retries))
}
```

**Strategy:**
- Retry 3x with exponential backoff (1s, 2s, 4s)
- If still fails, show user error with details
- Offer manual input alternative (e.g., "Tavily failed, enter company info manually")

**2. LLM API Failures (NOTES-001-ERR-002):**
```rust
// @taxonomy CSE-05 (Control Flow - provider fallback)
async fn generate_with_fallback_chain(req: LLMRequest) -> Result<String> {
    // Try primary provider
    if let Ok(result) = llm_client.generate(req.clone()).await {
        return Ok(result);
    }

    // Try fallback provider (Claude → DeepSeek or vice versa)
    warn!("Primary LLM failed, trying fallback");
    let fallback_model = get_fallback_model(&req.model);
    let fallback_req = req.with_model(fallback_model);

    match llm_client.generate(fallback_req).await {
        Ok(result) => Ok(result),
        Err(e) => {
            // All providers failed, save state and notify user
            error!("All LLM providers failed: {}", e);
            save_workflow_state().await?;
            Err(anyhow!("LLM service unavailable, workflow saved for later resume"))
        }
    }
}
```

**Strategy:**
- Try fallback provider (Claude → DeepSeek or vice versa)
- If all fail, save workflow state to SQLite
- Allow user to resume later when APIs recover

**3. Quality Gate Failures (NOTES-001-ERR-003):**
```rust
// @taxonomy CSE-05 (Control Flow - quality validation)
async fn execute_phase_with_quality_check(
    phase: &Phase,
    validator: &QualityGateValidator
) -> Result<String> {
    let max_attempts = 3;

    for attempt in 0..max_attempts {
        // Generate phase output
        let output = generate_phase_output(phase).await?;

        // Validate against quality gates
        match validator.validate(&phase.id, &output) {
            ValidationResult::Passed => return Ok(output),
            ValidationResult::Failed { gate_name, reason } => {
                warn!("Quality gate {} failed: {}", gate_name, reason);

                // Emit event to UI
                emit_event("quality_gate_failed", json!({
                    "phase_id": phase.id,
                    "gate_name": gate_name,
                    "reason": reason,
                    "retry_attempt": attempt + 1,
                }));

                // Last attempt? Ask user
                if attempt == max_attempts - 1 {
                    return Err(anyhow!(
                        "Quality gate {} failed after {} attempts: {}",
                        gate_name, max_attempts, reason
                    ));
                }
            }
        }
    }

    unreachable!()
}
```

**Strategy:**
- Show specific failure reason to user
- Offer to regenerate phase (retry with stricter prompt)
- Or allow manual editing of output (advanced users)

**Error Response Format (per L2-ICD-01 Section 5.1):**
```rust
// @taxonomy FOC-06 (Struct - error response)
#[derive(Serialize)]
struct ErrorResponse {
    error_code: ErrorCode,          // INVALID_INPUT, TOOL_FAILED, etc.
    message: String,                // Human-readable error
    details: String,                // Technical details for debugging
    recovery_action: String,        // Suggested user action
}

enum ErrorCode {
    InvalidInput,
    DependencyNotMet,
    QualityGateFailed,
    ToolExecutionFailed,
    LlmFailure,
    StateCorruption,
}
```

**Unknowns:**
- **UNKNOWN-ERR-001** (Confidence: MEDIUM): How long to wait before declaring API "permanently failed"?
- **UNKNOWN-ERR-002** (Confidence: HIGH): Should quality gate failures count against cost budget (wasted LLM calls)?

---

## Open Questions for Ted

**Integration Questions:**
1. **Budget Clarification:** What's the absolute maximum acceptable cost per research brief?
   - Current estimate: $0.035-0.050 per brief (LLM + tools)
   - With paid tool upgrades: $0.10-0.50 per brief
   - **UNKNOWN-COST-002** (Confidence: HIGH)

2. **Data Access:** Does Fullintel have:
   - Internal media monitoring API we can integrate? (cheaper than NewsAPI)
   - Existing company databases with structured data?
   - Case study database in CSV/JSON format? (currently using local JSON)
   - **UNKNOWN-INTEG-006** (Confidence: HIGH)

3. **Deployment Preferences:**
   - Desktop app only (current scope) or web version later?
   - Single user or team collaboration features needed?
   - Cloud sync for session history across devices?
   - **UNKNOWN-DEPLOY-001** (Confidence: MEDIUM)

4. **Compliance Requirements:**
   - Preferred data retention policy (currently 90 days)?
   - GDPR/CCPA considerations for storing company names?
   - LinkedIn scraping legal review status?
   - **UNKNOWN-SEC-003** (Confidence: HIGH)

5. **Integration Plans:**
   - Which CRM does sales team use? (Salesforce, HubSpot, other)
   - Export format preferences (PDF, Word, both)?
   - Email integration needed (send briefs directly)?
   - **UNKNOWN-INTEG-007** (Confidence: MEDIUM)

---

## Risk Mitigation Strategies

### Technical Risks

**Risk:** API rate limits hit during demo/production
**Mitigation:** Pre-cache common company data in SQLite, show cached results first before making live API calls
**Component-ID:** NOTES-001-DB-001 (cache layer in sessions table)

**Risk:** LLM generates poor quality briefs
**Mitigation:** Quality gates BLOCK workflow completion, force regeneration with stricter prompts, track failure rate
**Component-ID:** NOTES-001-IMPL-002 (QualityGateValidator)

**Risk:** Tool APIs change/break unexpectedly
**Mitigation:** Abstract tool interface (trait), easy to swap implementations, monitor API version announcements
**Component-ID:** NOTES-001-IMPL-001 (Tool registry pattern)

### Business Risks

**Risk:** Cost per brief too high for ROI
**Mitigation:** Use cheaper models for phases 1-3 (DeepSeek), optimize prompts to reduce token count, cache frequently-used data
**Component-ID:** NOTES-001-IMPL-003 (phase-specific model selection)

**Risk:** LinkedIn contact data unavailable (no automation API)
**Mitigation:** Make contact discovery optional, provide manual input UI, consider Apollo.io paid tier
**Component-ID:** UNKNOWN-INTEG-003 (Apollo.io integration)

**Risk:** Generated briefs lack personalization (generic content)
**Mitigation:** Add template customization UI, allow users to tune prompts per sales vertical, A/B test outputs
**Component-ID:** UNKNOWN-UI-002 (template customization)

---

## Next Steps

**Completed:**
- ✅ ULTRATHINK (7 perspectives) - DOC-ULTRATHINK-001
- ✅ RESEARCH (codebase + tools analysis) - DOC-RESEARCH-001
- ✅ NOTES (architectural decisions with Taxonomy v3.0) - DOC-NOTES-001

**Next (MANIFEST Continuous Building):**
- 📋 PLAN - Create L1-SAD with PC-{ID}-* Component-IDs (upgrade from NOTES-{ID}-*)
- 📋 PRE-CODE - Add detailed specs, resolve UNKNOWN-* placeholders, add technical tags
- 📋 TESTING PLAN - Define test specifications FROM manifest
- 📋 MANIFEST - Formalize as DOC-MANIFEST-001 (final contract)

---

## Appendix A: Unknowns Inventory

### Critical Unknowns (Block Implementation)
- **UNKNOWN-INTEG-001** - Tavily API authentication method
- **UNKNOWN-DB-002** - 90-day retention automation mechanism
- **UNKNOWN-IMPL-002** - Quality gate behavior (block vs. warn)
- **UNKNOWN-SEC-001** - Cross-platform credential storage testing

### Important Unknowns (Impact Cost/Quality)
- **UNKNOWN-INTEG-002** - NewsAPI free tier limits
- **UNKNOWN-IMPL-003** - DeepSeek API stability/uptime
- **UNKNOWN-COST-002** - Acceptable maximum cost per brief
- **UNKNOWN-INTEG-006** - Fullintel internal data sources
- **UNKNOWN-SEC-003** - GDPR compliance for session data

### Nice-to-Know Unknowns (Future Enhancements)
- **UNKNOWN-INTEG-003** - Apollo.io API pricing
- **UNKNOWN-INTEG-004** - LinkedIn scraping legal status
- **UNKNOWN-UI-001** - Multiple concurrent workflows support
- **UNKNOWN-UI-002** - Template customization requirements
- **UNKNOWN-DEPLOY-001** - Web version / team collaboration needs
- **UNKNOWN-INTEG-007** - CRM integration requirements

---

## Appendix B: Component-ID Cross-Reference

| Component-ID | Category | Description | Technical Tag | Parent Doc |
|--------------|----------|-------------|---------------|------------|
| NOTES-001-IMPL-001 | Implementation | Tool Registry Pattern | FOC-22 | DOC-NOTES-001 |
| NOTES-001-IMPL-002 | Implementation | Quality Gate Validator | TVC-01 | DOC-NOTES-001 |
| NOTES-001-IMPL-003 | Implementation | LLM Provider Routing | CSE-05 | DOC-NOTES-001 |
| NOTES-001-IMPL-004 | Implementation | Error Recovery Strategy | CSE-05 | DOC-NOTES-001 |
| NOTES-001-DB-001 | Database | SQLite State Persistence | DMC-08 | DOC-NOTES-001 |
| NOTES-001-DB-002 | Database | Session Management | DMC-08 | DOC-NOTES-001 |
| NOTES-001-DB-003 | Database | LLM Call Tracking | DMC-08 | DOC-NOTES-001 |
| NOTES-001-UI-001 | User Interface | Progressive Disclosure | N/A | DOC-NOTES-001 |
| NOTES-001-UI-002 | User Interface | Setup Screen | N/A | DOC-NOTES-001 |
| NOTES-001-UI-003 | User Interface | Progress Screen | N/A | DOC-NOTES-001 |
| NOTES-001-UI-004 | User Interface | Results Screen | N/A | DOC-NOTES-001 |
| NOTES-001-INTEG-001 | Integration | Tavily API | N/A | DOC-NOTES-001 |
| NOTES-001-INTEG-002 | Integration | NewsAPI | N/A | DOC-NOTES-001 |
| NOTES-001-INTEG-003 | Integration | Anthropic Claude | FOC-22 | DOC-NOTES-001 |
| NOTES-001-INTEG-004 | Integration | Google Gemini | FOC-22 | DOC-NOTES-001 |
| NOTES-001-INTEG-005 | Integration | DeepSeek | FOC-22 | DOC-NOTES-001 |
| NOTES-001-SEC-001 | Security | Credential Manager | SRC-02 | DOC-NOTES-001 |
| NOTES-001-SEC-002 | Security | Data Retention | N/A | DOC-NOTES-001 |
| NOTES-001-SEC-003 | Security | Input Sanitization | SRC-08 | DOC-NOTES-001 |
| NOTES-001-SEC-004 | Security | Rate Limiting | SRC-06 | DOC-NOTES-001 |
| NOTES-001-ERR-001 | Error Handling | Exponential Backoff | CSE-05 | DOC-NOTES-001 |
| NOTES-001-ERR-002 | Error Handling | Fallback Providers | CSE-05 | DOC-NOTES-001 |
| NOTES-001-ERR-003 | Error Handling | Quality Gate Retry | CSE-05 | DOC-NOTES-001 |
| NOTES-001-TRANSFORM-001 | Data Transform | YAML Deserialization | N/A | DOC-NOTES-001 |
| NOTES-001-TRANSFORM-002 | Data Transform | LLM Response Mapping | DMC-05 | DOC-NOTES-001 |

**Total Component-IDs:** 25

---

**Document Status:** Complete - Taxonomy v3.0 Compliant - Ready for Microgate 2 Validation
**Next Document:** DOC-PLAN-001 (L1-SAD System Architecture Document)
