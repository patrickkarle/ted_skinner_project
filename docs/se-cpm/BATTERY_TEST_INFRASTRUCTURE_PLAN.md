# Battery Test Infrastructure Plan - TEST-PRE-CODE Phase
**Date:** 2025-11-22
**Phase:** TEST-PRE-CODE (Step 3: Infrastructure Plan)
**Purpose:** Define mocks, fixtures, and utilities for strategic battery tests

---

## Infrastructure Overview

### Organization Structure
```
src/
├── agent_orchestrator.rs          # Production code (171 IM codes)
├── llm_client.rs                  # Production code (62 IM codes)
├── quality_gates.rs               # Production code (39 IM codes)
├── state_manager.rs               # Production code (38 IM codes)
└── ui/
    └── components/                # Production code (17 IM codes)

tests/
├── common/                        # Shared test infrastructure
│   ├── mod.rs                     # Exports all test utilities
│   ├── mocks.rs                   # Mock implementations
│   ├── fixtures.rs                # Test data loaders
│   ├── builders.rs                # Test data builders
│   └── assertions.rs              # Custom assertion helpers
│
├── fixtures/                      # Test data files
│   ├── manifests/
│   │   ├── valid_manifest.yaml
│   │   ├── minimal_manifest.yaml
│   │   ├── invalid_manifest.yaml
│   │   └── missing_keys_manifest.yaml
│   ├── companies/
│   │   └── test_companies.json
│   ├── prompts/
│   │   └── test_prompts.json
│   └── responses/
│       └── test_responses.json
│
├── unit/                          # Unit tests (56 tests)
│   ├── test_agent_orchestrator.rs # 21 tests
│   ├── test_llm_client.rs         # 13 tests
│   ├── test_quality_gates.rs      # 8 tests
│   ├── test_state_manager.rs      # 8 tests
│   └── test_frontend.rs           # 6 tests
│
├── integration/                   # Integration tests (23 tests)
│   ├── test_ao_state_manager.rs   # Orchestrator ↔ StateManager
│   ├── test_ao_llm_client.rs      # Orchestrator ↔ LLMClient
│   ├── test_ao_quality_gates.rs   # Orchestrator ↔ QualityGates
│   ├── test_llm_providers.rs      # Multi-provider fallback
│   └── test_cross_component.rs    # 8 cross-component tests
│
└── e2e/                           # E2E tests (12 tests)
    ├── test_research_workflow.rs  # Complete workflows
    ├── test_performance.rs         # Multi-modal (behavior + perf)
    └── test_error_recovery.rs      # Error handling workflows
```

---

## Mock Strategy

### 1. MockLLMClient

**Purpose**: Simulate LLM API behavior without external dependencies

**Interface**:
```rust
pub struct MockLLMClient {
    responses: HashMap<String, LLMResponse>,  // prompt → response mapping
    failures: Vec<LLMError>,                  // Queue of errors to return
    call_count: Arc<Mutex<usize>>,            // Track API calls
    latency_ms: Option<u64>,                  // Simulate API latency
}

impl MockLLMClient {
    pub fn new() -> Self;
    pub fn with_response(prompt: &str, response: LLMResponse) -> Self;
    pub fn with_failure(error: LLMError) -> Self;
    pub fn with_latency(latency_ms: u64) -> Self;
    pub fn call_count(&self) -> usize;
    pub fn reset(&mut self);
}

impl LLMProvider for MockLLMClient {
    fn generate(&self, request: LLMRequest) -> Result<LLMResponse, LLMError>;
}
```

**Behaviors**:
- **Happy path**: Return pre-configured response for given prompt
- **Error simulation**: Return errors from queue (rate limit, timeout, API error)
- **Latency simulation**: Sleep for configured duration before returning
- **Call tracking**: Count number of API calls for verification

**Usage Example**:
```rust
let mock_client = MockLLMClient::new()
    .with_response("Analyze Acme Corp", LLMResponse {
        content: "Acme Corp is a software company...",
        model: "claude-3-5-sonnet",
        token_usage: TokenUsage { input_tokens: 100, output_tokens: 200, total_tokens: 300 },
        cost: 0.05,
    })
    .with_failure(LLMError::RateLimitError("Rate limit exceeded".into()))
    .with_latency(100); // 100ms latency

// First call succeeds with response
let result1 = mock_client.generate(request1);
assert!(result1.is_ok());

// Second call fails with rate limit
let result2 = mock_client.generate(request2);
assert_eq!(result2.unwrap_err(), LLMError::RateLimitError);

// Verify call count
assert_eq!(mock_client.call_count(), 2);
```

**IM Codes Validated**: Enables testing of IM-2014 (generate_llm_response), IM-3012 (LLMClient::generate), all LLM-related error codes

---

### 2. MockStateManager

**Purpose**: Simulate database operations without SQLite dependency

**Interface**:
```rust
pub struct MockStateManager {
    sessions: Arc<Mutex<HashMap<String, Session>>>,       // session_id → session
    contexts: Arc<Mutex<HashMap<String, WorkflowContext>>>, // session_id → context
    phase_completions: Arc<Mutex<Vec<PhaseCompletion>>>,  // All phase completions
    fail_operations: Vec<StateOperation>,                 // Operations that should fail
}

#[derive(Debug, PartialEq)]
pub enum StateOperation {
    CreateSession,
    SaveContext,
    SavePhaseCompletion,
    LoadContext,
}

impl MockStateManager {
    pub fn new() -> Self;
    pub fn with_session(session_id: &str, session: Session) -> Self;
    pub fn with_failure(operation: StateOperation) -> Self;
    pub fn get_sessions(&self) -> Vec<Session>;
    pub fn get_contexts(&self) -> Vec<WorkflowContext>;
    pub fn reset(&mut self);
}

impl StateManager for MockStateManager {
    fn create_session(&self, company: &str) -> Result<String, StateError>;
    fn save_context(&self, session_id: &str, context: &WorkflowContext) -> Result<(), StateError>;
    fn load_context(&self, session_id: &str) -> Result<WorkflowContext, StateError>;
    fn save_phase_completion(&self, session_id: &str, phase: PhaseCompletion) -> Result<(), StateError>;
    // ... other methods
}
```

**Behaviors**:
- **In-memory storage**: All sessions/contexts stored in HashMaps
- **Deterministic session IDs**: Return predictable IDs ("session-00001", "session-00002", etc.)
- **Operation failure simulation**: Configurable failures for specific operations
- **State inspection**: Retrieve all stored sessions/contexts for verification

**Usage Example**:
```rust
let mock_state = MockStateManager::new()
    .with_failure(StateOperation::CreateSession); // First create will fail

// First create_session fails
let result1 = mock_state.create_session("Test Corp");
assert!(result1.is_err());
assert_eq!(result1.unwrap_err(), StateError::SessionCreationFailed);

// Remove failure, next create succeeds
mock_state.fail_operations.clear();
let session_id = mock_state.create_session("Test Corp").unwrap();
assert_eq!(session_id, "session-00001");

// Verify session stored
let sessions = mock_state.get_sessions();
assert_eq!(sessions.len(), 1);
assert_eq!(sessions[0].company, "Test Corp");
```

**IM Codes Validated**: Enables testing of IM-2010 (workflow state), IM-5xxx (StateManager methods), all state-related error codes

---

### 3. MockQualityGates

**Purpose**: Simulate quality validation without actual gate implementations

**Interface**:
```rust
pub struct MockQualityGates {
    gate_results: HashMap<String, ValidationResult>,  // gate_name → result
    default_score: u8,                                // Default score (0-100)
    always_pass: bool,                                // Override to always pass
    always_fail: bool,                                // Override to always fail
}

impl MockQualityGates {
    pub fn new() -> Self;
    pub fn with_score(score: u8) -> Self;
    pub fn with_gate_result(gate_name: &str, result: ValidationResult) -> Self;
    pub fn always_pass() -> Self;
    pub fn always_fail() -> Self;
    pub fn reset(&mut self);
}

impl QualityGateValidator for MockQualityGates {
    fn validate(&self, text: &str, gate_types: &[String]) -> Result<ValidationResult, QualityError>;
    fn calculate_quality_score(&self, results: &[ValidationResult]) -> u8;
}
```

**Behaviors**:
- **Configurable scores**: Return pre-configured score (0-100)
- **Per-gate results**: Different results for different gates
- **Always pass/fail**: Override for testing edge cases
- **Gate inspection**: Track which gates were checked

**Usage Example**:
```rust
let mock_gates = MockQualityGates::new()
    .with_score(99)
    .with_gate_result("NoGenericText", ValidationResult {
        passed: true,
        score: 100,
        failures: vec![],
    })
    .with_gate_result("CoverageQuantification", ValidationResult {
        passed: true,
        score: 98,
        failures: vec![],
    });

// Validate with multiple gates
let result = mock_gates.validate("Test output", &["NoGenericText", "CoverageQuantification"]);
assert!(result.is_ok());
assert_eq!(result.unwrap().score, 99);

// Test always_fail scenario
let failing_gates = MockQualityGates::always_fail();
let result = failing_gates.validate("Test output", &[]);
assert_eq!(result.unwrap().score, 0);
```

**IM Codes Validated**: Enables testing of IM-2015 (validate_output), IM-4010 (QualityGates::validate), quality gate error codes

---

### 4. MockUIWindow (Tauri)

**Purpose**: Simulate Tauri window event emission without frontend

**Interface**:
```rust
pub struct MockUIWindow {
    emitted_events: Arc<Mutex<Vec<EmittedEvent>>>,  // All emitted events
    should_fail: bool,                               // Emit should fail
}

#[derive(Debug, Clone)]
pub struct EmittedEvent {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: SystemTime,
}

impl MockUIWindow {
    pub fn new() -> Self;
    pub fn with_failure() -> Self;
    pub fn get_events(&self) -> Vec<EmittedEvent>;
    pub fn get_events_by_type(&self, event_type: &str) -> Vec<EmittedEvent>;
    pub fn clear_events(&mut self);
    pub fn event_count(&self) -> usize;
}

impl Window for MockUIWindow {
    fn emit(&self, event: &str, payload: impl Serialize) -> Result<(), TauriError>;
}
```

**Behaviors**:
- **Event capture**: Store all emitted events with timestamps
- **Event inspection**: Retrieve events by type or get all
- **Failure simulation**: Configure emit to return error
- **Event ordering**: Maintain order of emissions

**Usage Example**:
```rust
let mock_window = MockUIWindow::new();

// Emit progress events
mock_window.emit("phase_start", json!({"phase": "research"}));
mock_window.emit("phase_progress", json!({"progress": 50}));
mock_window.emit("phase_complete", json!({"phase": "research"}));

// Verify events
assert_eq!(mock_window.event_count(), 3);

let phase_events = mock_window.get_events_by_type("phase_complete");
assert_eq!(phase_events.len(), 1);
assert_eq!(phase_events[0].data["phase"], "research");
```

**IM Codes Validated**: Enables testing of IM-2020 (emit_progress), UI event handling codes

---

## Fixture Strategy

### 1. Test Manifests

**Location**: `tests/fixtures/manifests/`

**valid_manifest.yaml**:
```yaml
# Complete valid manifest for happy path testing
project:
  name: "Test Research Project"
  version: "1.0.0"

llm_config:
  anthropic_api_key: "test-anthropic-key-12345"
  gemini_api_key: "test-gemini-key-67890"
  deepseek_api_key: "test-deepseek-key-abcde"
  default_model: "claude-3-5-sonnet-20241022"
  temperature: 0.7
  max_tokens: 4000

database:
  path: "test_research.db"
  enable_wal: true

tools:
  - name: "web_search"
    description: "Search the web for information"
    parameters:
      query: "string"
  - name: "scrape_website"
    description: "Extract content from URL"
    parameters:
      url: "string"
  - name: "analyze_text"
    description: "Perform text analysis"
    parameters:
      text: "string"

phases:
  - name: "company_analysis"
    description: "Analyze company basics"
    dependencies: []
    tools: ["web_search"]
    prompt_template: "Analyze the company {{company}} and provide key insights."
    quality_gates: ["NoGenericText", "CoverageQuantification"]

  - name: "research"
    description: "Conduct deep research"
    dependencies: ["company_analysis"]
    tools: ["web_search", "scrape_website"]
    prompt_template: "Research {{company}} using {{tool_results}}."
    quality_gates: ["NoGenericText", "CoverageQuantification", "CaseStudyPresent"]
```

**minimal_manifest.yaml**:
```yaml
# Minimal required fields for edge case testing
project:
  name: "Minimal Test"

llm_config:
  anthropic_api_key: "test-key"
  default_model: "claude-3-5-sonnet-20241022"

database:
  path: "test.db"

tools: []

phases:
  - name: "basic_phase"
    description: "Basic phase"
    dependencies: []
    tools: []
    prompt_template: "Basic prompt"
    quality_gates: []
```

**invalid_manifest.yaml**:
```yaml
# Syntax errors for YAML parse error testing
project:
  name: "Invalid Test"
  version: 1.0.0  # Should be string, not number

llm_config:
  anthropic_api_key: test-key  # Missing quotes
  default_model: claude-3-5-sonnet

database
  path: "test.db"  # Missing colon after database

tools: [  # Unclosed bracket

phases:
  - name: "phase1"
    description: "Test"
```

**missing_keys_manifest.yaml**:
```yaml
# Missing required API keys for error testing
project:
  name: "Missing Keys Test"

llm_config:
  # No API keys defined
  default_model: "claude-3-5-sonnet-20241022"

database:
  path: "test.db"

tools: []
phases: []
```

**Usage**:
```rust
// Load fixture in tests
let manifest_path = "tests/fixtures/manifests/valid_manifest.yaml";
let orchestrator = AgentOrchestrator::new(manifest_path, llm_client, state_manager);
assert!(orchestrator.is_ok());
```

---

### 2. Test Companies

**Location**: `tests/fixtures/companies/test_companies.json`

```json
{
  "valid": [
    "Acme Corporation",
    "TechStart Inc",
    "Global Solutions Ltd",
    "Innovation Labs"
  ],
  "edge_cases": {
    "empty": "",
    "whitespace_only": "   ",
    "single_char": "A",
    "very_long": "A".repeat(1000),
    "special_chars": "Compa!@#$%^&*()ny",
    "unicode": "株式会社テスト",
    "with_newlines": "Company\nName\nWith\nNewlines"
  },
  "injection_attempts": {
    "sql_injection": "Company'; DROP TABLE sessions; --",
    "xss": "Company<script>alert('xss')</script>",
    "command_injection": "Company; rm -rf /"
  }
}
```

**Usage**:
```rust
let companies: TestCompanies = load_fixture("tests/fixtures/companies/test_companies.json");

// Test empty company
let result = orchestrator.run_workflow(&companies.edge_cases.empty, None);
assert_eq!(result.unwrap_err(), WorkflowError::EmptyCompanyName);

// Test SQL injection attempt (should be sanitized)
let result = orchestrator.run_workflow(&companies.injection_attempts.sql_injection, None);
assert!(result.is_ok()); // Should succeed after sanitization
```

---

### 3. Test Prompts

**Location**: `tests/fixtures/prompts/test_prompts.json`

```json
{
  "valid_templates": {
    "basic": "Analyze {{company}} and provide insights.",
    "with_tools": "Research {{company}} using {{tool_results}}.",
    "multi_variable": "Analyze {{company}} in {{industry}} sector with context: {{context}}."
  },
  "invalid_templates": {
    "missing_variable": "Analyze {{company}} using {{undefined_var}}.",
    "malformed_syntax": "Analyze {{company} using tools.",
    "empty": ""
  },
  "edge_cases": {
    "very_long": "Analyze {{company}} and " + "provide detailed insights. ".repeat(100),
    "special_chars": "Analyze {{company}} with criteria: !@#$%^&*()[]{}",
    "nested_braces": "Analyze {{company}} with {{nested {{inner}} variable}}"
  }
}
```

**Usage**:
```rust
let prompts: TestPrompts = load_fixture("tests/fixtures/prompts/test_prompts.json");

// Test valid template
let phase = Phase {
    prompt_template: prompts.valid_templates.basic,
    ...
};
let result = orchestrator.execute_phase(&phase, None);
assert!(result.is_ok());

// Test invalid template
let phase = Phase {
    prompt_template: prompts.invalid_templates.missing_variable,
    ...
};
let result = orchestrator.execute_phase(&phase, None);
assert_eq!(result.unwrap_err(), PhaseError::TemplateRenderingFailed);
```

---

### 4. Test Responses

**Location**: `tests/fixtures/responses/test_responses.json`

```json
{
  "research_responses": {
    "high_quality": {
      "content": "Acme Corporation is a leading software company founded in 2010...\n\n**Key Insights:**\n- Revenue: $500M (2023)\n- Employees: 2,000+\n- Notable clients: Fortune 500 companies\n\n**Case Study:** Acme's cloud platform reduced client costs by 40%...",
      "model": "claude-3-5-sonnet-20241022",
      "token_usage": { "input_tokens": 150, "output_tokens": 300, "total_tokens": 450 },
      "cost": 0.075
    },
    "low_quality": {
      "content": "Acme is a company. They do software. They are good.",
      "model": "claude-3-5-sonnet-20241022",
      "token_usage": { "input_tokens": 100, "output_tokens": 50, "total_tokens": 150 },
      "cost": 0.025
    },
    "empty": {
      "content": "",
      "model": "claude-3-5-sonnet-20241022",
      "token_usage": { "input_tokens": 50, "output_tokens": 0, "total_tokens": 50 },
      "cost": 0.01
    }
  },
  "error_responses": {
    "rate_limit": {
      "error_type": "RateLimitError",
      "message": "Rate limit exceeded. Retry after 60 seconds.",
      "retry_after": 60
    },
    "timeout": {
      "error_type": "TimeoutError",
      "message": "Request timeout after 30 seconds.",
      "duration_ms": 30000
    },
    "api_error": {
      "error_type": "ApiError",
      "message": "Invalid API key provided.",
      "status_code": 401
    }
  }
}
```

**Usage**:
```rust
let responses: TestResponses = load_fixture("tests/fixtures/responses/test_responses.json");

// Configure mock client with responses
let mock_client = MockLLMClient::new()
    .with_response(
        "Analyze Acme Corp",
        responses.research_responses.high_quality.clone()
    )
    .with_failure(LLMError::from(&responses.error_responses.rate_limit));

// Execute test
let result = orchestrator.generate_llm_response(&phase, &tool_results);
assert!(result.is_ok());
assert_eq!(result.unwrap().content, responses.research_responses.high_quality.content);
```

---

## Test Utilities

### 1. Assertion Helpers

**Location**: `tests/common/assertions.rs`

```rust
/// Assert that a test validated specific IM codes
pub fn assert_im_codes_validated(
    test_result: &TestResult,
    expected_codes: &[&str]
) -> Result<(), AssertionError> {
    let validated_codes = test_result.get_validated_im_codes();

    for expected in expected_codes {
        if !validated_codes.contains(&expected.to_string()) {
            return Err(AssertionError::MissingIMCode {
                expected: expected.to_string(),
                validated: validated_codes.clone(),
            });
        }
    }

    Ok(())
}

/// Assert that output passes quality gates with minimum score
pub fn assert_quality_score(
    validation_result: &ValidationResult,
    min_score: u8
) -> Result<(), AssertionError> {
    if validation_result.score < min_score {
        return Err(AssertionError::QualityScoreTooLow {
            actual: validation_result.score,
            expected: min_score,
            failures: validation_result.failures.clone(),
        });
    }

    Ok(())
}

/// Assert that error matches expected type
pub fn assert_error_type<E: Error + PartialEq>(
    result: Result<(), E>,
    expected_error: E
) -> Result<(), AssertionError> {
    match result {
        Err(actual_error) if actual_error == expected_error => Ok(()),
        Err(actual_error) => Err(AssertionError::WrongErrorType {
            expected: format!("{:?}", expected_error),
            actual: format!("{:?}", actual_error),
        }),
        Ok(_) => Err(AssertionError::ExpectedError {
            expected: format!("{:?}", expected_error),
        }),
    }
}

/// Assert that workflow executed specific phases
pub fn assert_phases_executed(
    workflow_result: &WorkflowResult,
    expected_phases: &[&str]
) -> Result<(), AssertionError> {
    let executed_phases: Vec<String> = workflow_result
        .phase_results
        .keys()
        .cloned()
        .collect();

    for expected in expected_phases {
        if !executed_phases.contains(&expected.to_string()) {
            return Err(AssertionError::PhaseNotExecuted {
                expected: expected.to_string(),
                executed: executed_phases.clone(),
            });
        }
    }

    Ok(())
}

/// Assert that state was persisted correctly
pub fn assert_state_persisted(
    state_manager: &MockStateManager,
    session_id: &str,
    expected_context_keys: &[&str]
) -> Result<(), AssertionError> {
    let context = state_manager
        .load_context(session_id)
        .map_err(|e| AssertionError::StateLoadFailed(e.to_string()))?;

    for key in expected_context_keys {
        if !context.contains_key(*key) {
            return Err(AssertionError::MissingContextKey {
                expected: key.to_string(),
                context: context.keys().cloned().collect(),
            });
        }
    }

    Ok(())
}
```

**Usage Examples**:
```rust
#[test]
fn test_agent_orchestrator_constructor() {
    let result = AgentOrchestrator::new(manifest_path, llm_client, state_manager);
    assert!(result.is_ok());

    // Verify IM codes validated
    assert_im_codes_validated(&test_result, &[
        "IM-2001", "IM-2001-F1", "IM-2001-F2", "IM-2002",
    ]).unwrap();
}

#[test]
fn test_quality_gates_validation() {
    let result = orchestrator.validate_output(&phase, &output);

    // Verify quality score
    assert_quality_score(&result.unwrap(), 99).unwrap();
}

#[test]
fn test_empty_company_error() {
    let result = orchestrator.run_workflow("", None);

    // Verify error type
    assert_error_type(result, WorkflowError::EmptyCompanyName).unwrap();
}
```

---

### 2. Test Data Builders

**Location**: `tests/common/builders.rs`

```rust
/// Builder for AgentOrchestrator with sensible defaults
pub struct TestOrchestratorBuilder {
    manifest_path: Option<String>,
    llm_client: Option<Box<dyn LLMProvider>>,
    state_manager: Option<Arc<dyn StateManager>>,
}

impl TestOrchestratorBuilder {
    pub fn new() -> Self {
        Self {
            manifest_path: None,
            llm_client: None,
            state_manager: None,
        }
    }

    pub fn with_manifest(mut self, path: &str) -> Self {
        self.manifest_path = Some(path.to_string());
        self
    }

    pub fn with_mock_llm(mut self) -> Self {
        self.llm_client = Some(Box::new(MockLLMClient::new()));
        self
    }

    pub fn with_mock_state(mut self) -> Self {
        self.state_manager = Some(Arc::new(MockStateManager::new()));
        self
    }

    pub fn build(self) -> Result<AgentOrchestrator, BuilderError> {
        let manifest_path = self.manifest_path
            .unwrap_or_else(|| "tests/fixtures/manifests/valid_manifest.yaml".to_string());
        let llm_client = self.llm_client
            .unwrap_or_else(|| Box::new(MockLLMClient::new()));
        let state_manager = self.state_manager
            .unwrap_or_else(|| Arc::new(MockStateManager::new()));

        AgentOrchestrator::new(
            &manifest_path,
            llm_client,
            state_manager
        ).map_err(|e| BuilderError::ConstructionFailed(e.to_string()))
    }
}

/// Builder for LLMRequest
pub struct TestLLMRequestBuilder {
    model: String,
    prompt: String,
    temperature: f32,
    max_tokens: usize,
}

impl TestLLMRequestBuilder {
    pub fn new() -> Self {
        Self {
            model: "claude-3-5-sonnet-20241022".to_string(),
            prompt: "Test prompt".to_string(),
            temperature: 0.7,
            max_tokens: 1000,
        }
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn with_prompt(mut self, prompt: &str) -> Self {
        self.prompt = prompt.to_string();
        self
    }

    pub fn build(self) -> LLMRequest {
        LLMRequest {
            model: self.model,
            prompt: self.prompt,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

/// Builder for Phase
pub struct TestPhaseBuilder {
    name: String,
    description: String,
    dependencies: Vec<String>,
    tools: Vec<String>,
    prompt_template: String,
    quality_gates: Vec<String>,
}

impl TestPhaseBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: format!("{} phase", name),
            dependencies: vec![],
            tools: vec![],
            prompt_template: "Default prompt template".to_string(),
            quality_gates: vec!["NoGenericText".to_string()],
        }
    }

    pub fn with_dependencies(mut self, deps: Vec<&str>) -> Self {
        self.dependencies = deps.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_tools(mut self, tools: Vec<&str>) -> Self {
        self.tools = tools.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn with_prompt(mut self, template: &str) -> Self {
        self.prompt_template = template.to_string();
        self
    }

    pub fn build(self) -> Phase {
        Phase {
            name: self.name,
            description: self.description,
            dependencies: self.dependencies,
            tools: self.tools,
            prompt_template: self.prompt_template,
            quality_gates: self.quality_gates,
        }
    }
}
```

**Usage Examples**:
```rust
#[test]
fn test_with_builder() {
    // Build orchestrator with defaults
    let orchestrator = TestOrchestratorBuilder::new()
        .with_mock_llm()
        .with_mock_state()
        .build()
        .unwrap();

    // Build custom phase
    let phase = TestPhaseBuilder::new("research")
        .with_dependencies(vec!["company_analysis"])
        .with_tools(vec!["web_search", "scrape_website"])
        .with_prompt("Research {{company}} using {{tool_results}}.")
        .build();

    // Build LLM request
    let request = TestLLMRequestBuilder::new()
        .with_model("gemini-2.0-flash-exp")
        .with_prompt("Custom prompt")
        .build();

    // Execute test
    let result = orchestrator.execute_phase(&phase, None);
    assert!(result.is_ok());
}
```

---

### 3. Fixture Loaders

**Location**: `tests/common/fixtures.rs`

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Load JSON fixture from file
pub fn load_fixture<T>(path: &str) -> T
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read fixture: {}", path));

    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse fixture {}: {}", path, e))
}

/// Load YAML fixture from file
pub fn load_yaml_fixture<T>(path: &str) -> T
where
    T: for<'de> Deserialize<'de>,
{
    let content = fs::read_to_string(path)
        .unwrap_or_else(|_| panic!("Failed to read YAML fixture: {}", path));

    serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse YAML fixture {}: {}", path, e))
}

/// Test companies fixture
#[derive(Debug, Deserialize)]
pub struct TestCompanies {
    pub valid: Vec<String>,
    pub edge_cases: EdgeCaseCompanies,
    pub injection_attempts: InjectionAttempts,
}

#[derive(Debug, Deserialize)]
pub struct EdgeCaseCompanies {
    pub empty: String,
    pub whitespace_only: String,
    pub single_char: String,
    pub very_long: String,
    pub special_chars: String,
    pub unicode: String,
    pub with_newlines: String,
}

#[derive(Debug, Deserialize)]
pub struct InjectionAttempts {
    pub sql_injection: String,
    pub xss: String,
    pub command_injection: String,
}

/// Test prompts fixture
#[derive(Debug, Deserialize)]
pub struct TestPrompts {
    pub valid_templates: ValidTemplates,
    pub invalid_templates: InvalidTemplates,
    pub edge_cases: EdgeCasePrompts,
}

#[derive(Debug, Deserialize)]
pub struct ValidTemplates {
    pub basic: String,
    pub with_tools: String,
    pub multi_variable: String,
}

#[derive(Debug, Deserialize)]
pub struct InvalidTemplates {
    pub missing_variable: String,
    pub malformed_syntax: String,
    pub empty: String,
}

#[derive(Debug, Deserialize)]
pub struct EdgeCasePrompts {
    pub very_long: String,
    pub special_chars: String,
    pub nested_braces: String,
}

/// Test responses fixture
#[derive(Debug, Deserialize, Clone)]
pub struct TestResponses {
    pub research_responses: ResearchResponses,
    pub error_responses: ErrorResponses,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ResearchResponses {
    pub high_quality: LLMResponse,
    pub low_quality: LLMResponse,
    pub empty: LLMResponse,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ErrorResponses {
    pub rate_limit: ErrorResponse,
    pub timeout: ErrorResponse,
    pub api_error: ErrorResponse,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ErrorResponse {
    pub error_type: String,
    pub message: String,
    #[serde(default)]
    pub retry_after: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub status_code: Option<u16>,
}
```

**Usage**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_fixtures() {
        // Load fixtures
        let companies: TestCompanies = load_fixture("tests/fixtures/companies/test_companies.json");
        let prompts: TestPrompts = load_fixture("tests/fixtures/prompts/test_prompts.json");
        let responses: TestResponses = load_fixture("tests/fixtures/responses/test_responses.json");

        // Use in tests
        let result = orchestrator.run_workflow(&companies.valid[0], None);
        assert!(result.is_ok());
    }
}
```

---

## Infrastructure Summary

### Components Created

| Component | Purpose | IM Codes Enabled |
|-----------|---------|------------------|
| **MockLLMClient** | Simulate LLM APIs | IM-2014, IM-3012, all LLM errors |
| **MockStateManager** | Simulate database | IM-2010, IM-5xxx, all state errors |
| **MockQualityGates** | Simulate validation | IM-2015, IM-4010, quality errors |
| **MockUIWindow** | Simulate Tauri events | IM-2020, UI codes |
| **Test Manifests** | YAML configs | IM-2002 (constructor) |
| **Test Companies** | Company data | IM-2010 (workflow) |
| **Test Prompts** | Prompt templates | IM-2014 (LLM generation) |
| **Test Responses** | LLM responses | IM-3012 (LLMClient) |
| **Assertion Helpers** | Verify IM codes | All tests |
| **Data Builders** | Construct test data | All tests |
| **Fixture Loaders** | Load test data | All tests |

### Benefits

1. **Isolation**: Tests run without external dependencies (no DB, no API calls)
2. **Speed**: In-memory mocks execute in milliseconds
3. **Determinism**: Predictable results, no flaky tests
4. **Error Injection**: Easy to test error paths
5. **IM Code Tracking**: Assertion helpers verify coverage

### Next Steps

**Phase Completion**: TEST-PRE-CODE (Step 3) → COMPLETE

**Ready for**: Step 4 - Write Complete Test Specifications (all 7 sections per test)

**Status**: Infrastructure plan complete with 4 mocks, 4 fixture types, and 3 utility categories. All dependencies resolved for battery test implementation.

---

**END OF DOCUMENT**
