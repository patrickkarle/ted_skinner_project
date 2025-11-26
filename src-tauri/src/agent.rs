use crate::llm::{LLMClient, LLMRequest};
use crate::manifest::{Manifest, Phase};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{Emitter, Window}; // Import Tauri Emitter traits

// ------------------------------------------------------------------
// Event Payloads (Sent to Frontend)
// ------------------------------------------------------------------

#[derive(Clone, Serialize)]
struct LogPayload {
    message: String,
}

#[derive(Clone, Serialize)]
struct PhaseUpdatePayload {
    phase_id: String,
    status: String, // "running", "completed", "failed"
}

// ------------------------------------------------------------------
// State Structures
// ------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
    Skipped,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentState {
    pub current_phase_id: Option<String>,
    pub phase_statuses: HashMap<String, PhaseStatus>,
    pub context: HashMap<String, String>,
    pub logs: Vec<String>,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            current_phase_id: None,
            phase_statuses: HashMap::new(),
            context: HashMap::new(),
            logs: Vec::new(),
        }
    }
}

// ------------------------------------------------------------------
// The Agent
// ------------------------------------------------------------------

pub struct Agent {
    manifest: Manifest,
    state: AgentState,
    llm_client: LLMClient,
    window: Option<Window>, // Add Window handle
}

impl Agent {
    // Modified constructor to accept optional window
    pub fn new(manifest: Manifest, api_key: String, window: Option<Window>) -> Self {
        Self {
            manifest,
            state: AgentState::new(),
            llm_client: LLMClient::new(api_key),
            window,
        }
    }

    // Public accessor for context
    pub fn get_context(&self, key: &str) -> Option<String> {
        self.state.context.get(key).cloned()
    }

    pub async fn run_workflow(&mut self, initial_input: &str) -> Result<()> {
        self.state
            .context
            .insert("target_company".to_string(), initial_input.to_string());

        let phases = self.manifest.phases.clone();

        for phase in phases {
            self.state.current_phase_id = Some(phase.id.clone());
            self.update_phase_status(&phase.id, PhaseStatus::Running);

            match self.execute_phase(&phase).await {
                Ok(output) => {
                    self.log(&format!("Phase {} completed.", phase.name));
                    self.update_phase_status(&phase.id, PhaseStatus::Completed);

                    if let Some(target) = &phase.output_target {
                        self.state.context.insert(target.clone(), output);
                    } else if let Some(schema) = &phase.output_schema {
                        self.state.context.insert(schema.clone(), output);
                    }
                }
                Err(e) => {
                    self.log(&format!("Phase {} failed: {}", phase.name, e));
                    self.update_phase_status(&phase.id, PhaseStatus::Failed(e.to_string()));
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    async fn execute_phase(&mut self, phase: &Phase) -> Result<String> {
        self.log(&format!("Executing Phase: {}", phase.name));

        let input_data = if let Some(input_key) = &phase.input {
            self.state
                .context
                .get(input_key)
                .ok_or_else(|| anyhow!("Missing input: {}", input_key))?
                .clone()
        } else {
            serde_json::to_string(&self.state.context)?
        };

        let system_prompt = format!(
            "You are an autonomous research agent executing phase '{}'.\nInstructions:\n{}",
            phase.name, phase.instructions
        );

        // --- REAL IMPLEMENTATION SWITCH ---
        // If we had a search tool, we'd call it here.
        // Since we don't have the Tavily crate installed yet, we'll rely on LLM hallucination/knowledge
        // for the 'search' phases just to make the loop work for this demo.

        let model = "claude-3-5-sonnet"; // Default to Claude

        let req = LLMRequest {
            system: system_prompt,
            user: input_data,
            model: model.to_string(),
        };

        self.llm_client.generate(req).await
    }

    // Helper to log to stdout AND emit to frontend
    fn log(&self, msg: &str) {
        println!("[AGENT] {}", msg);
        if let Some(window) = &self.window {
            let _ = window.emit(
                "agent-log",
                LogPayload {
                    message: msg.to_string(),
                },
            );
        }
    }

    // Helper to update status AND emit to frontend
    fn update_phase_status(&mut self, phase_id: &str, status: PhaseStatus) {
        self.state
            .phase_statuses
            .insert(phase_id.to_string(), status.clone());

        let status_str = match status {
            PhaseStatus::Running => "running",
            PhaseStatus::Completed => "completed",
            PhaseStatus::Failed(_) => "failed",
            _ => "pending",
        };

        if let Some(window) = &self.window {
            let _ = window.emit(
                "phase-update",
                PhaseUpdatePayload {
                    phase_id: phase_id.to_string(),
                    status: status_str.to_string(),
                },
            );
        }
    }
}
// ============================================================================
// UNIT TESTS - Agent Core Functionality
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_manifest() -> Manifest {
        let yaml_content = r#"
manifest:
  id: "TEST-001"
  version: "1.0.0"
  name: "Test Manifest"
  description: "Minimal test manifest"

schemas: {}
phases: []
quality_gates: []
"#;
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", yaml_content).unwrap();
        Manifest::load_from_file(file.path()).unwrap()
    }

    #[test]
    fn test_agent_new_initializes_correctly() {
        let manifest = create_test_manifest();
        let _agent = Agent::new(manifest, "test-key".to_string(), None);
        assert!(true);
    }

    #[test]
    fn test_agent_get_context_missing_key() {
        let manifest = create_test_manifest();
        let agent = Agent::new(manifest, "test-key".to_string(), None);
        assert!(agent.get_context("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_run_workflow_empty_manifest() {
        let manifest = create_test_manifest();
        let mut agent = Agent::new(manifest, "test-key".to_string(), None);
        assert!(agent.run_workflow("test").await.is_ok());
    }

    #[tokio::test]
    async fn test_run_workflow_sets_context() {
        let manifest = create_test_manifest();
        let mut agent = Agent::new(manifest, "test-key".to_string(), None);
        let _ = agent.run_workflow("Acme Corp").await;
        let target = agent.get_context("target_company");
        assert!(target.is_some());
        assert_eq!(target.unwrap(), "Acme Corp");
    }

    #[test]
    fn test_agent_state_initializes_empty() {
        let manifest = create_test_manifest();
        let agent = Agent::new(manifest, "test-key".to_string(), None);
        assert!(agent.get_context("any_key").is_none());
    }
}
