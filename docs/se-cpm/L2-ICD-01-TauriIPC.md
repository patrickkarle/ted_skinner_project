# L2-ICD-01: Tauri IPC Interface Control Document

**Document ID:** L2-ICD-TAURI-001
**Interface Name:** Frontend ↔ Backend (Tauri IPC)
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L1-SAD-FULLINTEL-001
**Traceability:** L1-SAD-1.1 MO-001 (Time Reduction), MO-002 (Quality Standardization)

---

## 1. Interface Overview

### 1.1 Purpose
Defines the communication contract between the React frontend and Rust backend using Tauri's IPC (Inter-Process Communication) mechanism.

### 1.2 Interface Participants
- **Client Side:** React TypeScript frontend (UI components)
- **Server Side:** Rust backend (AgentOrchestrator, ToolRegistry, LLMClient)
- **Transport:** Tauri IPC (serialize/deserialize via serde_json)

### 1.3 Communication Pattern
- **Async/Await:** Frontend invokes Rust commands asynchronously
- **Event-Driven:** Backend emits progress events to frontend
- **Request/Response:** Most commands follow request→response pattern
- **Streaming:** Progress updates use event streaming

---

## 2. Interface Contract

### 2.1 Command: run_research

**Purpose:** Execute complete 5-phase research workflow for target company

**Direction:** Frontend → Backend

**Rust Signature:**
```rust
#[tauri::command]
async fn run_research(
    company: String,
    window: tauri::Window,
    state: tauri::State<'_, AppState>
) -> Result<ResearchResult, String>
```

**TypeScript Signature:**
```typescript
import { invoke } from '@tauri-apps/api/core';

interface ResearchResult {
  success: boolean;
  markdown_output?: string;
  error?: string;
  session_id: string;
  duration_ms: number;
  cost_usd: number;
}

async function runResearch(company: string): Promise<ResearchResult>
```

**Request Schema:**
```json
{
  "company": "string (required, 1-200 chars)"
}
```

**Response Schema (Success):**
```json
{
  "success": true,
  "markdown_output": "string (FULLINTEL OPPORTUNITY BRIEF format)",
  "session_id": "uuid",
  "duration_ms": "number (workflow execution time)",
  "cost_usd": "number (total API costs)"
}
```

**Response Schema (Error):**
```json
{
  "success": false,
  "error": "string (human-readable error message)",
  "session_id": "uuid",
  "failed_phase": "string (phase_id where failure occurred)",
  "retry_count": "number"
}
```

**Error Conditions:**
| Error Code | Condition | HTTP Equivalent | Recovery Action |
|------------|-----------|-----------------|-----------------|
| INVALID_INPUT | Company name empty or > 200 chars | 400 Bad Request | Show validation error |
| DEPENDENCY_NOT_MET | Phase dependencies unsatisfied | 409 Conflict | Check state, retry |
| QUALITY_GATE_FAILED | Output failed quality validation | 422 Unprocessable | Offer regeneration |
| TOOL_EXECUTION_FAILED | External API error | 502 Bad Gateway | Retry or manual input |
| LLM_FAILURE | LLM API error | 503 Service Unavailable | Try fallback provider |
| STATE_CORRUPTION | Database integrity error | 500 Internal Error | Restore from backup |

**Performance Requirements:**
- **Latency:** < 5 minutes end-to-end (95th percentile)
- **Timeout:** 10 minutes maximum before auto-cancel
- **Progress Updates:** Emit event every 5 seconds minimum

---

### 2.2 Command: get_session_history

**Purpose:** Retrieve list of past research sessions

**Direction:** Frontend → Backend

**Rust Signature:**
```rust
#[tauri::command]
async fn get_session_history(
    limit: Option<usize>,
    state: tauri::State<'_, AppState>
) -> Result<Vec<SessionSummary>, String>
```

**TypeScript Signature:**
```typescript
interface SessionSummary {
  session_id: string;
  company: string;
  created_at: number; // Unix timestamp
  status: 'completed' | 'failed' | 'running';
  duration_ms?: number;
  cost_usd?: number;
}

async function getSessionHistory(limit?: number): Promise<SessionSummary[]>
```

**Request Schema:**
```json
{
  "limit": "number (optional, default 50, max 500)"
}
```

**Response Schema:**
```json
[
  {
    "session_id": "uuid",
    "company": "string",
    "created_at": "number (Unix timestamp ms)",
    "status": "completed | failed | running",
    "duration_ms": "number (optional)",
    "cost_usd": "number (optional)"
  }
]
```

**Performance Requirements:**
- **Latency:** < 100ms
- **Data Size:** Return max 500 sessions

---

### 2.3 Command: get_session_output

**Purpose:** Retrieve markdown output from specific session

**Direction:** Frontend → Backend

**Rust Signature:**
```rust
#[tauri::command]
async fn get_session_output(
    session_id: String,
    state: tauri::State<'_, AppState>
) -> Result<String, String>
```

**TypeScript Signature:**
```typescript
async function getSessionOutput(sessionId: string): Promise<string>
```

**Request Schema:**
```json
{
  "session_id": "uuid (required)"
}
```

**Response Schema (Success):**
```json
"string (markdown content)"
```

**Error Conditions:**
| Error Code | Condition | Recovery Action |
|------------|-----------|-----------------|
| SESSION_NOT_FOUND | Invalid/expired session_id | Show "Session not found" |
| SESSION_INCOMPLETE | Session not completed | Show "Research still running" |

**Performance Requirements:**
- **Latency:** < 50ms

---

### 2.4 Command: export_to_pdf

**Purpose:** Export markdown brief to PDF file

**Direction:** Frontend → Backend

**Rust Signature:**
```rust
#[tauri::command]
async fn export_to_pdf(
    session_id: String,
    output_path: String,
    state: tauri::State<'_, AppState>
) -> Result<String, String>
```

**TypeScript Signature:**
```typescript
async function exportToPdf(
  sessionId: string,
  outputPath: string
): Promise<string> // Returns file path
```

**Request Schema:**
```json
{
  "session_id": "uuid (required)",
  "output_path": "string (optional, defaults to Downloads)"
}
```

**Response Schema (Success):**
```json
"string (absolute path to generated PDF file)"
```

**Performance Requirements:**
- **Latency:** < 3 seconds
- **File Size:** < 5MB typical

---

### 2.5 Command: copy_to_clipboard

**Purpose:** Copy markdown brief to system clipboard

**Direction:** Frontend → Backend

**Rust Signature:**
```rust
#[tauri::command]
async fn copy_to_clipboard(
    session_id: String,
    state: tauri::State<'_, AppState>
) -> Result<(), String>
```

**TypeScript Signature:**
```typescript
async function copyToClipboard(sessionId: string): Promise<void>
```

**Request Schema:**
```json
{
  "session_id": "uuid (required)"
}
```

**Response Schema (Success):**
```json
null
```

**Performance Requirements:**
- **Latency:** < 100ms

---

### 2.6 Command: save_api_keys

**Purpose:** Securely store API keys for LLM providers

**Direction:** Frontend → Backend

**Rust Signature:**
```rust
#[tauri::command]
async fn save_api_keys(
    keys: ApiKeyConfig,
    state: tauri::State<'_, AppState>
) -> Result<(), String>
```

**TypeScript Signature:**
```typescript
interface ApiKeyConfig {
  anthropic?: string;
  google?: string;
  deepseek?: string;
  tavily?: string;
  newsapi?: string;
}

async function saveApiKeys(keys: ApiKeyConfig): Promise<void>
```

**Request Schema:**
```json
{
  "anthropic": "string (optional, Claude API key)",
  "google": "string (optional, Gemini API key)",
  "deepseek": "string (optional, DeepSeek API key)",
  "tavily": "string (optional, Tavily Search API key)",
  "newsapi": "string (optional, NewsAPI.org API key)"
}
```

**Security Requirements:**
- Keys encrypted before storage using OS credential manager
- Never logged or transmitted in plain text
- Validation of key format before storage

**Performance Requirements:**
- **Latency:** < 200ms

---

## 3. Event Emissions (Backend → Frontend)

### 3.1 Event: workflow_started

**Purpose:** Notify frontend that research workflow has begun

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "company": "string",
  "timestamp": "number (Unix timestamp ms)"
}
```

**TypeScript Listener:**
```typescript
import { listen } from '@tauri-apps/api/event';

listen('workflow_started', (event) => {
  const { session_id, company, timestamp } = event.payload;
  // Update UI to show progress screen
});
```

---

### 3.2 Event: phase_started

**Purpose:** Notify frontend that a new phase has begun

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "phase_id": "string (phase_1, phase_2, etc.)",
  "phase_name": "string (human-readable name)",
  "phase_number": "number (1-5)",
  "timestamp": "number"
}
```

**Emission Frequency:** Once per phase (5 times per workflow)

---

### 3.3 Event: phase_progress

**Purpose:** Real-time updates during phase execution

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "phase_id": "string",
  "message": "string (log message, e.g., 'Calling Tavily API...')",
  "progress_percent": "number (0-100, optional)",
  "timestamp": "number"
}
```

**Emission Frequency:** Every 5 seconds during phase execution

---

### 3.4 Event: phase_completed

**Purpose:** Notify frontend that phase finished successfully

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "phase_id": "string",
  "output_preview": "string (first 200 chars of output)",
  "duration_ms": "number",
  "timestamp": "number"
}
```

**Emission Frequency:** Once per phase (5 times per workflow)

---

### 3.5 Event: quality_gate_failed

**Purpose:** Notify frontend that output failed quality validation

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "phase_id": "string",
  "gate_name": "string (e.g., 'no_generic_text')",
  "reason": "string (detailed failure reason)",
  "retry_attempt": "number (1-3)",
  "timestamp": "number"
}
```

**User Action Required:** Show error, offer regeneration

---

### 3.6 Event: workflow_completed

**Purpose:** Notify frontend that entire workflow finished

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "success": "boolean",
  "duration_ms": "number",
  "cost_usd": "number",
  "timestamp": "number"
}
```

---

### 3.7 Event: workflow_error

**Purpose:** Notify frontend of unrecoverable error

**Payload Schema:**
```json
{
  "session_id": "uuid",
  "phase_id": "string (phase where error occurred)",
  "error_type": "string (TOOL_FAILED, LLM_FAILED, etc.)",
  "error_message": "string (user-friendly)",
  "retry_count": "number",
  "timestamp": "number"
}
```

---

## 4. Data Schemas

### 4.1 AppState (Rust Backend)
```rust
pub struct AppState {
    pub orchestrator: Arc<Mutex<AgentOrchestrator>>,
    pub state_manager: Arc<StateManager>,
    pub config: Arc<RwLock<AppConfig>>,
}
```

### 4.2 AppConfig (Rust Backend)
```rust
pub struct AppConfig {
    pub api_keys: ApiKeyConfig,
    pub model_preferences: ModelPreferences,
    pub ui_settings: UiSettings,
}

pub struct ModelPreferences {
    pub phase_1_model: String, // Default: "deepseek-chat"
    pub phase_2_model: String, // Default: "deepseek-chat"
    pub phase_3_model: String, // Default: "deepseek-chat"
    pub phase_4_model: Option<String>, // None (logic-based)
    pub phase_5_model: String, // Default: "claude-3-5-sonnet"
}
```

---

## 5. Error Handling Contract

### 5.1 Error Response Format
All command errors return `Result<T, String>` where `String` contains:
```json
{
  "error_code": "ENUM_VALUE",
  "message": "Human-readable error",
  "details": "Technical details for debugging",
  "recovery_action": "Suggested user action"
}
```

### 5.2 Frontend Error Display
```typescript
try {
  const result = await invoke('run_research', { company: 'TechCorp' });
} catch (error) {
  const err = JSON.parse(error as string);
  showErrorDialog({
    title: err.error_code,
    message: err.message,
    action: err.recovery_action
  });
}
```

---

## 6. Performance Requirements

| Operation | Target Latency | Timeout | Priority |
|-----------|---------------|---------|----------|
| run_research | < 5 min | 10 min | CRITICAL |
| get_session_history | < 100ms | 5s | HIGH |
| get_session_output | < 50ms | 2s | HIGH |
| export_to_pdf | < 3s | 10s | MEDIUM |
| copy_to_clipboard | < 100ms | 1s | MEDIUM |
| save_api_keys | < 200ms | 5s | HIGH |

---

## 7. Security Requirements

### 7.1 Input Validation
- **Company name:** Sanitize special characters, max 200 chars
- **Session ID:** Validate UUID format
- **File paths:** Prevent directory traversal attacks
- **API keys:** Validate format before storage

### 7.2 Output Sanitization
- Markdown content: Escape HTML if rendering in webview
- File paths: Normalize and validate before operations
- Error messages: Never expose internal paths or sensitive data

---

## 8. Testing Requirements

### 8.1 Integration Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_research_success() {
        // Mock AppState
        // Invoke run_research command
        // Assert success response with valid markdown
    }

    #[tokio::test]
    async fn test_run_research_invalid_input() {
        // Invoke with empty company name
        // Assert INVALID_INPUT error
    }

    #[tokio::test]
    async fn test_quality_gate_failure() {
        // Mock LLM to return generic text
        // Assert QUALITY_GATE_FAILED error
    }
}
```

### 8.2 Event Emission Tests
```rust
#[tokio::test]
async fn test_progress_events_emitted() {
    // Set up event listener
    // Run research
    // Assert workflow_started, phase_started (x5), workflow_completed events
}
```

---

## 9. Traceability Matrix

| L1 Requirement | Interface Element | Validation |
|----------------|------------------|------------|
| MO-001: Time < 5 min | run_research timeout | Performance test |
| MO-002: Quality gates | quality_gate_failed event | Unit test |
| SR-005: Export | export_to_pdf, copy_to_clipboard | Integration test |
| SR-007: API security | save_api_keys encryption | Security test |
| SR-009: Crash recovery | get_session_history | E2E test |

---

**Document Status:** Complete - Ready for L3-CDD Implementation
**Next Document:** L2-ICD-02-LLMClient.md
