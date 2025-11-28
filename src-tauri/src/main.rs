// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod agent;
mod auth;
mod llm;
mod manifest;

use agent::Agent;
use auth::{AuthManager, Provider, ApiKeyEntry};
use manifest::Manifest;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

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
            // Default path relative to where the app is run (dev mode)
            // In production, you'd likely bundle this differently.
            last_manifest_path: Some(PathBuf::from(
                "../manifests/fullintel_process_manifest.yaml",
            )),
            saved_manifests: vec![
                SavedManifest {
                    name: "Fullintel Default".to_string(),
                    path: PathBuf::from("../manifests/fullintel_process_manifest.yaml"),
                }
            ],
        }
    }
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
    }))
}

#[tauri::command]
async fn auth_is_logged_in(auth_state: State<'_, AuthState>) -> Result<bool, String> {
    let manager = auth_state.manager.lock().map_err(|_| "Failed to lock auth state")?;
    Ok(manager.is_logged_in())
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
// 3c. Legacy Config Commands
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
        // Check if already saved (by path)
        if !config.saved_manifests.iter().any(|m| m.path == path_buf) {
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

#[tauri::command]
async fn run_research(
    company: String,
    model: String,
    manifest_path_override: Option<String>,
    app: AppHandle,
    state: State<'_, AppState>,
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

    // 3. Initialize Agent with AppHandle Emitter
    // Using AppHandle instead of Window for global event emission (Tauri 2.0 pattern)
    // The model parameter allows overriding the default model for all phases
    let mut agent = Agent::new(manifest, api_key, Some(app), Some(model));

    // 4. Execute Workflow (The Heavy Lifting)
    // This runs the phases defined in the YAML
    agent
        .run_workflow(&company)
        .await
        .map_err(|e| e.to_string())?;

    // 5. Retrieve Final Artifact
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
            let config = if config_path.exists() {
                let content = fs::read_to_string(&config_path).unwrap_or_default();
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                AppConfig::default()
            };

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

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Authentication commands
            auth_register,
            auth_login,
            auth_logout,
            auth_current_user,
            auth_is_logged_in,
            // Secure API key commands
            store_provider_key,
            get_provider_key,
            delete_provider_key,
            list_provider_keys,
            // Legacy config commands
            set_api_key,
            get_app_state,
            set_manifest_path,
            get_manifest_phases,
            get_saved_manifests,
            save_manifest_to_list,
            remove_saved_manifest,
            send_followup,
            run_research
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
