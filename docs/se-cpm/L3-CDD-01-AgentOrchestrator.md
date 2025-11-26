# L3-CDD-01: AgentOrchestrator Component Design

**Document ID:** L3-CDD-FULLINTEL-001
**Component:** AgentOrchestrator
**Atomic Level:** COMPOUND
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L1-SAD-FULLINTEL-001

---

## 1. Component Overview

### 1.1 Purpose
The AgentOrchestrator is the central coordination component that executes the 5-phase research workflow. It manages phase dependencies, context accumulation, and error recovery.

### 1.2 Atomic Classification
- **Level:** COMPOUND (100-275 lines)
- **Rationale:** Manages state, coordinates multiple subsystems (ToolRegistry, LLMClient, QualityGates), handles complex error scenarios

### 1.3 Dependencies
```rust
// Internal dependencies
use crate::manifest::{Manifest, Phase};
use crate::llm::LLMClient;
use crate::tools::ToolRegistry;
use crate::quality_gates::QualityGateValidator;
use crate::state::StateManager;

// External dependencies
use anyhow::{Result, Context};
use serde_json::{json, Value};
use std::collections::HashMap;
use tauri::Window;
```

---

## 2. Data Structures

### 2.1 AgentOrchestrator Struct
```rust
pub struct AgentOrchestrator {
    manifest: Manifest,
    llm_client: LLMClient,
    tool_registry: ToolRegistry,
    quality_validator: QualityGateValidator,
    state_manager: StateManager,
    context: HashMap<String, Value>,
    current_phase: Option<String>,
}
```

### 2.2 WorkflowResult Enum
```rust
pub enum WorkflowResult {
    Success(String),           // Final output (markdown brief)
    QualityGateFailed {
        phase_id: String,
        reason: String,
        partial_output: String,
    },
    PhaseError {
        phase_id: String,
        error: String,
        retry_count: u32,
    },
}
```

### 2.3 PhaseContext Struct
```rust
pub struct PhaseContext {
    pub phase_id: String,
    pub inputs: HashMap<String, Value>,
    pub outputs: HashMap<String, Value>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub retry_count: u32,
}
```

---

## 3. Public Interface

### 3.1 Constructor
```rust
impl AgentOrchestrator {
    /// Creates new orchestrator from manifest file path
    ///
    /// # Arguments
    /// * `manifest_path` - Path to YAML manifest
    /// * `llm_client` - Configured LLM client
    /// * `state_manager` - State persistence manager
    ///
    /// # Errors
    /// Returns error if manifest cannot be loaded or validated
    pub fn new(
        manifest_path: &str,
        llm_client: LLMClient,
        state_manager: StateManager,
    ) -> Result<Self> {
        let manifest = Manifest::load(manifest_path)?;
        let tool_registry = ToolRegistry::new();
        let quality_validator = QualityGateValidator::from_manifest(&manifest)?;

        Ok(Self {
            manifest,
            llm_client,
            tool_registry,
            quality_validator,
            state_manager,
            context: HashMap::new(),
            current_phase: None,
        })
    }
}
```

### 3.2 Core Methods

#### run_workflow
```rust
/// Executes complete 5-phase workflow for target company
///
/// # Arguments
/// * `company` - Target company name
/// * `window` - Tauri window for progress updates
///
/// # Returns
/// WorkflowResult with final brief or error details
///
/// # Side Effects
/// - Emits progress events to window
/// - Saves state after each phase completion
/// - Updates context with phase outputs
pub async fn run_workflow(
    &mut self,
    company: String,
    window: Option<&Window>,
) -> Result<WorkflowResult> {
    // Initialize workflow
    self.context.insert("target_company".to_string(), json!(company));
    self.emit_progress(&window, "workflow_started", &json!({})).await;

    // Execute phases sequentially
    for phase in &self.manifest.phases {
        match self.execute_phase(phase, &window).await {
            Ok(_) => {
                self.state_manager.save_phase_completion(
                    phase.id.clone(),
                    &self.context
                )?;
            }
            Err(e) => {
                return Ok(WorkflowResult::PhaseError {
                    phase_id: phase.id.clone(),
                    error: e.to_string(),
                    retry_count: 0,
                });
            }
        }
    }

    // Extract final output
    let final_brief = self.context
        .get("markdown_file")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("No final output generated"))?;

    Ok(WorkflowResult::Success(final_brief.to_string()))
}
```

#### execute_phase
```rust
/// Executes single phase with dependency checking and error handling
///
/// # Arguments
/// * `phase` - Phase specification from manifest
/// * `window` - Optional Tauri window for progress updates
///
/// # Returns
/// Unit on success, error on failure
///
/// # Process
/// 1. Check dependencies satisfied
/// 2. Execute tools if specified
/// 3. Call LLM if model specified
/// 4. Validate with quality gates
/// 5. Store output in context
async fn execute_phase(
    &mut self,
    phase: &Phase,
    window: &Option<&Window>,
) -> Result<()> {
    self.current_phase = Some(phase.id.clone());

    // Emit phase start event
    self.emit_progress(
        window,
        "phase_started",
        &json!({
            "phase_id": phase.id,
            "phase_name": phase.name,
        })
    ).await;

    // Check dependencies
    if !self.check_dependencies(phase) {
        anyhow::bail!("Dependencies not satisfied for phase {}", phase.id);
    }

    // Execute tools
    let mut tool_results = HashMap::new();
    for tool_name in &phase.tools {
        let tool_output = self.execute_tool(tool_name, &phase.id).await?;
        tool_results.insert(tool_name.clone(), tool_output);
    }

    // Call LLM if specified
    let phase_output = if let Some(model) = &phase.model {
        self.generate_with_llm(phase, &tool_results).await?
    } else {
        // Logic-based phase (e.g., Phase 4: Solution Matching)
        self.execute_logic(phase, &tool_results)?
    };

    // Quality gate validation
    if let Err(validation_error) = self.quality_validator.validate(&phase.id, &phase_output) {
        self.emit_progress(
            window,
            "quality_gate_failed",
            &json!({
                "phase_id": phase.id,
                "reason": validation_error.to_string(),
            })
        ).await;

        // Retry with stricter prompt if configured
        if phase.retry_on_quality_failure {
            return self.retry_phase_with_penalty(phase, &tool_results).await;
        } else {
            anyhow::bail!("Quality gate failed: {}", validation_error);
        }
    }

    // Store output in context
    self.context.insert(phase.output_key.clone(), json!(phase_output));

    // Emit phase completion
    self.emit_progress(
        window,
        "phase_completed",
        &json!({
            "phase_id": phase.id,
            "output_preview": phase_output.chars().take(200).collect::<String>(),
        })
    ).await;

    Ok(())
}
```

### 3.3 Helper Methods

#### check_dependencies
```rust
/// Verifies all phase dependencies are satisfied
///
/// # Arguments
/// * `phase` - Phase to check
///
/// # Returns
/// true if all dependencies met, false otherwise
fn check_dependencies(&self, phase: &Phase) -> bool {
    phase.dependencies.iter().all(|dep_phase_id| {
        // Check if dependency phase output exists in context
        self.manifest.phases
            .iter()
            .find(|p| &p.id == dep_phase_id)
            .and_then(|dep_phase| self.context.get(&dep_phase.output_key))
            .is_some()
    })
}
```

#### execute_tool
```rust
/// Executes external tool from registry
///
/// # Arguments
/// * `tool_name` - Tool identifier from manifest
/// * `phase_id` - Current phase ID for context
///
/// # Returns
/// Tool output as string
async fn execute_tool(&self, tool_name: &str, phase_id: &str) -> Result<String> {
    let tool_args = self.build_tool_args(tool_name, phase_id);

    self.tool_registry
        .execute(tool_name, tool_args)
        .await
        .with_context(|| format!("Tool execution failed: {}", tool_name))
}
```

#### generate_with_llm
```rust
/// Generates output using LLM with context from previous phases
///
/// # Arguments
/// * `phase` - Phase specification
/// * `tool_results` - Results from tool executions
///
/// # Returns
/// LLM-generated output
async fn generate_with_llm(
    &self,
    phase: &Phase,
    tool_results: &HashMap<String, String>,
) -> Result<String> {
    use crate::llm::LLMRequest;

    // Build prompt with accumulated context
    let prompt = self.build_phase_prompt(phase, tool_results);

    let request = LLMRequest {
        model: phase.model.clone().unwrap(),
        prompt,
        max_tokens: phase.max_tokens.unwrap_or(4000),
        temperature: phase.temperature.unwrap_or(0.7),
        system: phase.system_prompt.clone(),
    };

    self.llm_client.generate(request).await
}
```

#### emit_progress
```rust
/// Emits progress event to Tauri frontend
///
/// # Arguments
/// * `window` - Optional Tauri window
/// * `event_type` - Event name
/// * `payload` - Event data
async fn emit_progress(&self, window: &Option<&Window>, event_type: &str, payload: &Value) {
    if let Some(w) = window {
        let _ = w.emit(event_type, payload);
    }
}
```

---

## 4. Error Handling Strategy

### 4.1 Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum OrchestratorError {
    #[error("Phase {0} failed: {1}")]
    PhaseExecutionFailed(String, String),

    #[error("Dependencies not satisfied for phase {0}")]
    DependenciesNotMet(String),

    #[error("Quality gate failed for phase {0}: {1}")]
    QualityGateFailed(String, String),

    #[error("Tool execution failed: {0}")]
    ToolFailed(String),

    #[error("LLM generation failed: {0}")]
    LLMFailed(String),
}
```

### 4.2 Retry Logic
```rust
/// Retry phase with stricter prompt after quality gate failure
async fn retry_phase_with_penalty(
    &mut self,
    phase: &Phase,
    tool_results: &HashMap<String, String>,
) -> Result<()> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 1..=MAX_RETRIES {
        // Add penalty instruction to system prompt
        let mut retry_phase = phase.clone();
        retry_phase.system_prompt = Some(format!(
            "{}\n\nIMPORTANT: Previous attempt failed quality checks. Be more specific and avoid generic language.",
            phase.system_prompt.as_deref().unwrap_or("")
        ));

        let output = self.generate_with_llm(&retry_phase, tool_results).await?;

        if self.quality_validator.validate(&phase.id, &output).is_ok() {
            self.context.insert(phase.output_key.clone(), json!(output));
            return Ok(());
        }

        if attempt == MAX_RETRIES {
            anyhow::bail!("Quality gate failed after {} retries", MAX_RETRIES);
        }
    }

    Ok(())
}
```

---

## 5. State Management

### 5.1 Session Persistence
```rust
/// Save current workflow state to database
pub fn save_checkpoint(&self) -> Result<()> {
    self.state_manager.save_session(
        &self.current_phase,
        &self.context,
    )
}

/// Resume workflow from saved state
pub fn resume_from_checkpoint(session_id: &str) -> Result<Self> {
    let (manifest_path, phase, context) = StateManager::load_session(session_id)?;

    let mut orchestrator = Self::new(manifest_path, /* ... */)?;
    orchestrator.current_phase = phase;
    orchestrator.context = context;

    Ok(orchestrator)
}
```

---

## 6. Testing Requirements

### 6.1 Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dependency_checking() {
        // Test that phase dependencies are correctly validated
    }

    #[tokio::test]
    async fn test_context_accumulation() {
        // Test that outputs are stored in context correctly
    }

    #[tokio::test]
    async fn test_quality_gate_enforcement() {
        // Test that quality gates block completion on failure
    }
}
```

### 6.2 Integration Tests
```rust
#[tokio::test]
async fn test_full_workflow_execution() {
    // Test complete 5-phase workflow with mocked tools and LLM
}

#[tokio::test]
async fn test_error_recovery() {
    // Test that workflow can resume after crash
}
```

---

## 7. Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Phase execution | < 60s each | Average time per phase |
| Context lookup | < 1ms | HashMap access time |
| State save | < 100ms | SQLite write time |
| Total workflow | < 5 min | End-to-end duration |

---

## 8. Implementation Notes

### 8.1 Critical Considerations
- **Context Size:** Monitor context HashMap size - if exceeds 10MB, implement compression
- **Parallel Phases:** Currently sequential, but Phase 3 and 4 could run in parallel (same dependencies)
- **Streaming:** Consider streaming LLM responses for better UX (future enhancement)
- **Cancellation:** Support workflow cancellation via Tauri window close event

### 8.2 Future Enhancements
- Phase timeout limits
- Automatic retry with exponential backoff
- Metrics collection (phase duration, token usage)
- A/B testing different prompts

---

**Status:** Ready for Implementation
**Next:** L3-CDD-02-ToolRegistry.md
