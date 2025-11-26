# L1-SAD: System Architecture Document
## Fullintel Sales Intelligence Generator

**Document ID:** L1-SAD-FULLINTEL-001
**Version:** 1.0
**Date:** 2025-11-19
**Status:** Ready for Review
**Timeline:** Implement TODAY

---

## 1. Mission Intent

Build a **desktop application** that automates sales research for Fullintel sales team, generating ready-to-use opportunity briefs in under 5 minutes instead of 2-4 hours of manual work.

**Success Criteria:**
- ✅ Research brief generated in < 5 minutes
- ✅ Quality gates ensure no generic/placeholder content
- ✅ Cost per brief < $0.10
- ✅ Desktop app runs offline (cached data)
- ✅ One-click export to PDF/clipboard

---

## 2. System Overview

### 2.1 High-Level Architecture

```
┌─────────────────────────────────────────────────────┐
│              TAURI DESKTOP APP                      │
│                                                     │
│  ┌──────────────────────────────────────────────┐  │
│  │         REACT FRONTEND (TypeScript)          │  │
│  │  ┌────────┐  ┌─────────┐  ┌──────────────┐  │  │
│  │  │ Setup  │  │Progress │  │   Results    │  │  │
│  │  │ Screen │→ │ Screen  │→ │   Viewer     │  │  │
│  │  └────────┘  └─────────┘  └──────────────┘  │  │
│  └──────────────────┬───────────────────────────┘  │
│                     │ Tauri IPC                    │
│  ┌──────────────────▼───────────────────────────┐  │
│  │          RUST BACKEND                        │  │
│  │  ┌──────────────┐  ┌────────────────────┐   │  │
│  │  │   Manifest   │  │  Agent Orchestrator│   │  │
│  │  │    Parser    │  │   (5-phase runner) │   │  │
│  │  └──────────────┘  └─────────┬──────────┘   │  │
│  │                              │               │  │
│  │  ┌────────┬──────────┬──────▼─────┬───────┐ │  │
│  │  │  Tool  │   LLM    │  Quality   │SQLite │ │  │
│  │  │Registry│  Client  │   Gates    │ State │ │  │
│  │  └────────┴──────────┴────────────┴───────┘ │  │
│  └──────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### 2.2 Core Components

| Component | Responsibility | Atomic Level |
|-----------|---------------|--------------|
| **ManifestParser** | Parse/validate YAML workflow | ATOM |
| **AgentOrchestrator** | Execute 5-phase pipeline | COMPOUND |
| **ToolRegistry** | Manage external API tools | MOLECULE |
| **LLMClient** | Multi-provider LLM calls | MOLECULE |
| **QualityGates** | Validate phase outputs | MOLECULE |
| **StateManager** | SQLite persistence | MOLECULE |
| **UIComponents** | React screens | COMPOUND |

---

## 3. Functional Requirements

### REQ-SYS-001: Company Research Execution
**Priority:** CRITICAL
**Description:** User provides company name → system executes 5-phase workflow → generates opportunity brief
**Acceptance Criteria:**
- Input: Company name (text)
- Output: Markdown brief (FULLINTEL OPPORTUNITY BRIEF format)
- Time: < 5 minutes end-to-end
- Success Rate: > 95%

### REQ-SYS-002: Multi-LLM Support
**Priority:** HIGH
**Description:** Support Claude, Gemini, DeepSeek with automatic fallback
**Acceptance Criteria:**
- Claude primary for Phase 5
- DeepSeek for Phases 1-3 (cost optimization)
- Fallback to Claude if DeepSeek fails
- Model selection configurable

### REQ-SYS-003: Tool Integration
**Priority:** HIGH
**Description:** Integrate external APIs for data gathering
**Acceptance Criteria:**
- Tavily API for web search
- NewsAPI for news articles
- Manual input fallback for all tools
- Graceful degradation if APIs unavailable

### REQ-SYS-004: Quality Assurance
**Priority:** CRITICAL
**Description:** Enforce quality gates before completing
**Acceptance Criteria:**
- Block completion if quality gates fail
- Show specific failure reasons
- Allow regeneration or manual edit
- No generic/placeholder text in final output

### REQ-SYS-005: State Persistence
**Priority:** HIGH
**Description:** Save/resume research sessions
**Acceptance Criteria:**
- Auto-save every phase completion
- Resume from last successful phase on crash
- View past research history
- Delete sessions older than 90 days (privacy)

### REQ-SYS-006: Export Capabilities
**Priority:** MEDIUM
**Description:** Export results in multiple formats
**Acceptance Criteria:**
- Copy to clipboard (one-click)
- Export as PDF
- Export as Markdown file
- Include all sections from template

### REQ-SYS-007: Configuration Management
**Priority:** HIGH
**Description:** Secure API key storage and settings
**Acceptance Criteria:**
- API keys encrypted (Windows Credential Manager)
- Settings persist across restarts
- Model preferences configurable
- Cost tracking visible

---

## 4. Non-Functional Requirements

### NFR-001: Performance
- Phase execution: < 60 seconds each
- Total workflow: < 5 minutes
- UI responsiveness: < 100ms
- LLM streaming: Real-time updates

### NFR-002: Reliability
- Uptime: 99%+ (local app)
- Error recovery: Auto-retry 3x
- Data persistence: No data loss on crash
- Graceful degradation: All tools have fallbacks

### NFR-003: Security
- API keys: Encrypted storage
- Input sanitization: Prevent injection
- Rate limiting: 10 LLM calls/min max
- Data retention: 90-day auto-delete

### NFR-004: Usability
- Setup time: < 2 minutes (first run)
- Learning curve: < 10 minutes
- Error messages: Actionable, specific
- Progress indication: Real-time

### NFR-005: Maintainability
- Code coverage: > 80%
- Documentation: All public APIs
- Modular design: Easy to swap tools/LLMs
- Logging: Debug mode available

### NFR-006: Cost
- Cost per brief: < $0.10 (LLM + tools)
- Free tier tools: Where possible
- Cost tracking: Visible to user
- Budget alerts: Warn at $10/month

---

## 5. System Interfaces

### 5.1 External APIs

| API | Purpose | Phase | Cost | Fallback |
|-----|---------|-------|------|----------|
| **Tavily** | Web search | 1, 2 | $0.001/search | LLM knowledge |
| **NewsAPI** | News articles | 2 | Free tier | Google News |
| **Claude API** | LLM calls | All | $0.025/brief | DeepSeek |
| **DeepSeek API** | LLM calls | 1-3 | $0.001/brief | Claude |

### 5.2 Data Schemas

**CompanyProfile:**
```typescript
{
  company_name: string;
  industry_classification: string;
  revenue_tier: string;
  geographic_footprint: string;
  communications_leader_name?: string;
  communications_leader_title?: string;
}
```

**SituationAnalysis:**
```typescript
{
  scenario_type: 'CRISIS' | 'LAUNCH' | 'MA' | 'REGULATORY' | 'COMPETITIVE' | 'EXECUTIVE';
  coverage_volume: number;
  coverage_momentum: 'increasing' | 'stable' | 'declining';
  urgency_level: 'HIGH' | 'MEDIUM' | 'LOW';
}
```

---

## 6. Component Specifications

### 6.1 AgentOrchestrator (COMPOUND)
**Responsibility:** Execute 5-phase workflow with dependency tracking
**Interfaces:**
- `run_workflow(company: String) -> Result<String>`
- `execute_phase(phase: &Phase) -> Result<String>`
- `check_dependencies(phase: &Phase) -> bool`

**Dependencies:**
- ManifestParser
- ToolRegistry
- LLMClient
- QualityGates
- StateManager

### 6.2 ToolRegistry (MOLECULE)
**Responsibility:** Manage and execute external tools
**Interfaces:**
- `register(name: String, tool: Box<dyn Tool>)`
- `execute(name: &str, args: Value) -> Result<String>`
- `list_available() -> Vec<String>`

**Registered Tools:**
- `search_tool` → TavilySearch
- `news_search_tool` → NewsAPISearch
- `manual_input_tool` → PromptUser

### 6.3 LLMClient (MOLECULE)
**Responsibility:** Multi-provider LLM orchestration
**Interfaces:**
- `generate(req: LLMRequest) -> Result<String>`
- `stream(req: LLMRequest) -> Stream<String>`

**Providers:**
- Anthropic (Claude)
- Google (Gemini)
- DeepSeek

### 6.4 QualityGates (MOLECULE)
**Responsibility:** Validate phase outputs against business rules
**Interfaces:**
- `validate(phase_id: &str, output: &str) -> ValidationResult`

**Gates:**
- Coverage quantification check
- Generic text detection
- ROI calculation verification
- Case study presence

---

## 7. Data Flow

### 7.1 Happy Path

```
User enters "TechCorp"
  ↓
Agent loads manifest
  ↓
Phase 1: Context & Firmographics
  ├─ Tool: TavilySearch("TechCorp revenue industry")
  ├─ LLM: Extract structured CompanyProfile
  └─ Store: context["CompanyProfile"] = {...}
  ↓
Phase 2: Situation Analysis
  ├─ Tool: NewsAPISearch("TechCorp", last_14_days)
  ├─ LLM: Classify scenario, analyze momentum
  ├─ QualityGate: Check coverage_volume quantified
  └─ Store: context["SituationAnalysis"] = {...}
  ↓
Phase 3: Comms Team Intelligence
  ├─ Tool: ManualInput("Enter VP of Comms name")
  ├─ LLM: Map scenario → pain points
  └─ Store: context["pain_points_list"] = [...]
  ↓
Phase 4: Solution Matching
  ├─ Logic: scenario_type → solution from logic_map
  ├─ Tool: CaseStudySearch(scenario_type)
  └─ Store: context["solution_package"] = {...}
  ↓
Phase 5: Brief Generation
  ├─ LLM: Claude Sonnet with full context
  ├─ QualityGate: No generic text, ROI present, case study included
  └─ Output: markdown_file
  ↓
Display results → Copy/Export
```

### 7.2 Error Handling

```
Tool API fails
  → Retry 3x (exponential backoff)
  → If still fails: Fallback to manual input or skip

LLM API fails
  → Try fallback provider
  → If all fail: Save state, notify user, allow resume

Quality Gate fails
  → Block completion
  → Show failure reason
  → Offer regenerate or manual edit
```

### 7.3 API Rate Limiting Strategy

**Objective:** Prevent rate limit violations while maximizing throughput and minimizing latency. All LLM providers enforce rate limits; exceeding them results in 429 errors and temporary bans.

#### 7.3.1 Provider-Specific Rate Limits

| Provider | Tier | RPM (Requests/Min) | TPM (Tokens/Min) | Daily Limit | Backoff Strategy |
|----------|------|-------------------|------------------|-------------|------------------|
| **Anthropic Claude** | Free | 5 | 10,000 | 25 requests | Exponential |
| **Anthropic Claude** | Tier 1 | 50 | 100,000 | 1,000 requests | Exponential |
| **Google Gemini** | Free | 15 | 1M | 1,500 requests | Exponential |
| **DeepSeek** | Standard | 60 | 2M | Unlimited | Linear |

**Assumptions:**
- Default to free tier limits unless user configures API tier
- Conservative limits prevent unexpected charges
- User can override in configuration if they have higher tier

#### 7.3.2 Rate Limiting Implementation

**Token Bucket Algorithm:**
```rust
struct RateLimiter {
    tokens: f64,              // Current token count
    capacity: f64,            // Max tokens (RPM limit)
    refill_rate: f64,         // Tokens per second (RPM / 60)
    last_refill: Instant,     // Last refill timestamp
}

impl RateLimiter {
    fn try_acquire(&mut self) -> Result<(), Duration> {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            Ok(())
        } else {
            // Calculate wait time for next token
            let wait_seconds = (1.0 - self.tokens) / self.refill_rate;
            Err(Duration::from_secs_f64(wait_seconds))
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}
```

**Per-Provider Instances:**
- Each LLM provider gets independent RateLimiter instance
- Configuration loaded from settings file
- Default to conservative free tier limits

#### 7.3.3 Retry Backoff Strategies

**Exponential Backoff (Anthropic, Google):**
```
Base delay: 1 second
Max retries: 4
Backoff formula: min(base_delay * 2^attempt + jitter, max_backoff)
Max backoff: 32 seconds
Jitter: random(0, base_delay * 0.5)

Retry sequence:
  Attempt 1: Wait 1.0-1.5s  (1s + jitter)
  Attempt 2: Wait 2.0-3.0s  (2s + jitter)
  Attempt 3: Wait 4.0-6.0s  (4s + jitter)
  Attempt 4: Wait 8.0-12.0s (8s + jitter)
  Attempt 5: FAIL (max retries exceeded)

Total max time: ~27 seconds before failure
```

**Linear Backoff (DeepSeek):**
```
Base delay: 2 seconds
Max retries: 3
Backoff formula: base_delay * attempt + jitter
Max backoff: 10 seconds
Jitter: random(0, 1s)

Retry sequence:
  Attempt 1: Wait 2.0-3.0s  (2s + jitter)
  Attempt 2: Wait 4.0-5.0s  (4s + jitter)
  Attempt 3: Wait 6.0-7.0s  (6s + jitter)
  Attempt 4: FAIL (max retries exceeded)

Total max time: ~15 seconds before failure
```

**Jitter Rationale:**
- Prevents "thundering herd" when multiple requests retry simultaneously
- Uses random jitter (not fixed intervals) to spread retries over time
- Cryptographically secure random not required (use thread_rng)

#### 7.3.4 429 Rate Limit Response Handling

**When API returns 429 (Too Many Requests):**

```rust
match response.status() {
    StatusCode::TOO_MANY_REQUESTS => {
        // Check for Retry-After header
        let retry_after = response.headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        match retry_after {
            Some(duration) => {
                // Honor server-provided wait time
                tokio::time::sleep(duration).await;
                retry_request()
            }
            None => {
                // Fall back to exponential backoff
                apply_exponential_backoff(attempt);
                retry_request()
            }
        }
    }
    // ... other status codes
}
```

**Priority:**
1. If `Retry-After` header present → Honor exact duration
2. If no header → Use exponential/linear backoff based on provider
3. If max retries exceeded → Return RateLimitError to caller

#### 7.3.5 Timeout Configuration

**Per-Request Timeouts:**
```
Phase 1-3 (DeepSeek/Gemini):
  - Connection timeout: 5 seconds
  - Read timeout: 30 seconds
  - Total timeout: 35 seconds

Phase 5 (Claude Sonnet):
  - Connection timeout: 5 seconds
  - Read timeout: 60 seconds (longer output)
  - Total timeout: 65 seconds

Tool API calls:
  - Connection timeout: 3 seconds
  - Read timeout: 10 seconds
  - Total timeout: 13 seconds
```

**Rationale:**
- Phase 5 generates longer outputs (briefs), needs more time
- Phases 1-3 are shorter extractions, can timeout faster
- Tool APIs (Tavily, NewsAPI) are simple searches, should be fast

**Timeout Handling:**
```rust
match tokio::time::timeout(timeout_duration, api_call()).await {
    Ok(Ok(response)) => Ok(response),
    Ok(Err(api_error)) => Err(api_error),
    Err(_elapsed) => Err(LLMError::Timeout(timeout_duration)),
}
```

#### 7.3.6 Circuit Breaker Pattern

**Prevents cascading failures when provider consistently fails:**

```rust
struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    failure_threshold: u32,    // 5 consecutive failures
    success_count: u32,
    success_threshold: u32,    // 2 consecutive successes
    open_until: Option<Instant>,
    timeout_duration: Duration, // 60 seconds
}

enum CircuitState {
    Closed,      // Normal operation
    Open,        // Blocking all requests, waiting for timeout
    HalfOpen,    // Allowing test request
}

impl CircuitBreaker {
    fn call<F, T>(&mut self, f: F) -> Result<T, CircuitBreakerError>
    where F: FnOnce() -> Result<T, LLMError>
    {
        match self.state {
            CircuitState::Open => {
                if self.open_until.unwrap() <= Instant::now() {
                    // Timeout expired, try one test request
                    self.state = CircuitState::HalfOpen;
                } else {
                    return Err(CircuitBreakerError::Open);
                }
            }
            CircuitState::HalfOpen | CircuitState::Closed => {}
        }

        match f() {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(error) => {
                self.on_failure();
                Err(CircuitBreakerError::RequestFailed(error))
            }
        }
    }

    fn on_success(&mut self) {
        self.failure_count = 0;
        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    self.state = CircuitState::Closed;
                    self.success_count = 0;
                }
            }
            CircuitState::Closed => {}
            CircuitState::Open => unreachable!(),
        }
    }

    fn on_failure(&mut self) {
        self.success_count = 0;
        self.failure_count += 1;

        if self.failure_count >= self.failure_threshold {
            self.state = CircuitState::Open;
            self.open_until = Some(Instant::now() + self.timeout_duration);
        }
    }
}
```

**When Circuit Opens:**
- After 5 consecutive failures from a provider
- All subsequent requests immediately fail (fast-fail)
- After 60 seconds, allow ONE test request (HalfOpen state)
- If test succeeds → Close circuit, resume normal operation
- If test fails → Reopen circuit for another 60 seconds

**Benefits:**
- Prevents wasting time/cost on failing provider
- Automatically triggers fallback to next provider
- Self-healing when provider recovers
- Reduces latency during outages (fail-fast)

#### 7.3.7 Rate Limit Budget Tracking

**Cost Tracking per Session:**
```rust
struct CostTracker {
    total_llm_cost: f64,
    total_tool_cost: f64,
    request_count: u32,
    token_usage: HashMap<String, u32>, // provider -> tokens
}

impl CostTracker {
    fn record_llm_request(&mut self, provider: &str, tokens: u32, cost: f64) {
        self.total_llm_cost += cost;
        self.request_count += 1;
        *self.token_usage.entry(provider.to_string()).or_insert(0) += tokens;

        // Check budget threshold
        if self.total_cost() > 0.10 {
            warn!("Session cost ${:.4} exceeds $0.10 target", self.total_cost());
        }
    }

    fn total_cost(&self) -> f64 {
        self.total_llm_cost + self.total_tool_cost
    }
}
```

**Budget Alerts:**
- Warn at $0.10 (per-brief target)
- Hard limit at $0.25 (prevents runaway costs)
- Display cost breakdown in UI

#### 7.3.8 Traceability

**L0 Requirements:**
- **NFR-003:** Rate limiting: 10 LLM calls/min max
- **NFR-006:** Cost per brief < $0.10

**L2 Interface Requirements:**
- Rate limiting implemented in LLMClient (IM-3010)
- RateLimitError variant (IM-3004-V4)
- Circuit breaker prevents cascading failures

**L4 Implementation:**
- RateLimiter struct with token bucket algorithm
- CircuitBreaker struct with Open/Closed/HalfOpen states
- CostTracker for budget enforcement

**L5 Test Coverage:**
- Test rate limiter prevents exceeding RPM limits
- Test exponential/linear backoff sequences
- Test circuit breaker opens after 5 failures
- Test Retry-After header honored
- Test timeout handling for all phases

---

## 8. Deployment Architecture

### 8.1 Desktop Application (Tauri)
- **Platform:** Windows 10+, macOS 11+, Linux (Ubuntu 20.04+)
- **Installer:** MSI (Windows), DMG (macOS), AppImage (Linux)
- **Auto-update:** Tauri updater plugin
- **Size:** < 50MB installed

### 8.2 Data Storage
- **Config:** `%APPDATA%/fullintel-agent/config.json`
- **Database:** `%APPDATA%/fullintel-agent/sessions.db` (SQLite)
- **Logs:** `%APPDATA%/fullintel-agent/logs/` (rotating, 7-day retention)

---

## 9. Testing Strategy

### 9.1 Unit Tests (80% coverage target)
- All public functions in Rust modules
- Mock external APIs
- Test quality gate logic

### 9.2 Integration Tests
- Full 5-phase workflow with mocked tools
- LLM API integration (record/replay)
- Database persistence

### 9.3 E2E Tests
- Happy path: Company input → Brief output
- Error scenarios: API failures, quality gate failures
- Resume from crash

---

## 10. Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to Brief | < 5 min | Average workflow duration |
| Quality Gate Pass Rate | > 90% | First-time pass percentage |
| Cost per Brief | < $0.10 | LLM + tool API costs |
| User Satisfaction | > 4/5 | Post-use survey |
| Error Rate | < 5% | Workflows with unhandled errors |

---

## 11. Timeline (TODAY)

- ✅ ULTRATHINK, RESEARCH, NOTES: Complete
- 🔄 PLAN (L1-SAD): In progress
- ⏳ PRE-CODE: 30 min
- ⏳ MANIFEST: 30 min
- ⏳ TESTING PLAN: 30 min
- ⏳ PRE-IMPLEMENTATION REVIEW: 15 min
- ⏳ IMPLEMENT: 3-4 hours
- ⏳ EXECUTE TESTS: 1 hour
- ⏳ POST-IMPLEMENTATION REVIEW: 15 min

**Total:** ~6-7 hours to working prototype

---

**Document Status:** Ready for Implementation
**Next:** Create PRE-CODE specs (component details)
