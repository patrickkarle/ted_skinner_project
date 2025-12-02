// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod auth;
mod llm;
mod manifest;

use agent::Agent;
use auth::{AuthManager, Provider, ApiKeyEntry, Brief, BriefSummary, ConversationMessage, CustomProvider, CustomProviderSummary, PhaseOutput, Project, ProjectSummary, ResearchSession, ResearchSessionSummary, ResumeSessionResult, SessionContext, SessionHistoryMessage, SessionMessage, UserProfile};
use manifest::Manifest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, image::Image};

// ------------------------------------------------------------------
// 1. Persistent Configuration Structs
// ------------------------------------------------------------------
// We define a config struct to save to disk (app_data/config.json)
// This ensures your API Key and Settings persist across restarts.

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SavedManifest {
    name: String,
    path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppConfig {
    api_key: Option<String>,
    last_manifest_path: Option<PathBuf>,
    saved_manifests: Vec<SavedManifest>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            // Paths will be resolved properly in setup() using resolve_manifest_path()
            last_manifest_path: None,
            saved_manifests: vec![],
        }
    }
}

/// Resolves the default manifest path based on execution context.
/// In development: resolves from CARGO_MANIFEST_DIR (src-tauri/) to ../manifests/
/// In production: looks for bundled resource or falls back to executable-relative path
fn resolve_default_manifest_path() -> Option<PathBuf> {
    // Try 1: Development mode - use CARGO_MANIFEST_DIR set at compile time
    // This points to src-tauri/ directory during development
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        let dev_path = PathBuf::from(manifest_dir)
            .parent() // Go from src-tauri/ to project root
            .map(|p| p.join("manifests").join("fullintel_process_manifest.yaml"));

        if let Some(path) = dev_path {
            if path.exists() {
                println!("[DEBUG] Found manifest at dev path: {:?}", path);
                return Some(path);
            }
        }
    }

    // Try 2: Look relative to executable (for production builds)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // Try resources folder next to executable
            let resource_path = exe_dir.join("resources").join("manifests").join("fullintel_process_manifest.yaml");
            if resource_path.exists() {
                println!("[DEBUG] Found manifest at resource path: {:?}", resource_path);
                return Some(resource_path);
            }

            // Try direct manifests folder next to executable
            let direct_path = exe_dir.join("manifests").join("fullintel_process_manifest.yaml");
            if direct_path.exists() {
                println!("[DEBUG] Found manifest at direct path: {:?}", direct_path);
                return Some(direct_path);
            }
        }
    }

    // Try 3: Current working directory
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_path = cwd.join("manifests").join("fullintel_process_manifest.yaml");
        if cwd_path.exists() {
            println!("[DEBUG] Found manifest at cwd path: {:?}", cwd_path);
            return Some(cwd_path);
        }
    }

    println!("[DEBUG] Could not find default manifest in any expected location");
    None
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct PhaseInfo {
    id: String,
    name: String,
}

// ------------------------------------------------------------------
// 2. Runtime Application State
// ------------------------------------------------------------------
struct AppState {
    config: Mutex<AppConfig>,
    config_path: PathBuf,
}

impl AppState {
    // Helper to save current config state to disk
    fn save(&self) -> Result<(), String> {
        let config = self.config.lock().map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&*config).map_err(|e| e.to_string())?;

        // Ensure directory exists before writing
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
        }

        fs::write(&self.config_path, json).map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ------------------------------------------------------------------
// 2b. Authentication State
// ------------------------------------------------------------------
struct AuthState {
    manager: Mutex<AuthManager>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UserInfo {
    id: i64,
    username: String,
    first_name: Option<String>,
    last_name: Option<String>,
    role: Option<String>,
    location: Option<String>,
}

// ------------------------------------------------------------------
// 3. Tauri Commands (Frontend Callable)
// ------------------------------------------------------------------

// ------------------------------------------------------------------
// 3a. Authentication Commands
// ------------------------------------------------------------------

#[tauri::command]
async fn auth_register(
    username: String,
    password: String,
    auth_state: State<'_, AuthState>,
) -> Result<UserInfo, String> {
    let mut manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    let user = manager.register(&username, &password).map_err(|e| e.to_string())?;
    Ok(UserInfo {
        id: user.id,
        username: user.username,
        first_name: user.first_name,
        last_name: user.last_name,
        role: user.role,
        location: user.location,
    })
}

#[tauri::command]
async fn auth_login(
    username: String,
    password: String,
    auth_state: State<'_, AuthState>,
) -> Result<UserInfo, String> {
    let mut manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    let user = manager.login(&username, &password).map_err(|e| e.to_string())?;
    Ok(UserInfo {
        id: user.id,
        username: user.username,
        first_name: user.first_name,
        last_name: user.last_name,
        role: user.role,
        location: user.location,
    })
}

#[tauri::command]
async fn auth_logout(auth_state: State<'_, AuthState>) -> Result<(), String> {
    let mut manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.logout();
    Ok(())
}

#[tauri::command]
async fn auth_current_user(auth_state: State<'_, AuthState>) -> Result<Option<UserInfo>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    Ok(manager.current_user().map(|u| UserInfo {
        id: u.id,
        username: u.username.clone(),
        first_name: u.first_name.clone(),
        last_name: u.last_name.clone(),
        role: u.role.clone(),
        location: u.location.clone(),
    }))
}

#[tauri::command]
async fn auth_is_logged_in(auth_state: State<'_, AuthState>) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    Ok(manager.is_logged_in())
}

#[tauri::command]
async fn get_user_profile(auth_state: State<'_, AuthState>) -> Result<UserProfile, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_user_profile().map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_user_profile(
    first_name: Option<String>,
    last_name: Option<String>,
    role: Option<String>,
    location: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<UserInfo, String> {
    let mut manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    let profile = UserProfile {
        first_name,
        last_name,
        role,
        location,
    };
    let user = manager.update_user_profile(profile).map_err(|e| e.to_string())?;
    Ok(UserInfo {
        id: user.id,
        username: user.username,
        first_name: user.first_name,
        last_name: user.last_name,
        role: user.role,
        location: user.location,
    })
}

// ------------------------------------------------------------------
// 3b. Secure API Key Commands
// ------------------------------------------------------------------

#[tauri::command]
async fn store_provider_key(
    provider: String,
    api_key: String,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    let provider_enum = Provider::from_str(&provider)
        .ok_or_else(|| format!("Unknown provider: {}", provider))?;
    manager.store_api_key(provider_enum, &api_key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_provider_key(
    provider: String,
    auth_state: State<'_, AuthState>,
) -> Result<Option<String>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    let provider_enum = Provider::from_str(&provider)
        .ok_or_else(|| format!("Unknown provider: {}", provider))?;
    manager.get_api_key(provider_enum).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_provider_key(
    provider: String,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    let provider_enum = Provider::from_str(&provider)
        .ok_or_else(|| format!("Unknown provider: {}", provider))?;
    manager.delete_api_key(provider_enum).map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_provider_keys(auth_state: State<'_, AuthState>) -> Result<Vec<ApiKeyEntry>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_api_keys().map_err(|e| e.to_string())
}

// ------------------------------------------------------------------
// 3c. Brief & Conversation Commands
// ------------------------------------------------------------------

#[tauri::command]
async fn save_brief(
    company: String,
    model: String,
    manifest_name: Option<String>,
    content: String,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .save_brief(&company, &model, manifest_name.as_deref(), &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_briefs(auth_state: State<'_, AuthState>) -> Result<Vec<BriefSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_briefs().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_brief(brief_id: i64, auth_state: State<'_, AuthState>) -> Result<Option<Brief>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_brief(brief_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_brief(brief_id: i64, auth_state: State<'_, AuthState>) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.delete_brief(brief_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_conversation_message(
    brief_id: i64,
    role: String,
    content: String,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .add_conversation_message(brief_id, &role, &content)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_conversation(
    brief_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Vec<ConversationMessage>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_conversation(brief_id).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------
// 3d. Custom Provider Commands
// ------------------------------------------------------------------

#[tauri::command]
async fn add_custom_provider(
    name: String,
    endpoint_url: String,
    model_id: String,
    api_key_header: String,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .add_custom_provider(&name, &endpoint_url, &model_id, &api_key_header)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_custom_providers(auth_state: State<'_, AuthState>) -> Result<Vec<CustomProviderSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_custom_providers().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_custom_provider(
    provider_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Option<CustomProvider>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_custom_provider(provider_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_custom_provider_by_key(
    provider_key: String,
    auth_state: State<'_, AuthState>,
) -> Result<Option<CustomProvider>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_custom_provider_by_key(&provider_key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_custom_provider(
    provider_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.delete_custom_provider(provider_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn store_custom_provider_key(
    provider_key: String,
    api_key: String,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.store_custom_api_key(&provider_key, &api_key).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_custom_provider_api_key(
    provider_key: String,
    auth_state: State<'_, AuthState>,
) -> Result<Option<String>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_custom_api_key(&provider_key).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------
// 3e. Research Session Commands
// ------------------------------------------------------------------

#[tauri::command]
async fn create_research_session(
    company: String,
    model: String,
    manifest_name: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .create_research_session(&company, &model, manifest_name.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_research_session(
    session_id: i64,
    status: String,
    current_phase_id: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .update_research_session(session_id, &status, current_phase_id.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_research_sessions(auth_state: State<'_, AuthState>) -> Result<Vec<ResearchSessionSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_research_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_research_session(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Option<ResearchSession>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_research_session(session_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_research_session(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.delete_research_session(session_id).map_err(|e| e.to_string())
}

/// Rename a research session (update the company/name field)
#[tauri::command]
async fn rename_research_session(
    session_id: i64,
    new_name: String,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.rename_research_session(session_id, &new_name).map_err(|e| e.to_string())
}

/// Save or update a phase output (upserts on session_id + phase_id)
/// Extended for user data accessibility (IM-5010): system_prompt and user_input fields
#[tauri::command]
async fn save_phase_output(
    session_id: i64,
    phase_id: String,
    phase_name: String,
    status: String,
    system_prompt: Option<String>,
    user_input: Option<String>,
    output: Option<String>,
    error: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .save_phase_output(
            session_id,
            &phase_id,
            &phase_name,
            &status,
            system_prompt.as_deref(),
            user_input.as_deref(),
            output.as_deref(),
            error.as_deref(),
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_phase_outputs(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Vec<PhaseOutput>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_phase_outputs(session_id).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_last_completed_phase(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Option<PhaseOutput>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_last_completed_phase(session_id).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------
// 3e-bis. Session Conversation Commands (IM-5031, IM-5032)
// ------------------------------------------------------------------

/// Add a message to session conversation history (IM-5031)
#[tauri::command]
async fn add_session_message(
    session_id: i64,
    phase_id: Option<String>,
    role: String,
    content: String,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .add_session_message(session_id, phase_id.as_deref(), &role, &content)
        .map_err(|e| e.to_string())
}

/// Get session conversation history (IM-5032)
#[tauri::command]
async fn get_session_conversation(
    session_id: i64,
    phase_id: Option<String>,
    limit: Option<u32>,
    auth_state: State<'_, AuthState>,
) -> Result<Vec<SessionMessage>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .get_session_conversation(session_id, phase_id.as_deref(), limit)
        .map_err(|e| e.to_string())
}

// ------------------------------------------------------------------
// 3e-ter. Session Resume Command (IM-5020, IM-5021)
// ------------------------------------------------------------------

/// Reconstruct session context from phase outputs for resume (IM-5021)
/// Uses sliding window (default 25 pairs) to manage token limits
fn reconstruct_session_context(
    phase_outputs: &[PhaseOutput],
    max_pairs: usize,
) -> Vec<SessionHistoryMessage> {
    // Filter completed phases with user_input (represents a full conversation turn)
    let pairs: Vec<(SessionHistoryMessage, SessionHistoryMessage)> = phase_outputs
        .iter()
        .filter(|p| p.status == "completed" && p.user_input.is_some() && p.output.is_some())
        .map(|p| (
            SessionHistoryMessage {
                role: "user".to_string(),
                content: p.user_input.clone().unwrap_or_default(),
            },
            SessionHistoryMessage {
                role: "assistant".to_string(),
                content: p.output.clone().unwrap_or_default(),
            },
        ))
        .collect();

    // Apply sliding window - keep most recent pairs
    let window_start = pairs.len().saturating_sub(max_pairs);
    pairs[window_start..]
        .iter()
        .flat_map(|(user, assistant)| vec![user.clone(), assistant.clone()])
        .collect()
}

/// Resume a paused research session with full context (IM-5020)
/// Requires: IM-5011 (get_phase_outputs), IM-5021 (reconstruct_session_context)
#[tauri::command]
async fn resume_research_session(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<ResumeSessionResult, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;

    // 1. Load session
    let session = manager.get_research_session(session_id)
        .map_err(|e| format!("Session {} not found: {}", session_id, e))?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    // 2. Validate status
    match session.status.as_str() {
        "completed" => return Err(format!("Session {} already completed", session_id)),
        "failed" => return Err(format!("Session {} failed and cannot be resumed", session_id)),
        "in_progress" | "paused" => {}, // OK to resume
        _ => return Err(format!("Session {} has invalid status: {}", session_id, session.status)),
    }

    // 3. Load phase outputs
    let phase_outputs = manager.get_phase_outputs(session_id)
        .map_err(|e| e.to_string())?;

    // 4. Find completed phases
    let completed_phases: Vec<_> = phase_outputs.iter()
        .filter(|p| p.status == "completed")
        .collect();

    if completed_phases.is_empty() {
        return Err(format!("Session {} has no completed phases to resume from", session_id));
    }

    let last_completed = completed_phases.last().unwrap();

    // 5. Determine next phase (simple increment for now - manifest-specific logic can be added)
    let next_phase_id = determine_next_phase(&last_completed.phase_id);

    // 6. Reconstruct context with sliding window (25 pairs default)
    let history = reconstruct_session_context(&phase_outputs, 25);

    // 7. Count total phases (from manifest would be better, but we use phase_outputs as proxy)
    let total_phases = phase_outputs.iter()
        .map(|p| &p.phase_id)
        .collect::<std::collections::HashSet<_>>()
        .len()
        .max(7); // Default minimum of 7 phases

    Ok(ResumeSessionResult {
        session,
        next_phase_id,
        context: SessionContext {
            history,
            last_completed_phase: last_completed.phase_id.clone(),
            total_phases,
            completed_phases: completed_phases.len(),
        },
    })
}

/// Determine the next phase ID after a completed phase
/// This is a simple implementation - could be enhanced with manifest phase order
fn determine_next_phase(last_phase_id: &str) -> String {
    // Parse phase number from ID (e.g., "phase_1" -> 1)
    if let Some(num_str) = last_phase_id.strip_prefix("phase_") {
        if let Ok(num) = num_str.parse::<i32>() {
            return format!("phase_{}", num + 1);
        }
    }
    // Fallback: append "_next" if we can't parse
    format!("{}_next", last_phase_id)
}

// ------------------------------------------------------------------
// 3e-quater. Project Management Commands
// ------------------------------------------------------------------

/// Create a new research project
#[tauri::command]
async fn create_project(
    name: String,
    description: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<i64, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .create_project(&name, description.as_deref())
        .map_err(|e| e.to_string())
}

/// List all projects for the current user
#[tauri::command]
async fn list_projects(auth_state: State<'_, AuthState>) -> Result<Vec<ProjectSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_projects().map_err(|e| e.to_string())
}

/// Get a specific project by ID
#[tauri::command]
async fn get_project(
    project_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Option<Project>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_project(project_id).map_err(|e| e.to_string())
}

/// Update a project's name and/or description (discards bool result)
#[tauri::command]
async fn update_project(
    project_id: i64,
    name: String,
    description: Option<String>,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .update_project(project_id, &name, description.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a project (cascades to remove project-session associations)
#[tauri::command]
async fn delete_project(
    project_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.delete_project(project_id).map_err(|e| e.to_string())
}

// =====================================================
// ARCHIVE COMMANDS
// =====================================================

/// Archive a project (soft delete - hides from main list)
#[tauri::command]
async fn archive_project(
    project_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.archive_project(project_id).map_err(|e| e.to_string())
}

/// Unarchive a project (restore to main list)
#[tauri::command]
async fn unarchive_project(
    project_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.unarchive_project(project_id).map_err(|e| e.to_string())
}

/// Archive a research session (soft delete - hides from main list)
#[tauri::command]
async fn archive_session(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.archive_session(session_id).map_err(|e| e.to_string())
}

/// Unarchive a research session (restore to main list)
#[tauri::command]
async fn unarchive_session(
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.unarchive_session(session_id).map_err(|e| e.to_string())
}

/// List archived projects (for the Archived section in sidebar)
#[tauri::command]
async fn list_archived_projects(
    auth_state: State<'_, AuthState>,
) -> Result<Vec<auth::ProjectSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_archived_projects().map_err(|e| e.to_string())
}

/// List archived research sessions (for the Archived section in sidebar)
#[tauri::command]
async fn list_archived_sessions(
    auth_state: State<'_, AuthState>,
) -> Result<Vec<auth::ResearchSessionSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.list_archived_sessions().map_err(|e| e.to_string())
}

/// Add a session to a project
#[tauri::command]
async fn add_session_to_project(
    project_id: i64,
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .add_session_to_project(project_id, session_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Remove a session from a project
#[tauri::command]
async fn remove_session_from_project(
    project_id: i64,
    session_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<(), String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager
        .remove_session_from_project(project_id, session_id)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Get all sessions in a project
#[tauri::command]
async fn get_project_sessions(
    project_id: i64,
    auth_state: State<'_, AuthState>,
) -> Result<Vec<ResearchSessionSummary>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    manager.get_project_sessions(project_id).map_err(|e| e.to_string())
}

// ------------------------------------------------------------------
// 3f. Document Export Commands
// ------------------------------------------------------------------

/// Get the base documents directory for research outputs
/// Creates the directory structure if it doesn't exist
#[tauri::command]
async fn get_documents_base_path(app: AppHandle) -> Result<String, String> {
    let doc_dir = app
        .path()
        .document_dir()
        .map_err(|e| format!("Failed to get documents directory: {}", e))?;

    let research_dir = doc_dir.join("Fullintel Research");

    if !research_dir.exists() {
        fs::create_dir_all(&research_dir)
            .map_err(|e| format!("Failed to create research directory: {}", e))?;
    }

    research_dir.to_string_lossy().to_string().pipe(Ok)
}

/// Helper trait for pipe-style programming
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R where F: FnOnce(Self) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

/// Export a single phase output as a Markdown file
/// Directory structure: base/subject/manifest-name/phase-name.md
#[tauri::command]
async fn export_phase_as_markdown(
    session_id: i64,
    phase_id: String,
    auth_state: State<'_, AuthState>,
    app: AppHandle,
) -> Result<String, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;

    // Get session info for directory naming
    let session = manager.get_research_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    // Get phase outputs
    let phase_outputs = manager.get_phase_outputs(session_id)
        .map_err(|e| e.to_string())?;

    // Find the specific phase
    let phase = phase_outputs.iter()
        .find(|p| p.phase_id == phase_id)
        .ok_or_else(|| format!("Phase {} not found in session {}", phase_id, session_id))?;

    // Get base documents path
    let doc_dir = app.path()
        .document_dir()
        .map_err(|e| format!("Failed to get documents directory: {}", e))?;

    // Build directory path: base/subject/manifest-name/
    let safe_company = sanitize_filename(&session.company);
    let safe_manifest = session.manifest_name
        .as_ref()
        .map(|m| sanitize_filename(m))
        .unwrap_or_else(|| "default-manifest".to_string());

    let output_dir = doc_dir
        .join("Fullintel Research")
        .join(&safe_company)
        .join(&safe_manifest);

    // Create directory structure
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Build filename from phase name
    let safe_phase_name = sanitize_filename(&phase.phase_name);
    let filename = format!("{}.md", safe_phase_name);
    let file_path = output_dir.join(&filename);

    // Build markdown content
    let content = format_phase_as_markdown(&session, phase);

    // Write the file
    fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Export entire research session as a combined Markdown document
#[tauri::command]
async fn export_session_as_markdown(
    session_id: i64,
    auth_state: State<'_, AuthState>,
    app: AppHandle,
) -> Result<String, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;

    // Get session info
    let session = manager.get_research_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    // Get all phase outputs
    let phase_outputs = manager.get_phase_outputs(session_id)
        .map_err(|e| e.to_string())?;

    // Get base documents path
    let doc_dir = app.path()
        .document_dir()
        .map_err(|e| format!("Failed to get documents directory: {}", e))?;

    // Build directory path
    let safe_company = sanitize_filename(&session.company);
    let safe_manifest = session.manifest_name
        .as_ref()
        .map(|m| sanitize_filename(m))
        .unwrap_or_else(|| "default-manifest".to_string());

    let output_dir = doc_dir
        .join("Fullintel Research")
        .join(&safe_company)
        .join(&safe_manifest);

    // Create directory structure
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    // Build combined filename with timestamp
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M");
    let filename = format!("Full-Report_{}.md", timestamp);
    let file_path = output_dir.join(&filename);

    // Build combined markdown content
    let mut content = format!(
        "# Research Report: {}\n\n\
         **Generated:** {}\n\
         **Model:** {}\n\
         **Manifest:** {}\n\n\
         ---\n\n",
        session.company,
        chrono::Local::now().format("%B %d, %Y at %H:%M"),
        session.model,
        session.manifest_name.as_deref().unwrap_or("Default")
    );

    // Add each completed phase
    for phase in phase_outputs.iter().filter(|p| p.status == "completed") {
        content.push_str(&format!(
            "## {}\n\n{}\n\n---\n\n",
            phase.phase_name,
            phase.output.as_deref().unwrap_or("*No output*")
        ));
    }

    // Write the file
    fs::write(&file_path, &content)
        .map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}

/// Export all phases as individual Markdown files
#[tauri::command]
async fn export_all_phases_as_markdown(
    session_id: i64,
    auth_state: State<'_, AuthState>,
    app: AppHandle,
) -> Result<Vec<String>, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;

    // Get session info
    let session = manager.get_research_session(session_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Session {} not found", session_id))?;

    // Get all phase outputs
    let phase_outputs = manager.get_phase_outputs(session_id)
        .map_err(|e| e.to_string())?;

    // Get base documents path
    let doc_dir = app.path()
        .document_dir()
        .map_err(|e| format!("Failed to get documents directory: {}", e))?;

    // Build directory path
    let safe_company = sanitize_filename(&session.company);
    let safe_manifest = session.manifest_name
        .as_ref()
        .map(|m| sanitize_filename(m))
        .unwrap_or_else(|| "default-manifest".to_string());

    let output_dir = doc_dir
        .join("Fullintel Research")
        .join(&safe_company)
        .join(&safe_manifest);

    // Create directory structure
    if !output_dir.exists() {
        fs::create_dir_all(&output_dir)
            .map_err(|e| format!("Failed to create output directory: {}", e))?;
    }

    let mut saved_files = Vec::new();

    // Export each completed phase
    for (idx, phase) in phase_outputs.iter().filter(|p| p.status == "completed").enumerate() {
        let safe_phase_name = sanitize_filename(&phase.phase_name);
        let filename = format!("{:02}_{}.md", idx + 1, safe_phase_name);
        let file_path = output_dir.join(&filename);

        let content = format_phase_as_markdown(&session, phase);

        fs::write(&file_path, &content)
            .map_err(|e| format!("Failed to write {}: {}", filename, e))?;

        saved_files.push(file_path.to_string_lossy().to_string());
    }

    Ok(saved_files)
}

/// Sanitize a string to be safe for use as a filename
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Format a phase output as Markdown
fn format_phase_as_markdown(session: &ResearchSession, phase: &PhaseOutput) -> String {
    format!(
        "# {}\n\n\
         **Subject:** {}\n\
         **Phase:** {}\n\
         **Status:** {}\n\
         **Generated:** {}\n\n\
         ---\n\n\
         {}\n",
        phase.phase_name,
        session.company,
        phase.phase_id,
        phase.status,
        chrono::Local::now().format("%B %d, %Y at %H:%M"),
        phase.output.as_deref().unwrap_or("*No output available*")
    )
}

// ------------------------------------------------------------------
// 3g. Legacy Config Commands
// ------------------------------------------------------------------

#[tauri::command]
async fn set_api_key(key: String, state: State<'_, AppState>) -> Result<(), String> {
    // 1. Trim whitespace from API key (common copy-paste issue)
    let trimmed_key = key.trim().to_string();

    // 2. Debug log key length (NOT the key itself for security)
    println!("[DEBUG] API key received, length: {} chars", trimmed_key.len());

    // 3. Update Memory
    {
        let mut config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config.api_key = Some(trimmed_key);
    }

    // 4. Persist to Disk
    state.save()?;

    Ok(())
}

#[tauri::command]
async fn get_app_state(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|_| "Failed to lock state")?;
    Ok(config.clone())
}

#[tauri::command]
async fn set_manifest_path(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("Manifest file not found: {}", path));
    }
    {
        let mut config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config.last_manifest_path = Some(path_buf);
    }
    state.save()?;
    Ok(())
}

#[tauri::command]
async fn get_manifest_phases(manifest_path: Option<String>, state: State<'_, AppState>) -> Result<Vec<PhaseInfo>, String> {
    // Use provided path or fall back to saved path
    let path = if let Some(p) = manifest_path {
        PathBuf::from(p)
    } else {
        let config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config.last_manifest_path.clone().ok_or("No manifest path set")?
    };

    if !path.exists() {
        return Err(format!("Manifest not found: {:?}", path));
    }

    let manifest = Manifest::load_from_file(&path).map_err(|e| e.to_string())?;

    let phases: Vec<PhaseInfo> = manifest.phases.iter().map(|p| PhaseInfo {
        id: p.id.clone(),
        name: p.name.clone(),
    }).collect();

    Ok(phases)
}

/// Manifest info returned to frontend - contains name, description, and input label
#[derive(Debug, Clone, serde::Serialize)]
struct ManifestInfo {
    id: String,
    version: String,
    name: String,
    description: String,
    input_label: Option<String>,
}

#[tauri::command]
async fn get_manifest_info(manifest_path: Option<String>, state: State<'_, AppState>) -> Result<ManifestInfo, String> {
    // Use provided path or fall back to saved path
    let path = if let Some(p) = manifest_path {
        PathBuf::from(p)
    } else {
        let config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config.last_manifest_path.clone().ok_or("No manifest path set")?
    };

    if !path.exists() {
        return Err(format!("Manifest not found: {:?}", path));
    }

    let manifest = Manifest::load_from_file(&path).map_err(|e| e.to_string())?;

    Ok(ManifestInfo {
        id: manifest.manifest.id,
        version: manifest.manifest.version,
        name: manifest.manifest.name,
        description: manifest.manifest.description,
        input_label: manifest.manifest.input_label,
    })
}

#[tauri::command]
async fn get_saved_manifests(state: State<'_, AppState>) -> Result<Vec<SavedManifest>, String> {
    let config = state.config.lock().map_err(|_| "Failed to lock state")?;
    Ok(config.saved_manifests.clone())
}

#[tauri::command]
async fn save_manifest_to_list(name: String, path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("Manifest file not found: {}", path));
    }

    {
        let mut config = state.config.lock().map_err(|_| "Failed to lock state")?;
        // Find existing manifest by path and update name, or add new
        if let Some(existing) = config.saved_manifests.iter_mut().find(|m| m.path == path_buf) {
            existing.name = name;
        } else {
            config.saved_manifests.push(SavedManifest { name, path: path_buf });
        }
    }
    state.save()?;
    Ok(())
}

#[tauri::command]
async fn remove_saved_manifest(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    {
        let mut config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config.saved_manifests.retain(|m| m.path != path_buf);
    }
    state.save()?;
    Ok(())
}

// ------------------------------------------------------------------
// 3g. Manifest Content Management Commands
// ------------------------------------------------------------------

/// Load raw manifest file content for editing
#[tauri::command]
async fn load_manifest_file(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("Manifest file not found: {}", path));
    }
    fs::read_to_string(&path_buf).map_err(|e| format!("Failed to read manifest file: {}", e))
}

/// Validate manifest YAML content and return phases if valid
#[tauri::command]
async fn validate_manifest(content: String) -> Result<Vec<PhaseInfo>, String> {
    // Try to parse the YAML content
    let manifest: Manifest = serde_yaml::from_str(&content)
        .map_err(|e| format!("Invalid YAML: {}", e))?;

    // Return phase info
    let phases: Vec<PhaseInfo> = manifest.phases.iter().map(|p| PhaseInfo {
        id: p.id.clone(),
        name: p.name.clone(),
    }).collect();

    Ok(phases)
}

/// Get the input_label from a manifest file (for dynamic placeholder text)
#[tauri::command]
async fn get_manifest_input_label(path: String) -> Result<Option<String>, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path_buf)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    let manifest: Manifest = serde_yaml::from_str(&content)
        .map_err(|e| format!("Invalid manifest YAML: {}", e))?;

    Ok(manifest.manifest.input_label)
}

/// Get the name from a manifest file (from manifest.name field in YAML)
#[tauri::command]
async fn get_manifest_name(path: String) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        // Fallback to filename if file doesn't exist
        let filename = path_buf.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        return Ok(filename);
    }

    let content = fs::read_to_string(&path_buf)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;

    let manifest: Manifest = serde_yaml::from_str(&content)
        .map_err(|e| format!("Invalid manifest YAML: {}", e))?;

    Ok(manifest.manifest.name)
}

/// Save manifest content to file
#[tauri::command]
async fn save_manifest_file(path: String, content: String) -> Result<(), String> {
    // First validate the content
    let _manifest: Manifest = serde_yaml::from_str(&content)
        .map_err(|e| format!("Invalid YAML - cannot save: {}", e))?;

    // Create parent directories if needed
    let path_buf = PathBuf::from(&path);
    if let Some(parent) = path_buf.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // Write the file
    fs::write(&path_buf, &content)
        .map_err(|e| format!("Failed to write manifest file: {}", e))?;

    Ok(())
}

/// Get a default manifest template for creating new manifests
#[tauri::command]
async fn get_default_manifest_template() -> Result<String, String> {
    Ok(r#"manifest:
  id: "NEW-MANIFEST-001"
  version: "1.0.0"
  name: "New Research Protocol"
  description: "Describe your research workflow here."

# ------------------------------------------------------------------
# DATA SCHEMAS (Define the shape of your data)
# ------------------------------------------------------------------
schemas:
  OutputSchema:
    fields:
      - name: summary
      - name: key_findings

# ------------------------------------------------------------------
# EXECUTION PHASES (Your workflow steps)
# ------------------------------------------------------------------
phases:
  - id: "PHASE-01-RESEARCH"
    name: "Initial Research"
    tools: ["search_tool"]
    input: "target_company"
    instructions: |
      Research the target and gather key information.
      Focus on:
      1. Overview and background
      2. Recent news and events
      3. Key stakeholders
    output_schema: "OutputSchema"

  - id: "PHASE-02-ANALYSIS"
    name: "Analysis"
    dependencies: ["PHASE-01-RESEARCH"]
    instructions: |
      Analyze the research findings.
      Identify patterns and insights.
    output_target: "analysis_output"

  - id: "PHASE-03-OUTPUT"
    name: "Generate Output"
    dependencies: ["ALL"]
    instructions: |
      Synthesize all previous phases into a comprehensive output.
    output_target: "markdown_file"

# ------------------------------------------------------------------
# QUALITY GATES (Optional validation checks)
# ------------------------------------------------------------------
quality_gates:
  - phase: "PHASE-01-RESEARCH"
    check: "Is the research comprehensive?"
    fail_action: "RETRY"
"#.to_string())
}

#[tauri::command]
async fn send_followup(
    question: String,
    context: String,
    model: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let api_key = {
        let config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config
            .api_key
            .clone()
            .ok_or("API Key not set. Please configure in settings.")?
    };

    let mut llm_client = llm::LLMClient::new(api_key);

    let system_prompt = "You are a helpful assistant analyzing business intelligence reports. \
        The user has generated a research report and wants to ask follow-up questions. \
        Use the provided context to give accurate, relevant answers.".to_string();

    let user_prompt = format!(
        "Here is the generated report:\n\n{}\n\n---\n\nUser question: {}",
        context, question
    );

    let req = llm::LLMRequest {
        system: system_prompt,
        user: user_prompt,
        model,
    };

    llm_client.generate(req).await.map_err(|e| e.to_string())
}

/// Run a single phase with custom prompts (IM-5045: Phase Relaunch)
/// Allows users to edit prompts and re-run a specific phase from the session view
#[tauri::command]
async fn run_single_phase(
    session_id: i64,
    phase_id: String,
    phase_name: String,
    system_prompt: String,
    user_input: String,
    model: String,
    state: State<'_, AppState>,
    auth_state: State<'_, AuthState>,
) -> Result<String, String> {
    // 1. Get API key from state
    let api_key = {
        let config = state.config.lock().map_err(|_| "Failed to lock state")?;
        config
            .api_key
            .clone()
            .ok_or("API Key not set. Please configure in settings.")?
    };

    // 2. Mark phase as "running" and save the prompts
    {
        let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
        manager
            .save_phase_output(
                session_id,
                &phase_id,
                &phase_name,
                "running",
                Some(&system_prompt),
                Some(&user_input),
                None, // output will be set on completion
                None, // no error yet
            )
            .map_err(|e| format!("Failed to save running state: {}", e))?;
    }

    // 3. Make the LLM API call
    let mut llm_client = llm::LLMClient::new(api_key);
    let req = llm::LLMRequest {
        system: system_prompt.clone(),
        user: user_input.clone(),
        model,
    };

    let result = llm_client.generate(req).await;

    // 4. Save the result (success or failure)
    match result {
        Ok(output) => {
            // Save successful output
            let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
            manager
                .save_phase_output(
                    session_id,
                    &phase_id,
                    &phase_name,
                    "completed",
                    Some(&system_prompt),
                    Some(&user_input),
                    Some(&output),
                    None,
                )
                .map_err(|e| format!("Failed to save phase output: {}", e))?;
            Ok(output)
        }
        Err(e) => {
            // Save error state
            let error_msg = e.to_string();
            let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
            manager
                .save_phase_output(
                    session_id,
                    &phase_id,
                    &phase_name,
                    "failed",
                    Some(&system_prompt),
                    Some(&user_input),
                    None,
                    Some(&error_msg),
                )
                .map_err(|_| format!("LLM call failed: {}", error_msg))?;
            Err(format!("LLM call failed: {}", error_msg))
        }
    }
}

#[tauri::command]
async fn run_research(
    company: String,
    model: String,
    manifest_path_override: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
    auth_state: State<'_, AuthState>,
) -> Result<String, String> {
    // 1. Retrieve Credentials from State
    let (api_key, manifest_path) = {
        let config = state.config.lock().map_err(|_| "Failed to lock state")?;
        let key = config
            .api_key
            .clone()
            .ok_or("API Key not set. Please configure in settings.")?;

        // Use override if provided, otherwise use saved path
        let path = manifest_path_override
            .map(PathBuf::from)
            .or(config.last_manifest_path.clone())
            .ok_or("Manifest path not found.")?;
        (key, path)
    };

    // 2. Load Manifest (The Brain)
    // We check if the file exists before attempting to load
    if !manifest_path.exists() {
        return Err(format!("Manifest not found at: {:?}", manifest_path));
    }
    let manifest = Manifest::load_from_file(&manifest_path).map_err(|e| e.to_string())?;

    // 3. Create research session for persistence (L1-ARCHITECTURE Section 5.3 requirement)
    // This enables auto-save of every phase completion to SQLite
    let manifest_name = manifest_path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());

    let session_id = {
        let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
        // Only create session if user is logged in
        if manager.is_logged_in() {
            match manager.create_research_session(&company, &model, manifest_name.as_deref()) {
                Ok(id) => {
                    println!("[RESEARCH] Created session {} for company: {}", id, company);
                    Some(id)
                }
                Err(e) => {
                    eprintln!("[RESEARCH] Warning: Could not create session: {}", e);
                    None
                }
            }
        } else {
            println!("[RESEARCH] No user logged in, skipping session persistence");
            None
        }
    };

    // 4. Initialize Agent with AppHandle Emitter and session_id for persistence
    // Using AppHandle instead of Window for global event emission (Tauri 2.0 pattern)
    // The model parameter allows overriding the default model for all phases
    // The session_id enables phase-output events to include session context
    let mut agent = Agent::new(manifest, api_key, Some(app.clone()), Some(model), session_id);

    // 5. Execute Workflow (The Heavy Lifting)
    // This runs the phases defined in the YAML
    let workflow_result = agent.run_workflow(&company).await;

    // 6. Update session status based on workflow result
    if let Some(sid) = session_id {
        let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
        match &workflow_result {
            Ok(_) => {
                let _ = manager.update_research_session(sid, "completed", None);
                println!("[RESEARCH] Session {} marked as completed", sid);
            }
            Err(_) => {
                let _ = manager.update_research_session(sid, "failed", None);
                println!("[RESEARCH] Session {} marked as failed", sid);
            }
        }
    }

    // 7. Return result
    workflow_result.map_err(|e| e.to_string())?;

    // 8. Retrieve Final Artifact
    // We pull the generated markdown from the agent's context blackboard
    // The key "markdown_file" must match the `output_format` or target defined in your manifest Phase 5
    let final_artifact = agent.get_context("markdown_file").unwrap_or_else(|| {
        "Workflow completed, but no final artifact found in context.".to_string()
    });

    Ok(final_artifact)
}

// ------------------------------------------------------------------
// 4. Main Entry Point & Setup
// ------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // A. Resolve Config Path
            // This stores config in standard OS app data locations
            // e.g., C:\Users\You\AppData\Roaming\com.fullintel.agent\config.json
            let app_dir = app
                .path()
                .app_data_dir()
                .expect("failed to get app data dir");
            if !app_dir.exists() {
                fs::create_dir_all(&app_dir).expect("failed to create app data dir");
            }
            let config_path = app_dir.join("config.json");

            // B. Load or Create Config
            let mut config: AppConfig = if config_path.exists() {
                let content = fs::read_to_string(&config_path).unwrap_or_default();
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                AppConfig::default()
            };

            // B2. Resolve default manifest path if not set or invalid
            // This handles both fresh installs and existing configs with broken paths
            let needs_manifest_resolution = config.last_manifest_path.as_ref()
                .map(|p| !p.exists())
                .unwrap_or(true);

            if needs_manifest_resolution {
                if let Some(resolved_path) = resolve_default_manifest_path() {
                    println!("[DEBUG] Resolved default manifest to: {:?}", resolved_path);
                    config.last_manifest_path = Some(resolved_path.clone());

                    // Add to saved manifests if not already present
                    if !config.saved_manifests.iter().any(|m| m.path == resolved_path) {
                        config.saved_manifests.push(SavedManifest {
                            name: "Fullintel Default".to_string(),
                            path: resolved_path,
                        });
                    }

                    // Save the updated config
                    if let Some(parent) = config_path.parent() {
                        if !parent.exists() {
                            let _ = fs::create_dir_all(parent);
                        }
                    }
                    if let Ok(json) = serde_json::to_string_pretty(&config) {
                        let _ = fs::write(&config_path, json);
                    }
                } else {
                    println!("[WARN] Could not resolve default manifest path - user will need to select manually");
                }
            }

            // C. Manage State (Inject into Tauri)
            app.manage(AppState {
                config: Mutex::new(config),
                config_path,
            });

            // D. Initialize Auth Manager
            // User database stored at: app_data/users.db
            let auth_db_path = app_dir.join("users.db");
            let auth_manager = AuthManager::new(&auth_db_path)
                .expect("Failed to initialize auth database");

            app.manage(AuthState {
                manager: Mutex::new(auth_manager),
            });

            println!("[DEBUG] Auth system initialized at: {:?}", auth_db_path);

            // E. Set Window Icon (for dev mode - title bar and taskbar)
            // In dev mode, the tauri.conf.json icon doesn't apply to the running window
            // We manually set it here to ensure the Fi logo appears
            if let Some(window) = app.get_webview_window("main") {
                // Try to load the icon from the icons directory
                // In dev mode, icons are in src-tauri/icons/ relative to CARGO_MANIFEST_DIR
                let icon_path = if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
                    PathBuf::from(manifest_dir).join("icons").join("icon.ico")
                } else {
                    // Fallback: try relative to executable
                    std::env::current_exe()
                        .map(|p| p.parent().unwrap_or(&p).join("icons").join("icon.ico"))
                        .unwrap_or_default()
                };

                if icon_path.exists() {
                    match Image::from_path(&icon_path) {
                        Ok(icon) => {
                            if let Err(e) = window.set_icon(icon) {
                                println!("[WARN] Failed to set window icon: {}", e);
                            } else {
                                println!("[DEBUG] Window icon set successfully from: {:?}", icon_path);
                            }
                        }
                        Err(e) => {
                            println!("[WARN] Failed to load icon image: {}", e);
                        }
                    }
                } else {
                    println!("[WARN] Icon file not found at: {:?}", icon_path);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Authentication commands
            auth_register,
            auth_login,
            auth_logout,
            auth_current_user,
            auth_is_logged_in,
            // User profile commands
            get_user_profile,
            update_user_profile,
            // Secure API key commands
            store_provider_key,
            get_provider_key,
            delete_provider_key,
            list_provider_keys,
            // Brief & conversation commands
            save_brief,
            list_briefs,
            get_brief,
            delete_brief,
            add_conversation_message,
            get_conversation,
            // Custom provider commands
            add_custom_provider,
            list_custom_providers,
            get_custom_provider,
            get_custom_provider_by_key,
            delete_custom_provider,
            store_custom_provider_key,
            get_custom_provider_api_key,
            // Research session commands
            create_research_session,
            update_research_session,
            list_research_sessions,
            get_research_session,
            delete_research_session,
            rename_research_session,
            save_phase_output,
            get_phase_outputs,
            get_last_completed_phase,
            // Session conversation commands (IM-5031, IM-5032)
            add_session_message,
            get_session_conversation,
            // Session resume command (IM-5020)
            resume_research_session,
            // Project management commands
            create_project,
            list_projects,
            get_project,
            update_project,
            delete_project,
            add_session_to_project,
            remove_session_from_project,
            get_project_sessions,
            // Archive commands
            archive_project,
            unarchive_project,
            archive_session,
            unarchive_session,
            list_archived_projects,
            list_archived_sessions,
            // Document export commands
            get_documents_base_path,
            export_phase_as_markdown,
            export_session_as_markdown,
            export_all_phases_as_markdown,
            // Legacy config commands
            set_api_key,
            get_app_state,
            set_manifest_path,
            get_manifest_phases,
            get_manifest_info,
            get_saved_manifests,
            save_manifest_to_list,
            remove_saved_manifest,
            // Manifest content management commands
            load_manifest_file,
            validate_manifest,
            get_manifest_input_label,
            get_manifest_name,
            save_manifest_file,
            get_default_manifest_template,
            send_followup,
            run_single_phase,
            run_research
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
