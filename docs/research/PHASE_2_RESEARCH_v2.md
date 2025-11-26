# PHASE 2: RESEARCH - Findings & Validation

**Document Classification**: DOC-RESEARCH-001
**Project:** Fullintel Sales Intelligence Generator
**Date:** 2025-11-19
**Status:** Completed (Remediated)
**Taxonomy Version:** 3.0
**Parent Document:** DOC-ULTRATHINK-001
**Traceability:** L1-SAD-1.1 MO-001 (Time Reduction), MO-002 (Quality Standardization)

---

## Document Metadata

**Component-IDs Referenced:**
- RESEARCH-001-IMPL-001: Rust main.rs Tauri commands
- RESEARCH-001-IMPL-002: Agent workflow execution
- RESEARCH-001-IMPL-003: LLM client multi-provider implementation
- RESEARCH-001-IMPL-004: YAML manifest parser
- RESEARCH-001-INTEG-001: Tauri IPC layer
- RESEARCH-001-INTEG-002: LLM provider routing
- RESEARCH-001-API-001: Anthropic Messages API
- RESEARCH-001-API-002: Google Gemini API
- RESEARCH-001-API-003: DeepSeek Chat API

**Technical Tags Applied:**
- @taxonomy CSE-05 (Control Structures - async workflow execution)
- @taxonomy FOC-22 (Async Functions - Tauri commands)
- @taxonomy FOC-06 (Classes/Structs - Agent, LLMClient, Manifest)
- @taxonomy DMC-05 (Hash Maps - context storage, phase statuses)
- @taxonomy SRC-02 (Encryption - API key storage)
- @taxonomy TVC-01 (Unit Testing - manifest.rs tests)

---

## 1. Current Project Inventory

### 1.1 File Structure Analysis (VERIFIED)

```
ted_skinner_project/
├── src-tauri/                    # Rust backend (Tauri v2.0)
│   ├── src/
│   │   ├── main.rs               ✅ VERIFIED: 167 lines, 3 Tauri commands
│   │   ├── manifest.rs           ✅ VERIFIED: 135 lines, YAML parser + unit tests
│   │   ├── agent.rs              ✅ VERIFIED: 167 lines, workflow orchestration
│   │   └── llm.rs                ✅ VERIFIED: 192 lines, multi-provider client
│   ├── Cargo.toml                ✅ VERIFIED: 27 lines, complete dependencies
│   ├── build.rs                  ✅ Standard Tauri build script
│   └── tauri.conf.json           ⚠️  Requires CSP headers update
├── src/                          ❌ MISSING: React components needed
├── manifests/
│   └── fullintel_process_manifest.yaml  ✅ Complete 5-phase workflow definition
├── package.json                  ✅ Complete Vite + React + Tauri
├── vite.config.ts                ✅ Complete
└── docs/                         🆕 SE-CPM documentation directory
    ├── research/                 ├── PHASE_1_ULTRATHINK.md
    │                             ├── PHASE_2_RESEARCH.md (original - Fantasy Land)
    │                             └── PHASE_2_RESEARCH_v2.md (this document - VERIFIED)
    └── se-cpm/                   ├── L1-SAD-1.1-MissionIntent.md
                                  ├── L2-ICD-01-TauriIPC.md
                                  └── [other SE-CPM docs]
```

### 1.2 Rust Backend Analysis (ACTUAL CODE EXTRACTION)

#### 1.2.1 Main Entry Point (main.rs)

**RESEARCH-001-IMPL-001: Tauri Command Implementations**

**Verified Tauri Commands** (3 total):

```rust
// Command 1: API Key Configuration
#[tauri::command]
async fn set_api_key(key: String, state: State<'_, AppState>) -> Result<(), String>
```
- **Purpose**: Persist API key to disk (`app_data/config.json`)
- **State Management**: Mutex-protected AppConfig
- **Persistence**: Automatic save after update
- **Error Handling**: String-based error propagation

```rust
// Command 2: Application State Retrieval
#[tauri::command]
async fn get_app_state(state: State<'_, AppState>) -> Result<AppConfig, String>
```
- **Purpose**: Retrieve current API key and manifest path
- **Return Type**: AppConfig struct with Optional fields
- **Use Case**: Settings page initialization

```rust
// Command 3: Research Workflow Execution
#[tauri::command]
async fn run_research(
    company: String,
    window: Window,
    state: State<'_, AppState>
) -> Result<String, String>
```
- **Purpose**: Execute complete 5-phase workflow
- **Input**: Company name (String)
- **Output**: Markdown brief (String) or error
- **Event Emission**: Uses `window` for real-time progress
- **State Access**: Retrieves API key and manifest path
- **Core Logic**:
  1. Lock state, extract credentials
  2. Load YAML manifest from filesystem
  3. Initialize Agent with window emitter
  4. Execute `agent.run_workflow(&company).await`
  5. Extract "markdown_file" from context blackboard
  6. Return final artifact or error

**AppState Structure**:
```rust
struct AppState {
    config: Mutex<AppConfig>,      // Thread-safe config
    config_path: PathBuf,           // Persistent storage location
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AppConfig {
    api_key: Option<String>,                    // User-provided LLM key
    last_manifest_path: Option<PathBuf>,        // YAML workflow definition
}
```

**Default Configuration**:
- API Key: None (user must configure)
- Manifest Path: `../manifests/fullintel_process_manifest.yaml` (relative)

**Persistence Mechanism**:
- Location: OS-specific app data directory (`C:\Users\You\AppData\Roaming\com.fullintel.agent\config.json`)
- Format: Pretty-printed JSON
- Auto-creation: Creates directory if missing
- Load Strategy: Loads existing or defaults on first run

---

#### 1.2.2 Agent Orchestration (agent.rs)

**RESEARCH-001-IMPL-002: Workflow Execution Engine**

**Agent Structure**:
```rust
pub struct Agent {
    manifest: Manifest,           // Loaded YAML workflow definition
    state: AgentState,            // Execution context + status tracking
    llm_client: LLMClient,        // Multi-provider LLM interface
    window: Option<Window>,       // Tauri window for event emission
}
```

**AgentState Structure**:
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentState {
    pub current_phase_id: Option<String>,                  // e.g., "phase_1"
    pub phase_statuses: HashMap<String, PhaseStatus>,     // Pending/Running/Completed/Failed
    pub context: HashMap<String, String>,                  // Blackboard pattern (key-value outputs)
    pub logs: Vec<String>,                                 // Sequential log entries
}
```

**PhaseStatus Enum**:
```rust
pub enum PhaseStatus {
    Pending,
    Running,
    Completed,
    Failed(String),    // Contains error message
    Skipped,
}
```

**Core Workflow Method**:
```rust
pub async fn run_workflow(&mut self, initial_input: &str) -> Result<()> {
    // 1. Initialize context with company name
    self.state.context.insert("target_company".to_string(), initial_input.to_string());

    // 2. Clone phases from manifest
    let phases = self.manifest.phases.clone();

    // 3. Sequential phase execution
    for phase in phases {
        self.state.current_phase_id = Some(phase.id.clone());
        self.update_phase_status(&phase.id, PhaseStatus::Running);

        // 4. Execute phase (calls LLM)
        match self.execute_phase(&phase).await {
            Ok(output) => {
                self.log(&format!("Phase {} completed.", phase.name));
                self.update_phase_status(&phase.id, PhaseStatus::Completed);

                // 5. Store output in context (blackboard pattern)
                if let Some(target) = &phase.output_target {
                     self.state.context.insert(target.clone(), output);
                } else if let Some(schema) = &phase.output_schema {
                     self.state.context.insert(schema.clone(), output);
                }
            },
            Err(e) => {
                // 6. Fail-fast on error
                self.log(&format!("Phase {} failed: {}", phase.name, e));
                self.update_phase_status(&phase.id, PhaseStatus::Failed(e.to_string()));
                return Err(e);
            }
        }
    }

    Ok(())
}
```

**Phase Execution Logic**:
```rust
async fn execute_phase(&self, phase: &Phase) -> Result<String> {
    self.log(&format!("Executing Phase: {}", phase.name));

    // 1. Retrieve input from context blackboard
    let input_data = if let Some(input_key) = &phase.input {
        self.state.context.get(input_key)
            .ok_or_else(|| anyhow!("Missing input: {}", input_key))?
            .clone()
    } else {
        serde_json::to_string(&self.state.context)?  // Full context as JSON
    };

    // 2. Build system prompt
    let system_prompt = format!(
        "You are an autonomous research agent executing phase '{}'.\nInstructions:\n{}",
        phase.name,
        phase.instructions
    );

    // 3. Default to Claude Sonnet
    let model = "claude-3-5-sonnet";

    // 4. Execute LLM call
    let req = LLMRequest {
        system: system_prompt,
        user: input_data,
        model: model.to_string(),
    };

    self.llm_client.generate(req).await
}
```

**Event Emission (Frontend Integration)**:
```rust
// Event 1: Log Messages
fn log(&self, msg: &str) {
    println!("[AGENT] {}", msg);  // Console output
    if let Some(window) = &self.window {
        let _ = window.emit("agent-log", LogPayload { message: msg.to_string() });
    }
}

// Event 2: Phase Status Updates
fn update_phase_status(&mut self, phase_id: &str, status: PhaseStatus) {
    self.state.phase_statuses.insert(phase_id.to_string(), status.clone());

    let status_str = match status {
        PhaseStatus::Running => "running",
        PhaseStatus::Completed => "completed",
        PhaseStatus::Failed(_) => "failed",
        _ => "pending",
    };

    if let Some(window) = &self.window {
        let _ = window.emit("phase-update", PhaseUpdatePayload {
            phase_id: phase_id.to_string(),
            status: status_str.to_string()
        });
    }
}
```

**Critical Gap Identified**:
- **Tool Execution Missing**: Lines 126-128 show "REAL IMPLEMENTATION SWITCH" comment
- **Current Behavior**: Relies on LLM knowledge/hallucination for search phases
- **Issue**: No Tavily, NewsAPI, or Apollo.io integration (crates not in Cargo.toml)
- **Workaround**: Demo works by LLM generating plausible but potentially fictional data

---

#### 1.2.3 LLM Client (llm.rs)

**RESEARCH-001-IMPL-003: Multi-Provider LLM Integration**

**Client Structure**:
```rust
#[derive(Debug, Clone)]
pub struct LLMClient {
    client: Client,      // reqwest::Client (HTTP client)
    api_key: String,     // Single API key (provider determined by model prefix)
}
```

**Request Schema**:
```rust
#[derive(Debug, Serialize)]
pub struct LLMRequest {
    pub system: String,   // System prompt
    pub user: String,     // User message
    pub model: String,    // Model identifier (determines provider)
}
```

**Provider Routing Logic**:
```rust
pub async fn generate(&self, req: LLMRequest) -> Result<String> {
    if req.model.starts_with("claude") {
        self.generate_anthropic(req).await        // RESEARCH-001-API-001
    } else if req.model.starts_with("gemini") {
        self.generate_gemini(req).await           // RESEARCH-001-API-002
    } else if req.model.starts_with("deepseek") {
        self.generate_deepseek(req).await         // RESEARCH-001-API-003
    } else {
        Err(anyhow!("Unsupported model: {}", req.model))
    }
}
```

**Provider 1: Anthropic Claude** (RESEARCH-001-API-001)
```rust
async fn generate_anthropic(&self, req: LLMRequest) -> Result<String> {
    let url = "https://api.anthropic.com/v1/messages";

    let body = serde_json::json!({
        "model": req.model,                // e.g., "claude-3-5-sonnet"
        "max_tokens": 4096,                // Fixed token limit
        "system": req.system,              // System prompt
        "messages": [{
            "role": "user",
            "content": req.user            // User message
        }]
    });

    let res = self.client.post(url)
        .header("x-api-key", &self.api_key)              // API key authentication
        .header("anthropic-version", "2023-06-01")       // API version
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("Anthropic API Error: {}", res.text().await?));
    }

    let anthropic_res: AnthropicResponse = res.json().await?;

    // Extract first content block
    anthropic_res.content.first()
        .map(|c| c.text.clone())
        .ok_or_else(|| anyhow!("No content in Anthropic response"))
}
```

**Response Schema (Anthropic)**:
```rust
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}
```

**Provider 2: Google Gemini** (RESEARCH-001-API-002)
```rust
async fn generate_gemini(&self, req: LLMRequest) -> Result<String> {
    // URL includes API key as query parameter
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        req.model, self.api_key
    );

    // Gemini combines system + user into single prompt
    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": format!("System Instruction: {}\n\nUser Request: {}", req.system, req.user)
            }]
        }]
    });

    let res = self.client.post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("Gemini API Error: {}", res.text().await?));
    }

    let gemini_res: GeminiResponse = res.json().await?;

    // Navigate nested response structure
    gemini_res.candidates.first()
        .and_then(|c| c.content.parts.first())
        .map(|p| p.text.clone())
        .ok_or_else(|| anyhow!("No content in Gemini response"))
}
```

**Response Schema (Gemini)**:
```rust
#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Deserialize)]
struct GeminiPart {
    text: String,
}
```

**Provider 3: DeepSeek** (RESEARCH-001-API-003)
```rust
async fn generate_deepseek(&self, req: LLMRequest) -> Result<String> {
    let url = "https://api.deepseek.com/chat/completions";

    // OpenAI-compatible message format
    let body = serde_json::json!({
        "model": req.model,              // "deepseek-chat" or "deepseek-reasoner"
        "messages": [
            {"role": "system", "content": req.system},
            {"role": "user", "content": req.user}
        ],
        "stream": false                  // Non-streaming response
    });

    let res = self.client.post(url)
        .header("Authorization", format!("Bearer {}", self.api_key))  // Bearer token auth
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(anyhow!("DeepSeek API Error: {}", res.text().await?));
    }

    let deepseek_res: DeepSeekResponse = res.json().await?;

    // Extract message content
    deepseek_res.choices.first()
        .map(|c| c.message.content.clone())
        .ok_or_else(|| anyhow!("No content in DeepSeek response"))
}
```

**Response Schema (DeepSeek)**:
```rust
#[derive(Debug, Deserialize)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekChoice {
    message: DeepSeekMessage,
}

#[derive(Debug, Deserialize)]
struct DeepSeekMessage {
    content: String,
}
```

**Critical Gaps Identified**:
- ❌ **No Token Counting**: Cannot estimate costs
- ❌ **No Streaming**: All responses synchronous (poor UX for long generations)
- ❌ **No Retry Logic**: Network failures cause immediate workflow failure
- ❌ **No Rate Limiting**: Could hit API rate limits
- ❌ **No Response Caching**: Repeated identical calls waste money
- ❌ **No Cost Tracking**: No cumulative cost calculation
- ❌ **Fixed max_tokens**: Anthropic hardcoded to 4096 (phase 5 may need 8192+)

---

#### 1.2.4 YAML Manifest Parser (manifest.rs)

**RESEARCH-001-IMPL-004: Workflow Definition Parser**

**Primary Structures**:
```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub manifest: ManifestHeader,                     // Metadata
    pub schemas: HashMap<String, DataSchema>,         // Output schemas
    pub phases: Vec<Phase>,                           // Workflow phases
    pub quality_gates: Vec<QualityGate>,              // Validation rules
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManifestHeader {
    pub id: String,              // e.g., "FULLINTEL-PROTO-001"
    pub version: String,         // e.g., "1.0.0"
    pub name: String,            // Human-readable name
    pub description: String,     // Purpose description
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataSchema {
    pub fields: Vec<SchemaField>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SchemaField {
    pub name: String,
    #[serde(default)]
    pub r#enum: Option<Vec<String>>,   // 'enum' reserved in Rust, use r#enum
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Phase {
    pub id: String,                                   // e.g., "phase_1"
    pub name: String,                                 // e.g., "Context & Firmographics"
    #[serde(default)]
    pub tools: Vec<String>,                           // e.g., ["search_tool", "finance_api"]
    #[serde(default)]
    pub dependencies: Vec<String>,                    // Phase IDs this depends on
    pub instructions: String,                         // LLM instructions
    #[serde(default)]
    pub input: Option<String>,                        // Context key to use as input
    #[serde(default)]
    pub output_schema: Option<String>,                // Schema name for validation
    #[serde(default)]
    pub output_target: Option<String>,                // Context key for output
    #[serde(default)]
    pub output_format: Option<String>,                // e.g., "markdown", "json"
    #[serde(default)]
    pub logic_map: Option<HashMap<String, HashMap<String, String>>>,  // Phase 4 mapping
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct QualityGate {
    pub phase: String,           // Phase ID
    pub check: String,           // Validation question
    pub fail_action: String,     // "RETRY_SEARCH", "REGENERATE_WITH_PENALTY", etc.
}
```

**Loading Method**:
```rust
impl Manifest {
    /// Load and parse a manifest file from disk
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read manifest file: {:?}", path.as_ref()))?;

        let manifest: Manifest = serde_yaml::from_str(&content)
            .with_context(|| "Failed to parse YAML manifest")?;

        Ok(manifest)
    }

    /// Get a specific phase by ID
    pub fn get_phase(&self, id: &str) -> Option<&Phase> {
        self.phases.iter().find(|p| p.id == id)
    }
}
```

**Unit Test Coverage** (TVC-01):
```rust
#[test]
fn test_parse_fullintel_manifest() {
    let yaml_content = r#"
manifest:
  id: "PROTO-TEST-001"
  version: "1.0.0"
  name: "Test Protocol"
  description: "Unit test protocol."

schemas:
  TestSchema:
    fields:
      - name: test_field

phases:
  - id: "PHASE-01"
    name: "Context"
    tools: ["search"]
    instructions: "Do research."
    output_schema: "TestSchema"

quality_gates:
  - phase: "PHASE-01"
    check: "Is good?"
    fail_action: "RETRY"
"#;
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", yaml_content).unwrap();

    let manifest = Manifest::load_from_file(file.path()).unwrap();

    assert_eq!(manifest.manifest.id, "PROTO-TEST-001");
    assert_eq!(manifest.phases.len(), 1);
    assert_eq!(manifest.phases[0].tools[0], "search");
    assert_eq!(manifest.schemas.get("TestSchema").unwrap().fields[0].name, "test_field");
}
```

**Test Coverage**: ✅ VERIFIED (lines 93-135)
- Parsing correctness
- Schema validation
- Phase structure
- Quality gate deserialization

---

### 1.3 Dependency Analysis (VERIFIED)

**RESEARCH-001-INTEG-001: Rust Crate Dependencies** (Cargo.toml)

```toml
[dependencies]
tauri = { version = "2.0", features = [] }       # Desktop framework
tauri-plugin-shell = "2.0"                       # Shell command execution
serde = { version = "1.0", features = ["derive"] }  # Serialization
serde_json = "1.0"                               # JSON handling
serde_yaml = "0.9"                               # YAML parsing
tokio = { version = "1", features = ["full"] }   # Async runtime
reqwest = { version = "0.11", features = ["json"] }  # HTTP client
anyhow = "1.0"                                   # Error handling

[dev-dependencies]
tempfile = "3.8"                                 # Temporary files for tests

[features]
default = ["custom-protocol"]
custom-protocol = ["tauri/custom-protocol"]      # Required for production build
```

**Build Dependencies**:
```toml
[build-dependencies]
tauri-build = { version = "2.0", features = [] }  # Build-time codegen
```

**Critical Missing Dependencies**:
- ❌ **No Tavily API client** (web search tool integration)
- ❌ **No NewsAPI client** (news search tool integration)
- ❌ **No Apollo.io client** (contact discovery tool integration)
- ❌ **No Hunter.io client** (email finder tool integration)
- ❌ **No LinkedIn client** (officially impossible due to ToS)
- ❌ **No SQLite** (session persistence missing)
- ❌ **No chromadb-rs** (vector storage missing)

**Implication**: Current implementation is **LLM-only workflow** with no external tool integrations.

---

## 2. Workflow Analysis (YAML Manifest)

**Reference Document**: `manifests/fullintel_process_manifest.yaml`

### 2.1 5-Phase Structure

```yaml
Phase 1: Context & Firmographics
  ID: phase_1
  Tools: [search_tool, finance_api]
  Dependencies: []
  Input: target_company
  Output Schema: CompanyProfile
  Output Target: company_context

Phase 2: Situation Analysis
  ID: phase_2
  Tools: [news_search_tool, sentiment_analysis]
  Dependencies: [phase_1]
  Input: company_context
  Output Schema: SituationAnalysis
  Output Target: situation_analysis

Phase 3: Comms Team Intelligence
  ID: phase_3
  Tools: [linkedin_search_tool]
  Dependencies: [phase_2]
  Input: situation_analysis
  Output Schema: pain_points_list
  Output Target: pain_points

Phase 4: Solution Matching
  ID: phase_4
  Logic Map: Map scenario_type → Fullintel solution
  Dependencies: [phase_2]
  Input: situation_analysis
  Output Schema: solution_package
  Output Target: matched_solution

Phase 5: Brief Generation
  ID: phase_5
  Model: claude-3-5-sonnet (REQUIRED)
  Dependencies: [phase_1, phase_2, phase_3, phase_4]
  Input: [ALL context keys]
  Output Format: markdown
  Output Target: markdown_file
  Template: "FULLINTEL OPPORTUNITY BRIEF"
```

### 2.2 Quality Gates Defined

**Gate 1 - Phase 2: Coverage Quantification**
```yaml
phase: phase_2
check: "Is coverage_volume quantified?"
fail_action: "RETRY_SEARCH"
```
- **Intent**: Ensure numeric metrics (e.g., "15 articles" not "some coverage")
- **Implementation Status**: ❌ NOT IMPLEMENTED (no validator in agent.rs)

**Gate 2 - Phase 5: Generic Text Detection**
```yaml
phase: phase_5
check: "Does output contain 'generic' or 'placeholder' text?"
fail_action: "REGENERATE_WITH_PENALTY"
```
- **Intent**: Block low-quality outputs with placeholders
- **Implementation Status**: ❌ NOT IMPLEMENTED

**Gate 3 - Phase 5: ROI Specificity**
```yaml
phase: phase_5
check: "Are ROI calculations present and specific?"
fail_action: "RECALCULATE_ROI"
```
- **Intent**: Require quantified ROI ($X saved, Y% increase)
- **Implementation Status**: ❌ NOT IMPLEMENTED

**Gate 4 - Phase 5: Case Study Relevance**
```yaml
phase: phase_5
check: "Is a specific, relevant case study included?"
fail_action: "SEARCH_CASE_STUDIES"
```
- **Intent**: Ensure real case study not hallucinated
- **Implementation Status**: ❌ NOT IMPLEMENTED

**Critical Gap**: All 4 quality gates defined in YAML but **no validation code** exists in agent.rs. Workflow completes regardless of output quality.

---

## 3. Token Economics Estimate (CALCULATED)

**Assumption**: Claude Sonnet 3.5 pricing (2025-11-19):
- Input: $3.00 / 1M tokens
- Output: $15.00 / 1M tokens

**Per-Phase Estimates** (based on typical prompt sizes):

| Phase | Tokens In | Tokens Out | Cost (Input) | Cost (Output) | Total Cost |
|-------|-----------|------------|--------------|---------------|------------|
| 1 - Context & Firmographics | 500 | 500 | $0.0015 | $0.0075 | $0.009 |
| 2 - Situation Analysis | 800 | 800 | $0.0024 | $0.0120 | $0.0144 |
| 3 - Comms Team Intelligence | 600 | 400 | $0.0018 | $0.0060 | $0.0078 |
| 4 - Solution Matching | 700 | 500 | $0.0021 | $0.0075 | $0.0096 |
| 5 - Brief Generation | 2000 | 4000 | $0.0060 | $0.0600 | $0.0660 |
| **TOTAL** | **4600** | **6200** | **$0.0138** | **$0.093** | **$0.1068** |

**Cost Per Brief**: ~$0.11 (exceeds $0.10 target by 10%)

**Optimization Opportunities**:
1. Use DeepSeek for Phases 1-3 (~$0.001 each) = Save $0.026
2. Reduce Phase 5 output from 4000 → 3000 tokens = Save $0.015
3. Implement response caching for repeated companies = 50% savings on cache hit

**Revised Estimate with DeepSeek (Phases 1-3)**:
- Phase 1-3: $0.003 total (DeepSeek)
- Phase 4: $0.0096 (Claude)
- Phase 5: $0.0660 (Claude)
- **New Total**: $0.0786 (~$0.08 per brief) ✅ Meets <$0.10 target

---

## 4. Tool Requirements Analysis

### 4.1 Phase 1: Context & Firmographics

**Declared Tools**: `search_tool`, `finance_api`

**Implementation Options**:
1. **Tavily API** ($) - $0.001/search, 100 req/day free tier
   - **Pros**: Structured JSON, fast, good coverage
   - **Cons**: Rate limits, requires crate integration

2. **Crunchbase API** ($$$) - Best data quality
   - **Pros**: Authoritative firmographic data
   - **Cons**: Expensive ($29/month+), requires approval

3. **Web Search + LLM Extraction** ($) - Current approach
   - **Pros**: No external dependencies
   - **Cons**: Hallucination risk, slower, less accurate

**Recommendation**: Tavily API for MVP (free tier), upgrade to Crunchbase post-validation

### 4.2 Phase 2: Situation Analysis

**Declared Tools**: `news_search_tool`, `sentiment_analysis`

**Implementation Options**:
1. **NewsAPI.org** (Free tier: 100 req/day)
   - **Endpoint**: `https://newsapi.org/v2/everything?q={company}&sortBy=publishedAt`
   - **Pros**: Free, structured JSON, recent articles
   - **Cons**: 100 req/day limit

2. **Google News RSS** (Free)
   - **Pros**: Unlimited, no API key
   - **Cons**: Requires scraping, less structured

3. **Fullintel Internal API** (Unknown)
   - **Action Required**: Ask Ted if Fullintel has media monitoring API access

**Recommendation**: NewsAPI.org for MVP, check internal resources

### 4.3 Phase 3: Comms Team Intelligence

**Declared Tools**: `linkedin_search_tool`

**Challenge**: LinkedIn prohibits scraping, official API restrictive/expensive

**Implementation Options**:
1. **Apollo.io** ($$) - Contact database
   - **Pricing**: Free tier (50 contacts/month), $49/month (1000 contacts)
   - **API**: Yes, includes job titles, LinkedIn URLs
   - **Pros**: Legal, accurate, includes emails

2. **Hunter.io** ($$) - Email finder
   - **Pricing**: Free tier (25 searches/month), $49/month (500 searches)
   - **API**: Yes, email verification included
   - **Pros**: Legal, good accuracy

3. **Manual Fallback** (Free)
   - **UX**: Prompt user to input contact info if API fails
   - **Pros**: Legal, no cost, always works
   - **Cons**: Friction, slower

**Recommendation**: Apollo.io (free tier) with manual fallback for MVP

### 4.4 Phase 4: Solution Matching

**Declared**: Logic map (no external tool needed)

**Implementation**:
- **Internal Case Study Database**: SQLite or JSON file
- **Logic**: Parse `scenario_type` from Phase 2
- **Mapping**:
  - `negative_coverage` → "Crisis Management Suite"
  - `industry_event` → "Thought Leadership Package"
  - `funding_announcement` → "Momentum Amplification"
  - `quiet_period` → "Proactive Outreach Program"

**Data Requirement**: Fullintel must provide:
1. Solution catalog (JSON)
2. Case study library (markdown files)
3. Scenario → solution mapping table

### 4.5 Phase 5: Brief Generation

**Model**: `claude-3-5-sonnet` (specified in manifest)

**Requirements**:
- ✅ 200K context window (fits all prior outputs)
- ✅ Markdown formatting capability
- ✅ Instruction following (template adherence)
- ✅ Already implemented in llm.rs

**Template Structure** (from manifest):
```markdown
# FULLINTEL OPPORTUNITY BRIEF

## COMPANY CONTEXT
[Phase 1 output]

## CURRENT SITUATION
[Phase 2 output]

## KEY CONTACTS & PAIN POINTS
[Phase 3 output]

## RECOMMENDED APPROACH
[Phase 4 output]

## NEXT STEPS
[Generated by Phase 5 LLM]
```

---

## 5. Build Status (VERIFIED)

**Rust Toolchain**:
- **Version**: 1.91.1 (VERIFIED via `rustc --version`)
- **Target**: `x86_64-pc-windows-msvc`
- **Installation**: Complete

**Build Errors Encountered** (Previous Session):
1. ❌ **Cargo.toml was incomplete** (only 5 lines) → FIXED (now 27 lines)
2. ❌ **Feature mismatch**: `protocol-asset` removed from tauri.conf.json → FIXED
3. ⚠️ **CSP Headers Missing**: `tauri.conf.json` needs Content Security Policy

**Current Build Status**:
- ✅ **Rust compilation**: SUCCESS (all files compile)
- ❌ **Frontend missing**: No React components in `src/` directory
- ❌ **No UI implementation**: Can't run full application
- ✅ **Build tooling**: Vite + TypeScript configured correctly

**To Build**:
```bash
cd ted_skinner_project/src-tauri
cargo build
```

**Expected Result**: Rust backend compiles, but Tauri won't run without frontend.

---

## 6. Critical Gaps Summary

### 6.1 Traceability Gaps (RESEARCH-001 Classification)
- ✅ **FIXED**: Document now has DOC-RESEARCH-001 classification
- ✅ **FIXED**: All Component-IDs documented (IMPL-001 through API-003)
- ✅ **FIXED**: Actual code extraction performed (not theoretical)

### 6.2 Implementation Gaps
1. **Tool Integration Layer Missing**:
   - No `ToolRegistry` trait implementation
   - No Tavily, NewsAPI, Apollo.io integrations
   - No fallback mechanisms

2. **Quality Gate Enforcement Missing**:
   - YAML defines 4 gates
   - Zero validation code in agent.rs
   - No quality_gates.rs module

3. **State Persistence Missing**:
   - No SQLite for session storage
   - No crash recovery mechanism
   - Workflow lost if application closes

4. **Frontend Missing**:
   - No React components
   - No progress visualization
   - No settings page

### 6.3 LLM Client Limitations
- ❌ No token counting
- ❌ No streaming
- ❌ No retry logic
- ❌ No rate limiting
- ❌ No response caching
- ❌ No cost tracking
- ❌ Fixed 4096 max_tokens (insufficient for Phase 5)

---

## 7. Recommendations

### 7.1 Immediate (Week 1)
1. ✅ **COMPLETED**: Fix Cargo.toml
2. ✅ **COMPLETED**: Fix Tauri config mismatch
3. ❌ **TODO**: Implement minimal React UI
4. ❌ **TODO**: Add SQLite state persistence
5. ❌ **TODO**: Implement tool registry pattern (ToolTrait)

### 7.2 Short-term (Weeks 2-3)
6. ❌ **TODO**: Integrate Tavily API for search (Rust crate: `reqwest` + custom client)
7. ❌ **TODO**: Add NewsAPI.org for news search
8. ❌ **TODO**: Implement quality gate validators (quality_gates.rs module)
9. ❌ **TODO**: Add progress streaming UI (React + Tauri events)
10. ❌ **TODO**: Write integration tests

### 7.3 Medium-term (Month 2)
11. ❌ **TODO**: Add Apollo.io/Hunter.io for contacts
12. ❌ **TODO**: Implement response caching (Redis or in-memory)
13. ❌ **TODO**: Add export to PDF/Word (tauri-plugin-fs)
14. ❌ **TODO**: Build analytics dashboard
15. ❌ **TODO**: CRM integration (Salesforce/HubSpot webhooks)

---

## 8. Traceability Matrix

| L1 Requirement | Research Finding | Component-ID | Status |
|----------------|------------------|--------------|--------|
| MO-001: Time < 5 min | Workflow executes 5 phases sequentially | RESEARCH-001-IMPL-002 | ✅ Architecture supports |
| MO-002: Quality gates | 4 gates defined in YAML | RESEARCH-001-IMPL-004 | ⚠️ Defined but not enforced |
| MO-003: Cost < $0.10 | ~$0.08 with DeepSeek optimization | RESEARCH-001-API-003 | ✅ Target achievable |
| SR-005: Multi-LLM | 3 providers implemented | RESEARCH-001-IMPL-003 | ✅ Complete |
| SR-007: API security | AppConfig with encrypted storage path | RESEARCH-001-IMPL-001 | ⚠️ Path exists, encryption TBD |
| SR-009: Crash recovery | No session persistence | N/A | ❌ Missing |

---

## 9. Open Questions for Ted

1. **Budget**: Confirmed $0.08-0.10 per brief acceptable?
   - Current estimate: $0.08 with DeepSeek (Phases 1-3) + Claude (Phases 4-5)
   - Tool API costs add: $0.001 Tavily + $0.00 NewsAPI (free tier) = ~$0.081 total

2. **Data Access**: Does Fullintel have:
   - Internal media monitoring API we can use? (replaces NewsAPI)
   - Existing company databases? (replaces Tavily)
   - Case study database in structured format? (JSON or markdown)

3. **Deployment**:
   - Desktop app only or web version later? ✅ **ANSWER: Desktop only (Tauri)**
   - Single user or team collaboration features? ✅ **ANSWER: Single user MVP**
   - Cloud sync needed? ✅ **ANSWER: Local-first, no cloud**

4. **Compliance**:
   - Data retention policy preferences? (90 days auto-delete?)
   - GDPR/CCPA considerations? (Fullintel clients in EU/CA?)
   - LinkedIn scraping legal review status? ⚠️ **RECOMMEND: Use Apollo.io instead**

5. **Integration**:
   - Which CRM does sales team use? (Salesforce, HubSpot, Pipedrive, other?)
   - Export format preferences? (PDF confirmed, Word needed?)
   - Email integration needed? (Gmail, Outlook integration?)

---

## 10. Next Phase Readiness

**Phase 3 (NOTES) Prerequisites**:
- ✅ Codebase structure understood (actual code read)
- ✅ Architecture patterns identified (Agent, LLMClient, Manifest)
- ✅ Tool gaps documented (no external integrations)
- ✅ Cost estimates calculated ($0.08 with optimization)
- ✅ Component-IDs established (RESEARCH-001-*)

**Ready to Proceed**: ✅ YES

**Phase 3 Deliverable**: Architectural decisions document with:
1. Tool Registry Pattern design
2. SQLite State Persistence schema
3. Quality Gate Validator implementation plan
4. Progressive Disclosure UI mockups
5. MVP Tool Stack final selections
6. LLM Provider Strategy (DeepSeek + Claude)
7. Security Architecture (API key encryption)
8. Error Handling Strategy (retry logic)

---

**Document Status**: Complete - Remediated (Microgate 1 Compliance)
**Next Action**: Create `PHASE_3_NOTES_v2.md` with Taxonomy v3.0 classification
**Traceability**: DOC-RESEARCH-001 → DOC-NOTES-001 → DOC-PLAN-001 → DOC-PRECODE-001

---

## Appendix A: Method Signatures Catalog

### A.1 main.rs

```rust
// Tauri Commands (RESEARCH-001-IMPL-001)
async fn set_api_key(key: String, state: State<'_, AppState>) -> Result<(), String>
async fn get_app_state(state: State<'_, AppState>) -> Result<AppConfig, String>
async fn run_research(company: String, window: Window, state: State<'_, AppState>) -> Result<String, String>

// AppState Methods
impl AppState {
    fn save(&self) -> Result<(), String>
}
```

### A.2 agent.rs

```rust
// Agent Constructor (RESEARCH-001-IMPL-002)
impl Agent {
    pub fn new(manifest: Manifest, api_key: String, window: Option<Window>) -> Self

    // Core Workflow
    pub async fn run_workflow(&mut self, initial_input: &str) -> Result<()>
    async fn execute_phase(&self, phase: &Phase) -> Result<String>

    // Helpers
    fn log(&self, msg: &str)
    fn update_phase_status(&mut self, phase_id: &str, status: PhaseStatus)
}

// AgentState Constructor
impl AgentState {
    pub fn new() -> Self
}
```

### A.3 llm.rs

```rust
// LLMClient (RESEARCH-001-IMPL-003)
impl LLMClient {
    pub fn new(api_key: String) -> Self
    pub async fn generate(&self, req: LLMRequest) -> Result<String>

    // Provider-Specific
    async fn generate_anthropic(&self, req: LLMRequest) -> Result<String>  // RESEARCH-001-API-001
    async fn generate_gemini(&self, req: LLMRequest) -> Result<String>     // RESEARCH-001-API-002
    async fn generate_deepseek(&self, req: LLMRequest) -> Result<String>   // RESEARCH-001-API-003
}
```

### A.4 manifest.rs

```rust
// Manifest (RESEARCH-001-IMPL-004)
impl Manifest {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self>
    pub fn get_phase(&self, id: &str) -> Option<&Phase>
}
```

---

## Appendix B: Error Handling Patterns

**Pattern 1: Mutex Lock Errors**
```rust
let config = state.config.lock().map_err(|_| "Failed to lock state")?;
```
- **Risk**: Panics converted to String errors
- **Improvement**: Use `map_err(|e| e.to_string())` for debugging

**Pattern 2: Missing Context Keys**
```rust
let input_data = self.state.context.get(input_key)
    .ok_or_else(|| anyhow!("Missing input: {}", input_key))?
    .clone();
```
- **Risk**: Workflow fails if phase dependency unsatisfied
- **Improvement**: Check dependencies before phase execution

**Pattern 3: API Errors**
```rust
if !res.status().is_success() {
    return Err(anyhow!("Anthropic API Error: {}", res.text().await?));
}
```
- **Risk**: No retry logic, immediate failure
- **Improvement**: Exponential backoff, max 3 retries

---

## Appendix C: Performance Estimates

**Workflow Duration Estimate** (sequential execution):
- Phase 1: ~15 seconds (LLM call)
- Phase 2: ~20 seconds (LLM call)
- Phase 3: ~15 seconds (LLM call)
- Phase 4: ~10 seconds (LLM call)
- Phase 5: ~45 seconds (LLM call, longest output)
- **Total**: ~105 seconds (~1.75 minutes) ✅ Well under 5-minute target

**With Tool Integration** (parallel external API calls):
- Phase 1: ~8 seconds (Tavily + LLM parallel)
- Phase 2: ~10 seconds (NewsAPI + LLM parallel)
- Phase 3: ~10 seconds (Apollo.io + LLM parallel)
- Phase 4: ~5 seconds (local DB lookup)
- Phase 5: ~45 seconds (LLM only)
- **Optimized Total**: ~78 seconds (~1.3 minutes) ✅ 50% buffer under target

---

**END OF DOCUMENT**
