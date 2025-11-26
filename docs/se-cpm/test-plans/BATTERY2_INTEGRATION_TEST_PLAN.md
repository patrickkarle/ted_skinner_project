# Battery 2: Integration Test Plan - Ted Skinner Project
**Project:** Ted Skinner SE-CPM Autonomous Agent
**Phase:** 10 - EXECUTE TESTS (Continuum Development Process v4.6)
**Battery:** 2 of 3 (Integration Tests)
**Target Tests:** 20 integration tests
**Status:** 📋 PLANNING
**Created:** 2025-11-24

---

## Executive Summary

Battery 2 focuses on **integration testing** - verifying that multiple components work together correctly. While Battery 1 tested individual components in isolation, Battery 2 tests the interactions, data flow, and contracts between components.

### Test Strategy

- **Focus:** Component interactions and data contracts
- **Methodology:** Integration testing with real component composition
- **Coverage Target:** Critical integration paths
- **Prerequisite:** Battery 1 complete (30/30 tests ✅)

### Test Distribution

| Group | Focus Area | Tests | Priority |
|-------|-----------|-------|----------|
| **Group 1** | Agent ↔ Manifest Integration | 5 | HIGH |
| **Group 2** | Agent ↔ LLMClient Integration | 5 | HIGH |
| **Group 3** | LLMClient ↔ RateLimiter ↔ CircuitBreaker | 5 | MEDIUM |
| **Group 4** | End-to-End Workflow | 5 | MEDIUM |
| **TOTAL** | | **20** | |

---

## Group 1: Agent ↔ Manifest Integration (5 tests)

**Focus:** Verify Agent correctly loads, interprets, and executes Manifest instructions.

### Test 2.1.1: Agent Loads Valid Manifest

**Objective:** Verify Agent successfully loads and parses a valid manifest YAML file.

**Components Tested:**
- Agent constructor
- Manifest::load_from_file()
- Agent.manifest field initialization

**Test Steps:**
1. Create a valid manifest YAML file with phases
2. Construct Agent with manifest path
3. Verify Agent initialized without errors
4. Verify manifest phases are accessible

**Expected Behavior:**
- Agent constructs successfully
- Manifest loaded with all phases present
- No errors or panics

**Code Pattern:**
```rust
#[tokio::test]
async fn test_agent_loads_valid_manifest() {
    let manifest_yaml = r#"
    manifest:
      id: "TEST-001"
      name: "Test Manifest"
    phases:
      - id: "phase1"
        name: "Test Phase"
        instructions: "Test instructions"
    "#;

    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", manifest_yaml).unwrap();
    let manifest = Manifest::load_from_file(file.path()).unwrap();

    let agent = Agent::new(manifest, "test-key".to_string(), None);
    // Verify agent initialized successfully
}
```

---

### Test 2.1.2: Agent Rejects Invalid Manifest

**Objective:** Verify Agent properly handles malformed or invalid manifest files.

**Components Tested:**
- Manifest::load_from_file() error handling
- YAML parsing error propagation
- Agent error handling

**Test Steps:**
1. Create invalid manifest YAML (syntax errors)
2. Attempt to load manifest
3. Verify appropriate error returned
4. Try multiple invalid formats

**Expected Behavior:**
- Manifest loading returns Err
- Error message indicates parsing failure
- No panics or undefined behavior

---

### Test 2.1.3: Agent Executes Phase from Manifest

**Objective:** Verify Agent can extract phase instructions from manifest and execute them.

**Components Tested:**
- Agent::execute_phase()
- Phase instruction interpretation
- Context variable substitution
- Phase input/output handling

**Test Steps:**
1. Create manifest with single phase
2. Initialize Agent
3. Execute phase directly
4. Verify phase instructions processed
5. Check context variables updated

**Expected Behavior:**
- Phase executes without errors
- Instructions interpreted correctly
- Context updated with output

---

### Test 2.1.4: Agent Handles Missing Phase Input

**Objective:** Verify Agent properly handles when required phase input is missing from context.

**Components Tested:**
- Phase.input handling
- Agent context lookup
- Error reporting

**Test Steps:**
1. Create phase with input: "required_input"
2. Don't populate "required_input" in context
3. Attempt to execute phase
4. Verify error returned

**Expected Behavior:**
- Phase execution returns Err
- Error message indicates missing input
- Agent state remains consistent

---

### Test 2.1.5: Agent Workflow Multi-Phase Execution

**Objective:** Verify Agent executes multiple phases sequentially with data flow.

**Components Tested:**
- Agent::run_workflow()
- Phase sequencing
- Context data flow between phases
- Phase status tracking

**Test Steps:**
1. Create manifest with 3 phases:
   - Phase 1: Generates output "data1"
   - Phase 2: Takes "data1" as input, generates "data2"
   - Phase 3: Takes "data2" as input
2. Run workflow with initial input
3. Verify all phases execute in order
4. Verify data flows correctly

**Expected Behavior:**
- All phases execute sequentially
- Each phase receives correct input from previous phase
- Final context contains all intermediate outputs
- Phase statuses updated correctly (Pending → Running → Completed)

---

## Group 2: Agent ↔ LLMClient Integration (5 tests)

**Focus:** Verify Agent correctly uses LLMClient for LLM API calls during phase execution.

### Test 2.2.1: Agent Uses LLMClient for Phase Execution

**Objective:** Verify Agent invokes LLMClient.generate() during phase execution.

**Components Tested:**
- Agent::execute_phase()
- LLMClient::generate()
- LLMRequest construction
- Agent ↔ LLMClient data flow

**Test Steps:**
1. Create manifest with phase requiring LLM
2. Initialize Agent with mock/test API key
3. Execute phase
4. Verify LLMClient called (may need mock or actual call)
5. Verify phase output generated

**Expected Behavior:**
- LLMClient.generate() invoked with correct request
- System prompt includes phase instructions
- User message includes phase input
- Phase output populated with LLM response

**Note:** This may require mocking or a test API key. Consider using mock implementation if testing without real API.

---

### Test 2.2.2: Agent Handles LLM Rate Limit Errors

**Objective:** Verify Agent properly handles when LLMClient encounters rate limiting.

**Components Tested:**
- Agent error handling
- LLMError::RateLimitExceeded propagation
- Phase failure handling

**Test Steps:**
1. Configure LLMClient with very low rate limit (via RateLimiter)
2. Execute phase that requires LLM call
3. Exhaust rate limit
4. Attempt phase execution
5. Verify error handled gracefully

**Expected Behavior:**
- Phase execution returns Err
- Error is LLMError::RateLimitExceeded
- Phase status set to Failed(error_message)
- Agent state remains consistent

---

### Test 2.2.3: Agent Handles LLM Network Errors

**Objective:** Verify Agent handles network/connectivity failures from LLMClient.

**Components Tested:**
- LLMError::NetworkError propagation
- Agent retry logic (if any)
- Error recovery

**Test Steps:**
1. Simulate network failure (invalid URL or unreachable endpoint)
2. Execute phase requiring LLM
3. Verify error caught and handled
4. Check phase status and error message

**Expected Behavior:**
- Phase execution returns Err
- Error is LLMError::NetworkError
- Phase status set to Failed
- Clear error message logged

---

### Test 2.2.4: Agent Constructs Proper LLMRequest from Phase

**Objective:** Verify Agent constructs well-formed LLMRequest from phase data.

**Components Tested:**
- LLMRequest construction
- System prompt formatting
- User message formatting
- Model selection

**Test Steps:**
1. Create phase with specific instructions and input
2. Execute phase
3. Capture LLMRequest constructed (may need instrumentation)
4. Verify system prompt includes phase name and instructions
5. Verify user message includes phase input

**Expected Behavior:**
- LLMRequest.system contains phase instructions
- LLMRequest.user contains input data
- LLMRequest.model set to default ("claude-3-5-sonnet")

**Implementation Note:** May need to add test instrumentation or extract method for request construction to verify.

---

### Test 2.2.5: Agent Streams LLM Responses (If Applicable)

**Objective:** Verify Agent can handle streaming LLM responses.

**Components Tested:**
- LLMClient streaming support
- Agent stream handling
- Response assembly

**Test Steps:**
1. Configure LLMClient for streaming
2. Execute phase
3. Verify Agent handles stream correctly
4. Verify complete response assembled

**Expected Behavior:**
- Stream initiated successfully
- Agent processes chunks as they arrive
- Final response complete and correct

**Note:** May be SKIPPED if streaming not yet implemented in current codebase. Check LLMClient API first.

---

## Group 3: LLMClient ↔ RateLimiter ↔ CircuitBreaker (5 tests)

**Focus:** Verify protective mechanisms (RateLimiter, CircuitBreaker) work correctly with LLMClient.

### Test 2.3.1: RateLimiter Throttles LLMClient Requests

**Objective:** Verify RateLimiter correctly limits LLMClient request rate.

**Components Tested:**
- LLMClient.rate_limiter
- RateLimiter::acquire()
- Request throttling

**Test Steps:**
1. Create LLMClient with low rate limit (e.g., 2 tokens per second)
2. Make 3 rapid requests
3. Verify first 2 succeed immediately
4. Verify 3rd waits or fails appropriately

**Expected Behavior:**
- First 2 requests succeed
- 3rd request throttled (waits or returns error)
- Rate limiter enforces token bucket algorithm

---

### Test 2.3.2: CircuitBreaker Opens on Repeated LLM Failures

**Objective:** Verify CircuitBreaker opens after threshold failures.

**Components Tested:**
- LLMClient.circuit_breaker
- CircuitBreaker failure tracking
- Circuit state transitions

**Test Steps:**
1. Create LLMClient with low failure threshold (e.g., 3 failures)
2. Trigger 3 consecutive failures (invalid API key or bad endpoint)
3. Verify circuit opens
4. Attempt 4th request
5. Verify request rejected immediately

**Expected Behavior:**
- First 3 failures recorded
- After 3rd failure, circuit opens
- 4th request returns CircuitBreakerError::Open
- No actual HTTP request made for 4th call

---

### Test 2.3.3: CircuitBreaker Recovers After Timeout

**Objective:** Verify CircuitBreaker transitions to HalfOpen and recovers.

**Components Tested:**
- CircuitBreaker timeout logic
- State transitions (Open → HalfOpen → Closed)
- Recovery mechanism

**Test Steps:**
1. Open circuit (as in test 2.3.2)
2. Wait for timeout duration
3. Make successful request
4. Verify circuit transitions to HalfOpen
5. Make another successful request
6. Verify circuit closes

**Expected Behavior:**
- After timeout, next request triggers HalfOpen
- Successful requests close circuit
- Normal operation resumes

---

### Test 2.3.4: RateLimiter and CircuitBreaker Work Together

**Objective:** Verify both protective mechanisms work correctly in combination.

**Components Tested:**
- RateLimiter + CircuitBreaker interaction
- Layered protection
- Request flow control

**Test Steps:**
1. Create LLMClient with both rate limiter and circuit breaker
2. Make requests that:
   - Trigger rate limiting
   - Trigger circuit breaker
   - Test recovery
3. Verify both mechanisms active

**Expected Behavior:**
- Rate limiter throttles excessive requests
- Circuit breaker opens on failures
- Both mechanisms independent and non-interfering
- Combined protection effective

---

### Test 2.3.5: LLMClient Provider Fallback with Circuit Breaker

**Objective:** Verify circuit breaker behavior with multiple LLM providers.

**Components Tested:**
- LLMClient provider switching (if implemented)
- Per-provider circuit breakers
- Fallback logic

**Test Steps:**
1. Configure LLMClient with multiple providers
2. Cause failures on primary provider (open circuit)
3. Verify fallback to secondary provider
4. Verify primary circuit opens, secondary remains closed

**Expected Behavior:**
- Primary provider circuit opens after failures
- Requests automatically route to secondary
- Secondary provider unaffected by primary failures

**Note:** May be SKIPPED if multi-provider fallback not yet implemented. Check LLMClient architecture first.

---

## Group 4: End-to-End Workflow Tests (5 tests)

**Focus:** Complete workflows exercising multiple components together.

### Test 2.4.1: Complete Agent Workflow (No API)

**Objective:** Run complete agent workflow end-to-end without real API calls.

**Components Tested:**
- Full agent workflow
- All component integration
- State management

**Test Steps:**
1. Create complete test manifest (3-5 phases)
2. Initialize Agent
3. Run full workflow
4. Verify all phases execute
5. Verify final context correct

**Expected Behavior:**
- All phases execute in sequence
- No errors or panics
- Context accumulates correctly
- Phase statuses all Completed

**Note:** Use mock LLM responses or override LLMClient to avoid real API calls.

---

### Test 2.4.2: Agent Workflow Handles Phase Failure

**Objective:** Verify agent handles mid-workflow failures gracefully.

**Components Tested:**
- Error propagation through workflow
- Workflow termination on failure
- State consistency after failure

**Test Steps:**
1. Create manifest with 3 phases
2. Configure 2nd phase to fail (missing input)
3. Run workflow
4. Verify workflow stops at failure
5. Verify phase statuses correct

**Expected Behavior:**
- Phase 1: Completed
- Phase 2: Failed(error message)
- Phase 3: Never executed (still Pending)
- Workflow returns Err

---

### Test 2.4.3: Agent Workflow with Context Sharing

**Objective:** Verify phases correctly share data via context.

**Components Tested:**
- Context as shared state
- Phase input/output mapping
- Data transformation pipeline

**Test Steps:**
1. Create workflow: Phase1 → Phase2 → Phase3
2. Phase1 outputs "data1"
3. Phase2 takes "data1", outputs "data2"
4. Phase3 takes "data2", outputs "final"
5. Verify data pipeline correct

**Expected Behavior:**
- Each phase receives correct input
- Context accumulates all outputs
- Final context has: data1, data2, final
- No data loss or corruption

---

### Test 2.4.4: Agent Handles Quality Gate Validation

**Objective:** Verify Agent evaluates quality gates correctly (if implemented).

**Components Tested:**
- QualityGate evaluation
- Gate passing/failing logic
- Workflow continuation based on gates

**Test Steps:**
1. Create manifest with phases and quality gates
2. Run workflow
3. Verify gates evaluated at correct points
4. Test both passing and failing gates

**Expected Behavior:**
- Gates evaluated after associated phase
- Passing gate allows workflow to continue
- Failing gate may trigger retry or failure

**Note:** May be SKIPPED if quality gate evaluation not yet implemented. Check manifest and agent code first.

---

### Test 2.4.5: Complete Workflow with Rate Limiting and Circuit Breaking

**Objective:** Full integration test with all protective mechanisms active.

**Components Tested:**
- Complete system integration
- RateLimiter in workflow context
- CircuitBreaker in workflow context
- Recovery and retry behavior

**Test Steps:**
1. Create multi-phase workflow
2. Configure restrictive rate limits
3. Configure sensitive circuit breaker
4. Run workflow that triggers both protections
5. Verify workflow handles limits gracefully

**Expected Behavior:**
- Workflow respects rate limits
- Circuit breaker prevents cascade failures
- Errors handled gracefully
- System remains stable under stress

---

## Implementation Strategy

### Test File Organization

**File:** `tests/battery2_integration_strategic.rs`

**Structure:**
```rust
// Battery 2: Integration Tests

// Group 1: Agent ↔ Manifest Integration
mod agent_manifest_integration {
    #[tokio::test]
    async fn test_agent_loads_valid_manifest() { ... }

    #[tokio::test]
    async fn test_agent_rejects_invalid_manifest() { ... }

    // ... 3 more tests
}

// Group 2: Agent ↔ LLMClient Integration
mod agent_llmclient_integration {
    #[tokio::test]
    async fn test_agent_uses_llmclient_for_phase() { ... }

    // ... 4 more tests
}

// Group 3: LLMClient ↔ RateLimiter ↔ CircuitBreaker
mod llm_protective_mechanisms {
    #[tokio::test]
    async fn test_rate_limiter_throttles_requests() { ... }

    // ... 4 more tests
}

// Group 4: End-to-End Workflow
mod end_to_end_workflow {
    #[tokio::test]
    async fn test_complete_workflow_no_api() { ... }

    // ... 4 more tests
}
```

### Test Utilities

Create helper functions for common test setup:

```rust
// Test utilities module
mod test_utils {
    use fullintel_agent::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Creates a test manifest with specified phases
    pub fn create_test_manifest(phases: Vec<TestPhase>) -> Manifest {
        let yaml = generate_manifest_yaml(phases);
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", yaml).unwrap();
        Manifest::load_from_file(file.path()).unwrap()
    }

    /// Creates a test agent with minimal setup
    pub fn create_test_agent(manifest: Manifest) -> Agent {
        Agent::new(manifest, "test-key".to_string(), None)
    }

    /// Mock LLM response for testing without API
    pub struct MockLLMClient {
        // Mock implementation
    }
}
```

### Mocking Strategy

**Option 1: Mock LLMClient**
- Create trait `LLMProvider`
- Implement for real `LLMClient` and `MockLLMClient`
- Agent uses trait instead of concrete type

**Option 2: Test-Only Methods**
- Add `#[cfg(test)]` methods to inject mock responses
- Minimal code changes
- Tests control LLM responses

**Option 3: Environment-Based**
- Check for test environment variable
- Return mock responses instead of real API calls
- Requires no code changes

**Recommendation:** Start with Option 2 (test-only methods) for quick implementation, refactor to Option 1 (traits) for cleaner architecture.

---

## Dependencies on Battery 1

Battery 2 tests assume Battery 1 is complete and all components work individually:

✅ **Required from Battery 1:**
- LLMClient basics working
- RateLimiter functional
- CircuitBreaker functional
- Agent initialization working
- Manifest loading working

⚠️ **Battery 1 Execution Blocked:**
- Tests compile but can't execute on Windows
- Need WSL or CI for execution
- Battery 2 will have same execution limitation

---

## Success Criteria

### Completion Criteria
- [ ] All 20 integration tests implemented
- [ ] All tests compile with 0 errors
- [ ] Mock/test infrastructure in place
- [ ] Tests verify component interactions
- [ ] Data contract validation complete

### Quality Gates
- [ ] 0 compilation errors
- [ ] All integration paths tested
- [ ] Error propagation validated
- [ ] Documentation complete

### Execution Criteria (If Windows Issue Resolved)
- [ ] All 20 tests passing
- [ ] No flaky tests
- [ ] Clear error messages
- [ ] Test execution time < 5 minutes

---

## Risks and Mitigation

### Risk 1: Windows Runtime Error Persists
**Impact:** Tests compile but can't execute
**Probability:** HIGH (known issue)
**Mitigation:**
- Focus on compilation success as proof of correctness
- Use WSL or CI for actual execution
- Document tests thoroughly

### Risk 2: Mocking Complexity
**Impact:** Tests harder to write without real API
**Probability:** MEDIUM
**Mitigation:**
- Start with simple mock implementation
- Use test environment variables for control
- Document mock behavior clearly

### Risk 3: Async Test Complexity
**Impact:** Async integration tests harder to debug
**Probability:** MEDIUM
**Mitigation:**
- Use tokio::test for async tests
- Add detailed assertions
- Log intermediate state

### Risk 4: Test Dependencies
**Impact:** Tests may be interdependent
**Probability:** LOW
**Mitigation:**
- Ensure each test fully isolated
- Use fresh test data per test
- No shared mutable state

---

## Timeline Estimate

**Assuming Windows issue resolved or using WSL:**

| Group | Setup | Implementation | Debug | Total |
|-------|-------|----------------|-------|-------|
| Group 1 (5 tests) | 30 min | 90 min | 30 min | 2.5 hours |
| Group 2 (5 tests) | 20 min | 90 min | 30 min | 2.3 hours |
| Group 3 (5 tests) | 20 min | 90 min | 30 min | 2.3 hours |
| Group 4 (5 tests) | 30 min | 120 min | 45 min | 3.25 hours |
| Documentation | - | - | 30 min | 30 min |
| **TOTAL** | | | | **10.65 hours** |

**Broken into sessions:**
- Session 1: Group 1 (2.5 hours)
- Session 2: Group 2 (2.3 hours)
- Session 3: Group 3 (2.3 hours)
- Session 4: Group 4 + Docs (3.55 hours)

---

## Next Steps

1. ✅ Battery 2 plan complete
2. ⚪ Review plan with user
3. ⚪ Set up test utilities and mocking
4. ⚪ Implement Group 1 (Agent ↔ Manifest)
5. ⚪ Implement Group 2 (Agent ↔ LLMClient)
6. ⚪ Implement Group 3 (Protective Mechanisms)
7. ⚪ Implement Group 4 (End-to-End)
8. ⚪ Run tests (WSL or CI)
9. ⚪ Generate coverage report
10. ⚪ Update documentation

---

**Document Status:** ✅ PLAN COMPLETE
**Ready for Implementation:** YES
**Prerequisites:** Battery 1 complete ✅
**Blocking Issues:** Windows runtime error (workaround: WSL/CI)

---

*Generated by Claude Code | Phase 10: EXECUTE TESTS | Continuum Development Process v4.6*
