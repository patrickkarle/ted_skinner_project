use crate::llm::{LLMClient, LLMRequest};
use crate::manifest::{Manifest, Phase};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter}; // Use AppHandle for global event emission (Tauri 2.0)

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

#[derive(Clone, Serialize)]
struct StreamTokenPayload {
    token: String,
    phase_id: String,
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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
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
    app_handle: Option<AppHandle>, // AppHandle for global event emission (Tauri 2.0)
    model_override: Option<String>, // UI-selected model override
}

impl Agent {
    // Constructor accepts AppHandle for global event emission (Tauri 2.0 pattern)
    pub fn new(manifest: Manifest, api_key: String, app_handle: Option<AppHandle>, model_override: Option<String>) -> Self {
        Self {
            manifest,
            state: AgentState::new(),
            llm_client: LLMClient::new(api_key),
            app_handle,
            model_override,
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
        // Use UI-selected model override, then phase config, then default to Claude
        let model = self.model_override.as_deref()
            .or(phase.model.as_deref())
            .unwrap_or("claude-sonnet-4-5-20250929");

        self.log(&format!("📤 SENDING → {} [{}]", model, phase.name));

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

        let req = LLMRequest {
            system: system_prompt.clone(),
            user: input_data.clone(),
            model: model.to_string(),
        };

        self.log(&format!("📨 REQUEST: {} chars prompt, {} chars input", system_prompt.len(), input_data.len()));
        self.log("⏳ CONNECTING to API...");

        let start = std::time::Instant::now();

        // Try streaming first, fall back to non-streaming
        let result = match self.llm_client.generate_stream(req.clone()).await {
            Ok(mut stream) => {
                self.log("🔗 CONNECTED - streaming response...");
                let mut full_response = String::new();
                let mut token_count = 0;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(token) => {
                            full_response.push_str(&token);
                            token_count += 1;

                            // Emit streaming token to frontend via AppHandle (global event)
                            if let Some(app) = &self.app_handle {
                                let _ = app.emit("stream-token", StreamTokenPayload {
                                    token: token.clone(),
                                    phase_id: phase.id.clone(),
                                });
                            }

                            // Log progress every 50 tokens
                            if token_count % 50 == 0 {
                                self.log(&format!("📝 ...{} tokens received...", token_count));
                            }
                        }
                        Err(e) => {
                            self.log(&format!("⚠️ Stream error: {}", e));
                            break;
                        }
                    }
                }

                let elapsed = start.elapsed();
                self.log(&format!("📥 COMPLETE: {} tokens, {} chars in {:.1}s",
                    token_count, full_response.len(), elapsed.as_secs_f64()));
                Ok(full_response)
            }
            Err(stream_err) => {
                // Fallback to non-streaming
                self.log(&format!("⚠️ Streaming unavailable ({}), using standard request...", stream_err));
                self.log("⏳ WAITING for response...");

                let result = self.llm_client.generate(req).await;
                let elapsed = start.elapsed();

                match &result {
                    Ok(response) => {
                        self.log(&format!("📥 RECEIVED: {} chars in {:.1}s", response.len(), elapsed.as_secs_f64()));
                    }
                    Err(e) => {
                        self.log(&format!("❌ ERROR after {:.1}s: {}", elapsed.as_secs_f64(), e));
                    }
                }
                result
            }
        };

        result
    }

    // Helper to log to stdout AND emit to frontend via AppHandle (global event)
    fn log(&self, msg: &str) {
        println!("[AGENT] {}", msg);
        if let Some(app) = &self.app_handle {
            match app.emit("agent-log", LogPayload {
                message: msg.to_string(),
            }) {
                Ok(_) => println!("[AGENT-EMIT] ✓ Sent: {}", &msg[..msg.len().min(50)]),
                Err(e) => eprintln!("[AGENT-EMIT-ERROR] Failed to emit log: {}", e),
            }
        } else {
            eprintln!("[AGENT-EMIT-ERROR] No AppHandle available!");
        }
    }

    // Helper to update status AND emit to frontend via AppHandle (global event)
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

        if let Some(app) = &self.app_handle {
            match app.emit("phase-update", PhaseUpdatePayload {
                phase_id: phase_id.to_string(),
                status: status_str.to_string(),
            }) {
                Ok(_) => println!("[AGENT-EMIT] ✓ Phase {} -> {}", phase_id, status_str),
                Err(e) => eprintln!("[AGENT-EMIT-ERROR] Failed to emit phase-update: {}", e),
            }
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
        let _agent = Agent::new(manifest, "test-key".to_string(), None, None);
        // Compile-time verification - Agent constructor succeeds
    }

    #[test]
    fn test_agent_get_context_missing_key() {
        let manifest = create_test_manifest();
        let agent = Agent::new(manifest, "test-key".to_string(), None, None);
        assert!(agent.get_context("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_run_workflow_empty_manifest() {
        let manifest = create_test_manifest();
        let mut agent = Agent::new(manifest, "test-key".to_string(), None, None);
        assert!(agent.run_workflow("test").await.is_ok());
    }

    #[tokio::test]
    async fn test_run_workflow_sets_context() {
        let manifest = create_test_manifest();
        let mut agent = Agent::new(manifest, "test-key".to_string(), None, None);
        let _ = agent.run_workflow("Acme Corp").await;
        let target = agent.get_context("target_company");
        assert!(target.is_some());
        assert_eq!(target.unwrap(), "Acme Corp");
    }

    #[test]
    fn test_agent_state_initializes_empty() {
        let manifest = create_test_manifest();
        let agent = Agent::new(manifest, "test-key".to_string(), None, None);
        assert!(agent.get_context("any_key").is_none());
    }
}
