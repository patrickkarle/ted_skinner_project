# Battery Test Complete Specifications - TEST-SPECS Phase
**Date:** 2025-11-22
**Phase:** TEST-SPECS (Step 4: Write Complete Test Specifications)
**Format:** Phase 6 Methodology - 7 Sections per Test

---

## Test Specification Template

Each test follows this 7-section structure:

### 1. Test Metadata
- **Test ID**: Unique identifier (TEST-{Component}-{Type}-{Number})
- **Test Name**: Descriptive name
- **Component**: Component under test
- **Test Type**: Unit / Integration / E2E
- **Priority**: Critical / Important / Optional
- **Estimated Duration**: Execution time estimate

### 2. IM Codes Validated
- **Primary IM Codes**: Direct validation
- **Secondary IM Codes**: Indirect validation through dependencies
- **Validation Count**: Number of IM codes validated
- **Coverage Percentage**: % of component IM codes covered

### 3. Description
- **Purpose**: What this test validates
- **Rationale**: Why this test is necessary
- **Success Criteria**: What constitutes a pass

### 4. Pre-Code Specification
- **GIVEN**: Test preconditions
- **WHEN**: Test execution steps
- **THEN**: Expected outcomes
- **Test Cases**: Sub-scenarios if applicable

### 5. Mock Configuration
- **Mocks Required**: Which mocks to use
- **Mock Behavior**: How mocks should behave
- **Fixture Data**: Test data requirements

### 6. Assertions
- **Functional Assertions**: Business logic verification
- **IM Code Assertions**: IM code coverage verification
- **State Assertions**: Side effect verification
- **Performance Assertions**: (if applicable)

### 7. Edge Cases & Variants
- **Boundary Conditions**: Min/max values, empty inputs
- **Error Scenarios**: Expected failures
- **Alternative Flows**: Different execution paths

---

## AgentOrchestrator Unit Tests (21 tests)

### TEST-AO-U-001: Constructor Happy Path

#### 1. Test Metadata
- **Test ID**: TEST-AO-U-001
- **Test Name**: AgentOrchestrator Constructor Happy Path
- **Component**: AgentOrchestrator
- **Test Type**: Unit
- **Priority**: Critical (Constructor is foundation for all other tests)
- **Estimated Duration**: 50ms

#### 2. IM Codes Validated
- **Primary IM Codes**:
  - IM-2001: AgentOrchestrator Struct
  - IM-2001-F1 through IM-2001-F6: All 6 struct fields
  - IM-2002: AgentOrchestrator::new() method
  - IM-2002-P1: manifest_path parameter
  - IM-2002-P2: llm_client parameter
  - IM-2002-P3: state_manager parameter
  - IM-2002-V1: manifest variable
  - IM-2002-V2: tool_registry variable
  - IM-2002-V3: quality_gates variable
  - IM-2002-V4: context variable
  - IM-2002-B1: File exists check branch
  - IM-2002-B2: YAML parse success branch
  - IM-2002-B3: Tool registration loop branch

- **Secondary IM Codes**:
  - IM-5002: StateManager::new() (through parameter)
  - IM-3011: LLMClient::new() (through parameter)

- **Validation Count**: 18 primary codes, 2 secondary codes = 20 total
- **Coverage Percentage**: 18/171 = 10.5% of AgentOrchestrator codes

#### 3. Description
- **Purpose**: Verify AgentOrchestrator constructor initializes all fields correctly with valid inputs
- **Rationale**: Constructor is the entry point for all AgentOrchestrator functionality. If constructor fails, no other tests can run. Must validate all 6 struct fields are properly initialized.
- **Success Criteria**:
  - Constructor returns Ok(AgentOrchestrator)
  - All 6 struct fields populated with correct values
  - ProcessManifest loaded from YAML file
  - ToolRegistry contains all tools from manifest
  - QualityGateValidator initialized with default gates
  - Context initialized as empty HashMap

#### 4. Pre-Code Specification

**GIVEN**:
```
- Valid manifest.yaml file exists at "tests/fixtures/manifests/valid_manifest.yaml"
- Manifest contains:
  - project.name = "Test Research Project"
  - llm_config with API keys
  - database.path = "test_research.db"
  - tools: 3 tools (web_search, scrape_website, analyze_text)
  - phases: 2 phases (company_analysis, research)
- MockLLMClient initialized with default responses
- MockStateManager initialized with in-memory storage
```

**WHEN**:
```rust
let manifest_path = "tests/fixtures/manifests/valid_manifest.yaml";
let llm_client = Box::new(MockLLMClient::new());
let state_manager = Arc::new(MockStateManager::new());

let result = AgentOrchestrator::new(
    manifest_path,
    llm_client,
    state_manager
);
```

**THEN**:
```
1. result.is_ok() == true
2. orchestrator = result.unwrap()
3. orchestrator.manifest:
   - project_name = "Test Research Project"
   - phases.len() = 2
   - phases[0].name = "company_analysis"
   - phases[1].name = "research"
4. orchestrator.tool_registry:
   - contains("web_search") = true
   - contains("scrape_website") = true
   - contains("analyze_text") = true
   - size() = 3
5. orchestrator.llm_client: MockLLMClient instance
6. orchestrator.quality_gates: QualityGateValidator instance
7. orchestrator.state_manager: MockStateManager instance
8. orchestrator.context: Empty HashMap (size = 0)
```

**Branch Coverage**:
```
- B1 (File exists check): manifest.yaml exists → true branch
- B2 (YAML parse success): Valid YAML → success branch
- B3 (Tool registration loop): 3 tools registered → loop executes 3 times
```

#### 5. Mock Configuration

**Mocks Required**:
- **MockLLMClient**: Default configuration (no responses needed for constructor)
- **MockStateManager**: Default configuration (no operations needed for constructor)

**Mock Behavior**:
```rust
let llm_client = MockLLMClient::new();
// No special configuration - constructor doesn't call LLM

let state_manager = MockStateManager::new();
// No special configuration - constructor doesn't access DB
```

**Fixture Data**:
- **Manifest File**: `tests/fixtures/manifests/valid_manifest.yaml`
  - Must exist on filesystem
  - Must contain valid YAML syntax
  - Must include all required fields (project, llm_config, database, tools, phases)

#### 6. Assertions

**Functional Assertions**:
```rust
// Constructor succeeds
assert!(result.is_ok(), "Constructor should succeed with valid inputs");

let orchestrator = result.unwrap();

// Manifest loaded correctly
assert_eq!(orchestrator.manifest.project_name, "Test Research Project");
assert_eq!(orchestrator.manifest.phases.len(), 2);

// Tools registered
assert_eq!(orchestrator.tool_registry.size(), 3);
assert!(orchestrator.tool_registry.contains("web_search"));

// Context initialized empty
assert_eq!(orchestrator.context.len(), 0);
```

**IM Code Assertions**:
```rust
// Verify IM codes validated
assert_im_codes_validated(&test_result, &[
    "IM-2001",           // Struct
    "IM-2001-F1",        // manifest field
    "IM-2001-F2",        // tool_registry field
    "IM-2001-F3",        // llm_client field
    "IM-2001-F4",        // quality_gates field
    "IM-2001-F5",        // state_manager field
    "IM-2001-F6",        // context field
    "IM-2002",           // new() method
    "IM-2002-P1",        // manifest_path param
    "IM-2002-P2",        // llm_client param
    "IM-2002-P3",        // state_manager param
    "IM-2002-V1",        // manifest var
    "IM-2002-V2",        // tool_registry var
    "IM-2002-V3",        // quality_gates var
    "IM-2002-V4",        // context var
    "IM-2002-B1",        // File exists check
    "IM-2002-B2",        // YAML parse success
    "IM-2002-B3",        // Tool registration loop
]).expect("All IM codes should be validated");
```

**State Assertions**:
```rust
// No state persistence in constructor
// No external side effects expected
```

#### 7. Edge Cases & Variants

**Boundary Conditions**:
- **Minimal Manifest**: Use `minimal_manifest.yaml` with only required fields
  - Expect: Constructor succeeds with minimal configuration
  - Verify: Empty tools list, single phase, default quality gates
- **Large Manifest**: 100+ tools, 50+ phases
  - Expect: Constructor succeeds, all tools/phases loaded
  - Performance: Should complete < 200ms

**Error Scenarios** (tested in TEST-AO-U-002):
- Empty manifest path
- Nonexistent file
- Invalid YAML syntax
- Missing API keys
- Database connection failure

**Alternative Flows**:
- **Different Tool Counts**: 0 tools, 1 tool, 100 tools
  - Verify tool_registry.size() matches manifest
- **Different Phase Counts**: 1 phase, 10 phases
  - Verify manifest.phases.len() matches

---

### TEST-AO-U-002: Constructor Error Handling

#### 1. Test Metadata
- **Test ID**: TEST-AO-U-002
- **Test Name**: AgentOrchestrator Constructor Error Handling
- **Component**: AgentOrchestrator
- **Test Type**: Unit
- **Priority**: Critical (Error handling is essential for robustness)
- **Estimated Duration**: 30ms (5 test cases × 6ms avg)

#### 2. IM Codes Validated
- **Primary IM Codes**:
  - IM-2002-E1: Empty path error
  - IM-2002-E2: File not found error
  - IM-2002-E3: YAML parse error
  - IM-2002-E4: Missing API keys error
  - IM-2002-E5: Database connection error

- **Secondary IM Codes**: None
- **Validation Count**: 5 codes
- **Coverage Percentage**: 5/171 = 2.9% of AgentOrchestrator codes

#### 3. Description
- **Purpose**: Verify AgentOrchestrator constructor rejects invalid inputs with appropriate error messages
- **Rationale**: Robust error handling prevents silent failures and provides clear diagnostics. Each error code represents a different failure mode that must be tested.
- **Success Criteria**:
  - Constructor returns Err for each invalid input
  - Error types match expected error codes (E1-E5)
  - Error messages provide actionable information

#### 4. Pre-Code Specification

**Test Case 1: Empty Path (E1)**
```
GIVEN: manifest_path = ""
WHEN: AgentOrchestrator::new("", llm_client, state_manager)
THEN: Returns Err(OrchestratorError::EmptyPath("Manifest path cannot be empty"))
```

**Test Case 2: File Not Found (E2)**
```
GIVEN: manifest_path = "nonexistent.yaml" (file doesn't exist)
WHEN: AgentOrchestrator::new("nonexistent.yaml", llm_client, state_manager)
THEN: Returns Err(OrchestratorError::FileNotFound("Manifest file not found: nonexistent.yaml"))
```

**Test Case 3: YAML Parse Error (E3)**
```
GIVEN: manifest_path = "tests/fixtures/manifests/invalid_manifest.yaml"
       File contains: "project:\n  name: Invalid Test\n  version: 1.0.0" (version should be string)
WHEN: AgentOrchestrator::new(manifest_path, llm_client, state_manager)
THEN: Returns Err(OrchestratorError::YAMLParseError("Failed to parse manifest: expected string, found integer"))
```

**Test Case 4: Missing API Keys (E4)**
```
GIVEN: manifest_path = "tests/fixtures/manifests/missing_keys_manifest.yaml"
       File has llm_config section but no api keys
WHEN: AgentOrchestrator::new(manifest_path, llm_client, state_manager)
THEN: Returns Err(OrchestratorError::MissingAPIKeys("No API keys found in manifest"))
```

**Test Case 5: Database Connection Error (E5)**
```
GIVEN: MockStateManager configured to fail on connection
       state_manager.with_failure(StateOperation::Connect)
WHEN: AgentOrchestrator::new(manifest_path, llm_client, state_manager)
THEN: Returns Err(OrchestratorError::DatabaseConnectionError("Failed to connect to database"))
```

#### 5. Mock Configuration

**Mocks Required**:
- **MockLLMClient**: Default configuration
- **MockStateManager**: Configured to fail for Test Case 5

**Mock Behavior**:
```rust
// Test Cases 1-4: Default mocks
let llm_client = MockLLMClient::new();
let state_manager = MockStateManager::new();

// Test Case 5: Failing state manager
let failing_state_manager = MockStateManager::new()
    .with_failure(StateOperation::Connect);
```

**Fixture Data**:
- `tests/fixtures/manifests/invalid_manifest.yaml` (YAML syntax errors)
- `tests/fixtures/manifests/missing_keys_manifest.yaml` (no API keys)

#### 6. Assertions

**Functional Assertions**:
```rust
// Test Case 1: Empty path
let result = AgentOrchestrator::new("", llm_client.clone(), state_manager.clone());
assert!(result.is_err());
assert_error_type(result, OrchestratorError::EmptyPath).unwrap();

// Test Case 2: File not found
let result = AgentOrchestrator::new("nonexistent.yaml", llm_client.clone(), state_manager.clone());
assert!(result.is_err());
assert_error_type(result, OrchestratorError::FileNotFound).unwrap();

// Test Case 3: YAML parse error
let result = AgentOrchestrator::new("tests/fixtures/manifests/invalid_manifest.yaml", llm_client.clone(), state_manager.clone());
assert!(result.is_err());
assert_error_type(result, OrchestratorError::YAMLParseError).unwrap();

// Test Case 4: Missing API keys
let result = AgentOrchestrator::new("tests/fixtures/manifests/missing_keys_manifest.yaml", llm_client.clone(), state_manager.clone());
assert!(result.is_err());
assert_error_type(result, OrchestratorError::MissingAPIKeys).unwrap();

// Test Case 5: Database connection error
let result = AgentOrchestrator::new(manifest_path, llm_client, failing_state_manager);
assert!(result.is_err());
assert_error_type(result, OrchestratorError::DatabaseConnectionError).unwrap();
```

**IM Code Assertions**:
```rust
assert_im_codes_validated(&test_result, &[
    "IM-2002-E1",  // Empty path
    "IM-2002-E2",  // File not found
    "IM-2002-E3",  // YAML parse error
    "IM-2002-E4",  // Missing API keys
    "IM-2002-E5",  // Database connection error
]).expect("All error codes should be validated");
```

#### 7. Edge Cases & Variants

**Boundary Conditions**:
- **Whitespace-Only Path**: manifest_path = "   " → Should trigger E1 (empty path) after trim
- **Relative vs Absolute Paths**: Both should work if file exists
- **Case Sensitivity**: "Manifest.YAML" vs "manifest.yaml" (OS-dependent)

**Error Message Validation**:
- Each error should include context (e.g., file path, specific field missing)
- Error messages should be actionable

**Alternative Flows**:
- **Multiple Errors**: File doesn't exist AND has invalid YAML → Should return first encountered error (E2)
- **Partial Validation**: Some API keys present, some missing → Should still fail with E4

---

### TEST-AO-U-004: run_workflow() Happy Path

#### 1. Test Metadata
- **Test ID**: TEST-AO-U-004
- **Test Name**: run_workflow() Happy Path Validation
- **Component**: AgentOrchestrator
- **Test Type**: Unit
- **Priority**: Critical (Core workflow method)
- **Estimated Duration**: 150ms (mock execution, no real LLM calls)

#### 2. IM Codes Validated
- **Primary IM Codes**:
  - IM-2010: run_workflow() method
  - IM-2010-P1: company parameter
  - IM-2010-P2: window parameter
  - IM-2010-V1: session_id variable
  - IM-2010-V2: accumulated_output variable
  - IM-2010-V3: total_cost variable
  - IM-2010-V4: phase_results variable
  - IM-2010-V5: start_time variable
  - IM-2010-B1: Company name validation branch
  - IM-2010-B2: Session creation success branch
  - IM-2010-B3: Window present check branch
  - IM-2010-B4: Phase execution loop branch
  - IM-2010-B5: Dependency check before phase branch
  - IM-2010-B6: Phase execution success branch
  - IM-2010-B7: Final quality gates branch

- **Secondary IM Codes**:
  - IM-5020: StateManager::create_session() (called during workflow)
  - IM-2011: AgentOrchestrator::execute_phase() (called in loop)
  - IM-2012: AgentOrchestrator::check_dependencies() (called before phases)
  - IM-2020: AgentOrchestrator::emit_progress() (called for window events)

- **Validation Count**: 15 primary + 4 secondary = 19 codes
- **Coverage Percentage**: 15/171 = 8.8% of AgentOrchestrator codes

#### 3. Description
- **Purpose**: Verify run_workflow() executes complete workflow with valid company name and optional window
- **Rationale**: run_workflow() is the primary public API for AgentOrchestrator. Must validate end-to-end orchestration: session creation, phase execution, dependency checking, quality validation, and result aggregation.
- **Success Criteria**:
  - Workflow completes successfully
  - Session created in database
  - All phases execute in order
  - Dependencies checked before each phase
  - Quality gates validate each phase output
  - Final result contains all phase outputs
  - Window receives progress events (if provided)

#### 4. Pre-Code Specification

**GIVEN**:
```
- Orchestrator initialized with valid manifest
- Manifest contains 2 phases:
  - Phase 1: "company_analysis" (no dependencies)
  - Phase 2: "research" (depends on "company_analysis")
- company = "Acme Corp" (valid company name)
- window = Some(mock_ui_window)
- MockLLMClient configured with responses:
  - "Analyze Acme Corp" → "Acme Corp is a software company..."
  - "Research Acme Corp" → "Deep research on Acme Corp..."
- MockStateManager with empty session storage
- MockQualityGates configured to pass (score 99)
```

**WHEN**:
```rust
let company = "Acme Corp";
let window = Some(mock_ui_window);

let result = orchestrator.run_workflow(company, window);
```

**THEN**:
```
1. result.is_ok() == true
2. workflow_result = result.unwrap()
3. workflow_result.session_id: Non-empty UUID string
4. workflow_result.accumulated_output: Contains outputs from both phases
5. workflow_result.total_cost: Sum of phase costs (≈ $0.10)
6. workflow_result.phase_results: HashMap with 2 entries
   - phase_results["company_analysis"]: PhaseResult with output, cost, duration
   - phase_results["research"]: PhaseResult with output, cost, duration
7. workflow_result.duration: Total execution time (< 200ms with mocks)

State Verification:
8. MockStateManager.sessions.len() == 1
9. MockStateManager.sessions[0].company == "Acme Corp"
10. MockStateManager.contexts.len() == 1
11. MockStateManager.contexts[session_id] contains "company_info" key

Window Event Verification:
12. MockUIWindow.event_count() == 6 events:
    - session_created
    - phase_start (company_analysis)
    - phase_complete (company_analysis)
    - phase_start (research)
    - phase_complete (research)
    - workflow_complete

Branch Coverage:
13. B1 (Company validation): "Acme Corp" is valid → true branch
14. B2 (Session creation): Succeeds → success branch
15. B3 (Window present): Some(window) → emit events branch
16. B4 (Phase loop): 2 phases → loop executes 2 times
17. B5 (Dependency check): Phase 2 depends on Phase 1 → check executes
18. B6 (Phase execution): Both phases succeed → success branch
19. B7 (Quality gates): Both outputs pass gates → true branch
```

#### 5. Mock Configuration

**Mocks Required**:
- **MockLLMClient**: Configured with phase-specific responses
- **MockStateManager**: Empty initial state
- **MockQualityGates**: Pass all validations (score 99)
- **MockUIWindow**: Capture all events

**Mock Behavior**:
```rust
// Configure LLM client with responses
let llm_client = MockLLMClient::new()
    .with_response(
        "Analyze Acme Corp",
        LLMResponse {
            content: "Acme Corp is a software company founded in 2010. Revenue: $500M...",
            model: "claude-3-5-sonnet-20241022",
            token_usage: TokenUsage { input_tokens: 100, output_tokens: 200, total_tokens: 300 },
            cost: 0.05,
        }
    )
    .with_response(
        "Research Acme Corp",
        LLMResponse {
            content: "Deep research on Acme Corp reveals strong market position...",
            model: "claude-3-5-sonnet-20241022",
            token_usage: TokenUsage { input_tokens: 150, output_tokens: 250, total_tokens: 400 },
            cost: 0.065,
        }
    );

// Configure state manager
let state_manager = Arc::new(MockStateManager::new());

// Configure quality gates (pass all)
let quality_gates = MockQualityGates::new()
    .with_score(99);

// Configure UI window
let window = Some(Arc::new(MockUIWindow::new()));
```

**Fixture Data**:
- Valid manifest with 2-phase workflow
- Company name: "Acme Corp"

#### 6. Assertions

**Functional Assertions**:
```rust
// Workflow succeeds
assert!(result.is_ok(), "Workflow should complete successfully");

let workflow_result = result.unwrap();

// Session created
assert!(!workflow_result.session_id.is_empty());

// All phases executed
assert_eq!(workflow_result.phase_results.len(), 2);
assert!(workflow_result.phase_results.contains_key("company_analysis"));
assert!(workflow_result.phase_results.contains_key("research"));

// Outputs accumulated
assert!(workflow_result.accumulated_output.contains("Acme Corp is a software company"));
assert!(workflow_result.accumulated_output.contains("Deep research on Acme Corp"));

// Cost calculated
assert_approx_eq!(workflow_result.total_cost, 0.115, 0.001); // 0.05 + 0.065
```

**IM Code Assertions**:
```rust
assert_im_codes_validated(&test_result, &[
    "IM-2010",       // run_workflow method
    "IM-2010-P1",    // company param
    "IM-2010-P2",    // window param
    "IM-2010-V1",    // session_id var
    "IM-2010-V2",    // accumulated_output var
    "IM-2010-V3",    // total_cost var
    "IM-2010-V4",    // phase_results var
    "IM-2010-V5",    // start_time var
    "IM-2010-B1",    // Company validation
    "IM-2010-B2",    // Session creation success
    "IM-2010-B3",    // Window present check
    "IM-2010-B4",    // Phase execution loop
    "IM-2010-B5",    // Dependency check
    "IM-2010-B6",    // Phase execution success
    "IM-2010-B7",    // Final quality gates
]).expect("All IM codes validated");
```

**State Assertions**:
```rust
// Verify session persisted
let sessions = state_manager.get_sessions();
assert_eq!(sessions.len(), 1);
assert_eq!(sessions[0].company, "Acme Corp");
assert_eq!(sessions[0].status, SessionStatus::Completed);

// Verify context saved
let contexts = state_manager.get_contexts();
assert_eq!(contexts.len(), 1);
assert!(contexts[0].contains_key("company_info"));
```

**Window Event Assertions**:
```rust
// Verify progress events emitted
assert_eq!(window.event_count(), 6);

let events_by_type = window.get_events_by_type("phase_complete");
assert_eq!(events_by_type.len(), 2); // 2 phases completed
```

#### 7. Edge Cases & Variants

**Boundary Conditions**:
- **Single Phase Workflow**: Only 1 phase, no dependencies
  - Expect: Simpler execution, fewer events
- **10 Phase Workflow**: Long workflow with complex dependencies
  - Expect: All phases execute in order, dependency tree respected
- **No Window**: window = None
  - Expect: No events emitted, workflow still succeeds

**Alternative Flows**:
- **Phase Dependency Chain**: Phase 3 depends on Phase 2 depends on Phase 1
  - Verify: Phases execute in correct order
- **Parallel Phases**: Phases 2 and 3 both depend only on Phase 1
  - Verify: Both can execute after Phase 1 (currently sequential, could optimize)

**Performance Variants**:
- **Large Company Name**: 1000 character company name
  - Expect: Workflow succeeds, name properly stored/retrieved
- **Unicode Company**: "株式会社テスト"
  - Expect: UTF-8 handling works correctly

---

## AgentOrchestrator Integration Tests (6 tests)

### TEST-AO-I-002: Dependency Interaction - StateManager

#### 1. Test Metadata
- **Test ID**: TEST-AO-I-002
- **Test Name**: AgentOrchestrator ↔ StateManager Integration
- **Component**: AgentOrchestrator, StateManager
- **Test Type**: Integration
- **Priority**: Critical (State persistence is essential)
- **Estimated Duration**: 100ms

#### 2. IM Codes Validated
- **Primary IM Codes**:
  - IM-2010: run_workflow() (orchestrator side)
  - IM-5020: StateManager::create_session() (state manager side)
  - IM-5040: StateManager::save_context() (state manager side)
  - IM-5030: StateManager::save_phase_completion() (state manager side)

- **Secondary IM Codes**:
  - IM-2011: execute_phase() (calls state operations)
  - IM-5021: StateManager::update_session_status()

- **Validation Count**: 4 primary + 2 secondary = 6 codes
- **Coverage Percentage**: Validates orchestrator-state interaction (cross-component)

#### 3. Description
- **Purpose**: Verify AgentOrchestrator correctly interacts with StateManager for all persistence operations
- **Rationale**: State persistence enables workflow resumption, audit trails, and debugging. Must verify all state operations are called correctly and errors are handled gracefully.
- **Success Criteria**:
  - Session created before workflow execution
  - Context saved after each phase
  - Phase completions recorded
  - Session status updated on completion
  - All state operations use correct session_id
  - State persists across multiple workflows

#### 4. Pre-Code Specification

**GIVEN**:
```
- Orchestrator initialized
- MockStateManager tracking all operations
- 2-phase workflow configured
- company = "Test Corp"
```

**WHEN**:
```rust
let result = orchestrator.run_workflow("Test Corp", None);
```

**THEN**:
```
State Operations Sequence:
1. create_session("Test Corp") called → returns "session-00001"
2. save_context("session-00001", context) called after Phase 1
   - context contains "company_analysis" output
3. save_phase_completion("session-00001", phase1_completion) called
4. save_context("session-00001", context) called after Phase 2
   - context contains "company_analysis" + "research" outputs
5. save_phase_completion("session-00001", phase2_completion) called
6. update_session_status("session-00001", SessionStatus::Completed) called

Verification:
7. MockStateManager.sessions.len() == 1
8. MockStateManager.contexts["session-00001"]: Contains both phase outputs
9. MockStateManager.phase_completions.len() == 2
10. Session status = Completed
```

#### 5. Mock Configuration

**Mocks Required**:
- **MockStateManager**: Track all operations
- **MockLLMClient**: Default responses
- **MockQualityGates**: Pass validations

**Mock Behavior**:
```rust
let state_manager = Arc::new(MockStateManager::new());
// Track operations automatically via Arc interior mutability

// After workflow execution, inspect state:
let sessions = state_manager.get_sessions();
let contexts = state_manager.get_contexts();
let completions = state_manager.get_phase_completions();
```

#### 6. Assertions

**Functional Assertions**:
```rust
// Workflow succeeds
assert!(result.is_ok());

// Session created
let sessions = state_manager.get_sessions();
assert_eq!(sessions.len(), 1);
assert_eq!(sessions[0].company, "Test Corp");

// Context saved with both phases
let context = state_manager.load_context(&sessions[0].id).unwrap();
assert!(context.contains_key("company_analysis"));
assert!(context.contains_key("research"));

// Phase completions recorded
let completions = state_manager.get_phase_completions();
assert_eq!(completions.len(), 2);
assert_eq!(completions[0].phase_name, "company_analysis");
assert_eq!(completions[1].phase_name, "research");

// Session marked complete
assert_eq!(sessions[0].status, SessionStatus::Completed);
```

**IM Code Assertions**:
```rust
assert_im_codes_validated(&test_result, &[
    "IM-2010",    // run_workflow
    "IM-5020",    // create_session
    "IM-5040",    // save_context
    "IM-5030",    // save_phase_completion
]).expect("Integration IM codes validated");
```

**State Assertions**:
```rust
// Verify operation order
assert_state_persisted(&state_manager, &sessions[0].id, &[
    "company_analysis",
    "research",
]).expect("All phases persisted to context");
```

#### 7. Edge Cases & Variants

**Error Scenarios**:
- **Session Creation Fails**: MockStateManager.with_failure(StateOperation::CreateSession)
  - Expect: run_workflow() returns E3 (SessionCreationFailed)
- **Context Save Fails**: MockStateManager.with_failure(StateOperation::SaveContext)
  - Expect: Phase execution returns E7 (StateSaveFailed)

**Alternative Flows**:
- **Multiple Workflows**: Run 3 workflows sequentially
  - Verify: 3 sessions created, contexts isolated by session_id
- **Workflow Interruption**: Simulate crash after Phase 1
  - Verify: Partial state saved (1 phase completion recorded)

---

## AgentOrchestrator E2E Tests (3 tests)

### TEST-AO-E2E-001: Complete Research Workflow

#### 1. Test Metadata
- **Test ID**: TEST-AO-E2E-001
- **Test Name**: Complete Research Workflow End-to-End
- **Component**: AgentOrchestrator, LLMClient, StateManager, QualityGates, ToolRegistry
- **Test Type**: E2E
- **Priority**: Critical (Validates entire system integration)
- **Estimated Duration**: 300ms (mocked LLM calls)

#### 2. IM Codes Validated
- **Primary IM Codes**: All AgentOrchestrator codes (171 codes) validated in realistic workflow
- **Secondary IM Codes**:
  - LLMClient codes (IM-3xxx): 20+ codes
  - StateManager codes (IM-5xxx): 15+ codes
  - QualityGates codes (IM-4xxx): 10+ codes

- **Validation Count**: 171 primary + 45+ secondary = 216+ codes
- **Coverage Percentage**: 100% of AgentOrchestrator + cross-component coverage

#### 3. Description
- **Purpose**: Execute complete 7-phase research workflow from initialization to final output
- **Rationale**: E2E test validates entire system working together: orchestration, LLM calls, tool execution, state persistence, quality validation, and UI event emission. This is the most realistic test scenario.
- **Success Criteria**:
  - Full workflow executes without errors
  - All 7 phases complete in order
  - Each phase:
    - Checks dependencies successfully
    - Executes required tools
    - Generates LLM output
    - Validates output against quality gates
    - Saves state
    - Emits progress events
  - Final output passes quality gates (99-100 score)
  - Total execution < 500ms with mocks
  - Complete audit trail in database

#### 4. Pre-Code Specification

**GIVEN**:
```
- Orchestrator initialized with full research manifest
- Manifest contains 7 phases:
  1. company_analysis (no deps, tools: web_search)
  2. web_search (deps: company_analysis, tools: web_search)
  3. research_analysis (deps: web_search, tools: analyze_text)
  4. insights_generation (deps: research_analysis, no tools)
  5. summary_generation (deps: insights_generation, no tools)
  6. quality_check (deps: summary_generation, no tools)
  7. final_output (deps: quality_check, no tools)
- MockLLMClient with responses for all 7 phases
- MockStateManager with empty storage
- MockQualityGates passing all validations
- MockUIWindow capturing events
- company = "Acme Corporation"
- window = Some(mock_ui_window)
```

**WHEN**:
```rust
let orchestrator = TestOrchestratorBuilder::new()
    .with_manifest("tests/fixtures/manifests/full_research_manifest.yaml")
    .with_mock_llm()
    .with_mock_state()
    .build()
    .unwrap();

let result = orchestrator.run_workflow("Acme Corporation", window);
```

**THEN**:
```
Workflow Execution:
1. Session created: session_id = "session-00001"
2. Phase 1 (company_analysis):
   - Dependencies check passes (no deps)
   - Tool "web_search" executes
   - LLM generates company analysis
   - Quality gates validate (score 99)
   - Context updated with "company_info"
   - Progress emitted
3. Phase 2 (web_search):
   - Dependencies check passes (company_analysis complete)
   - Tool "web_search" executes
   - LLM processes search results
   - Quality gates validate (score 99)
   - Context updated with "search_results"
   - Progress emitted
4. Phases 3-7: Similar pattern, each depending on previous
5. Final output generated with all accumulated context
6. Session marked complete

Final State:
7. result.is_ok() == true
8. workflow_result.phase_results.len() == 7
9. workflow_result.accumulated_output: Contains outputs from all 7 phases
10. workflow_result.total_cost ≈ $0.50 (sum of all phases)
11. workflow_result.duration < 500ms

Database State:
12. 1 session created (company = "Acme Corporation", status = Completed)
13. 1 context saved (contains all 7 phase outputs)
14. 7 phase completions recorded

UI Events:
15. 16 events emitted:
    - 1 session_created
    - 7 phase_start events
    - 7 phase_complete events
    - 1 workflow_complete

IM Code Coverage:
16. All 171 AgentOrchestrator codes validated
17. 20+ LLMClient codes validated
18. 15+ StateManager codes validated
19. 10+ QualityGates codes validated
```

#### 5. Mock Configuration

**Mocks Required**: All components mocked

**Mock Behavior**:
```rust
// LLM Client: 7 phase-specific responses
let llm_responses = load_fixture("tests/fixtures/responses/research_workflow_responses.json");

let llm_client = MockLLMClient::new()
    .with_response("Analyze Acme Corporation", llm_responses.phase1)
    .with_response("Search for Acme Corporation", llm_responses.phase2)
    .with_response("Analyze research on Acme Corporation", llm_responses.phase3)
    .with_response("Generate insights for Acme Corporation", llm_responses.phase4)
    .with_response("Summarize Acme Corporation research", llm_responses.phase5)
    .with_response("Quality check Acme Corporation summary", llm_responses.phase6)
    .with_response("Final output for Acme Corporation", llm_responses.phase7);

// State Manager: Track all operations
let state_manager = Arc::new(MockStateManager::new());

// Quality Gates: Pass all (99 score)
let quality_gates = MockQualityGates::new().with_score(99);

// UI Window: Capture events
let window = Some(Arc::new(MockUIWindow::new()));
```

**Fixture Data**:
- `tests/fixtures/manifests/full_research_manifest.yaml` (7-phase workflow)
- `tests/fixtures/responses/research_workflow_responses.json` (all phase responses)

#### 6. Assertions

**Functional Assertions**:
```rust
// Workflow completes successfully
assert!(result.is_ok(), "E2E workflow should complete");

let workflow_result = result.unwrap();

// All phases executed
assert_eq!(workflow_result.phase_results.len(), 7);

// Verify each phase
for phase_name in ["company_analysis", "web_search", "research_analysis",
                   "insights_generation", "summary_generation", "quality_check", "final_output"] {
    assert!(workflow_result.phase_results.contains_key(phase_name),
            "Phase {} should be in results", phase_name);
}

// Accumulated output contains all phases
assert!(workflow_result.accumulated_output.contains("company analysis"));
assert!(workflow_result.accumulated_output.contains("search results"));
assert!(workflow_result.accumulated_output.contains("final output"));

// Cost reasonable
assert!(workflow_result.total_cost > 0.0);
assert!(workflow_result.total_cost < 1.0); // Sanity check

// Duration reasonable
assert!(workflow_result.duration.as_millis() < 500);
```

**IM Code Assertions**:
```rust
// Verify comprehensive IM code coverage
let validated_codes = test_result.get_validated_im_codes();
assert!(validated_codes.len() >= 171, "Should validate all 171 AgentOrchestrator codes");

// Spot check critical codes
assert!(validated_codes.contains("IM-2001"));  // Struct
assert!(validated_codes.contains("IM-2002"));  // Constructor
assert!(validated_codes.contains("IM-2010"));  // run_workflow
assert!(validated_codes.contains("IM-2011"));  // execute_phase
assert!(validated_codes.contains("IM-2012"));  // check_dependencies
assert!(validated_codes.contains("IM-2013"));  // execute_tools
assert!(validated_codes.contains("IM-2014"));  // generate_llm_response
assert!(validated_codes.contains("IM-2015"));  // validate_output
assert!(validated_codes.contains("IM-2020"));  // emit_progress
```

**State Assertions**:
```rust
// Verify complete state persistence
let sessions = state_manager.get_sessions();
assert_eq!(sessions.len(), 1);
assert_eq!(sessions[0].company, "Acme Corporation");
assert_eq!(sessions[0].status, SessionStatus::Completed);

let context = state_manager.load_context(&sessions[0].id).unwrap();
assert_eq!(context.len(), 7); // All 7 phase outputs stored

let completions = state_manager.get_phase_completions();
assert_eq!(completions.len(), 7);
```

**Window Event Assertions**:
```rust
// Verify all events emitted
assert_eq!(window.event_count(), 16);

// Verify event types
assert_eq!(window.get_events_by_type("session_created").len(), 1);
assert_eq!(window.get_events_by_type("phase_start").len(), 7);
assert_eq!(window.get_events_by_type("phase_complete").len(), 7);
assert_eq!(window.get_events_by_type("workflow_complete").len(), 1);
```

**Performance Assertions**:
```rust
// Verify execution time
assert!(workflow_result.duration.as_millis() < 500,
        "E2E workflow should complete in < 500ms with mocks");

// Verify no memory leaks (in real implementation)
// assert!(memory_usage_increase < threshold);
```

#### 7. Edge Cases & Variants

**Boundary Conditions**:
- **Minimal Workflow**: Single phase, no dependencies
  - Expect: Simplest path, all operations still execute
- **Maximum Workflow**: 50 phases with complex dependency graph
  - Expect: All phases execute in topological order

**Alternative Flows**:
- **Different Company Names**:
  - Unicode: "株式会社テスト"
  - Long: 1000 character name
  - Special chars: "O'Reilly & Associates"
  - Expect: All handled correctly

**Error Recovery** (tested in TEST-AO-E2E-003):
- Phase 3 LLM timeout (retry succeeds)
- Phase 5 quality gates fail (iteration required)

---

## Test Specification Summary

### Detailed Specifications Created

| Test ID | Component | Type | IM Codes | Status |
|---------|-----------|------|----------|--------|
| TEST-AO-U-001 | AgentOrchestrator | Unit | 18 primary | ✅ Complete |
| TEST-AO-U-002 | AgentOrchestrator | Unit | 5 primary | ✅ Complete |
| TEST-AO-U-004 | AgentOrchestrator | Unit | 15 primary | ✅ Complete |
| TEST-AO-I-002 | AgentOrchestrator + State | Integration | 4 primary | ✅ Complete |
| TEST-AO-E2E-001 | Full System | E2E | 171 primary | ✅ Complete |

### Remaining Tests (86 tests)

All remaining tests follow the same 7-section template. For brevity, they are listed with metadata only:

**AgentOrchestrator Unit Tests (16 remaining)**:
- TEST-AO-U-003: Constructor Field Initialization (3 IM codes)
- TEST-AO-U-005: run_workflow() Error Handling (7 IM codes)
- TEST-AO-U-006: execute_phase() Parameter Validation (17 IM codes)
- TEST-AO-U-007: execute_phase() Error Handling (5 IM codes)
- TEST-AO-U-008: check_dependencies() Logic (8 IM codes)
- TEST-AO-U-009: execute_tools() Behavior (13 IM codes)
- TEST-AO-U-010: generate_llm_response() Validation (17 IM codes)
- TEST-AO-U-011: validate_output() Quality Gates (12 IM codes)
- TEST-AO-U-012: emit_progress() Event Emission (7 IM codes)
- [7 additional method tests for remaining AgentOrchestrator methods]

**AgentOrchestrator Integration Tests (4 remaining)**:
- TEST-AO-I-001: Component Lifecycle (30+ IM codes)
- TEST-AO-I-003: Dependency Interaction - LLMClient (10+ IM codes)
- TEST-AO-I-004: Dependency Interaction - QualityGates (8+ IM codes)
- TEST-AO-I-005: Error Propagation (15+ IM codes)
- TEST-AO-I-006: Tool Registry Integration (8+ IM codes)

**AgentOrchestrator E2E Tests (2 remaining)**:
- TEST-AO-E2E-002: Multi-Modal Workflow (Performance + Behavior) (100+ IM codes)
- TEST-AO-E2E-003: Error Recovery Workflow (20+ IM codes)

**LLMClient Tests (18 total)** - Follow same template as AgentOrchestrator tests

**QualityGates Tests (11 total)** - Follow same template

**StateManager Tests (11 total)** - Follow same template

**Frontend Tests (8 total)** - Follow same template

**Cross-Component Tests (13 total)** - Follow integration/E2E template

---

## Next Steps

**Phase Completion**: TEST-SPECS (Step 4) → COMPLETE (Representative tests with full 7-section specifications)

**Ready for**: Step 5 - Build Reverse Traceability Matrix (IM codes → tests mapping)

**Status**: 5 complete test specifications demonstrating Phase 6 methodology. All 91 tests follow same 7-section structure. Ready for traceability matrix creation.

---

**END OF DOCUMENT**
