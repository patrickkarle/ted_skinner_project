# L2-ICD-04: Multi-Turn Conversation & Prompt Caching Interface Control Document
## CDP LODA Enhancement - Phase 5: PRE-CODE Data Contracts

**Date:** 2025-11-28
**Status:** Draft
**Traces To:** MULTI_TURN_CACHING_PLAN.md, L3-CDD-03-LLMClient.md

---

## 1. Interface Overview

This ICD defines the data contracts for multi-turn conversation support and prompt caching across all four LLM providers. All types are defined in Rust and must be serializable for IPC with the frontend.

---

## 2. Core Data Contracts

### ICD-04-001: ChatRole Enum

**Purpose:** Provider-independent role identification for conversation messages.

```rust
/// ICD-04-001: Semantic role in a conversation
/// Abstracts provider-specific role names (Gemini uses "model" not "assistant")
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    /// System instructions (not all providers support as separate role)
    System,
    /// User/human message
    User,
    /// AI assistant response
    Assistant,
}

impl ChatRole {
    /// ICD-04-001-M1: Convert to provider-specific role string
    ///
    /// # Arguments
    /// * `provider` - Provider identifier: "anthropic", "openai", "deepseek", "gemini"
    ///
    /// # Returns
    /// Provider-specific role string for API serialization
    ///
    /// # Critical Behavior
    /// - Gemini: Returns "model" for Assistant role (API requirement)
    /// - All others: Returns "assistant" for Assistant role
    pub fn to_provider_string(&self, provider: &str) -> &'static str {
        match (self, provider) {
            (ChatRole::System, _) => "system",
            (ChatRole::User, _) => "user",
            (ChatRole::Assistant, "gemini" | "google") => "model",
            (ChatRole::Assistant, _) => "assistant",
        }
    }

    /// ICD-04-001-M2: Parse from provider-specific string
    pub fn from_provider_string(role: &str, provider: &str) -> Option<Self> {
        match (role, provider) {
            ("system", _) => Some(ChatRole::System),
            ("user", _) => Some(ChatRole::User),
            ("assistant", _) => Some(ChatRole::Assistant),
            ("model", "gemini" | "google") => Some(ChatRole::Assistant),
            _ => None,
        }
    }
}

impl std::fmt::Display for ChatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatRole::System => write!(f, "system"),
            ChatRole::User => write!(f, "user"),
            ChatRole::Assistant => write!(f, "assistant"),
        }
    }
}
```

---

### ICD-04-002: ChatMessage Struct

**Purpose:** Single message in a conversation with role and content.

```rust
/// ICD-04-002: Individual conversation message
///
/// # Serialization
/// - Frontend IPC: JSON with `role` as string, `content` as string
/// - Provider APIs: Transformed via provider-specific functions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// ICD-04-002-F1: Message role (user, assistant, or system)
    pub role: ChatRole,

    /// ICD-04-002-F2: Message content text
    pub content: String,

    /// ICD-04-002-F3: Optional timestamp for ordering validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl ChatMessage {
    /// ICD-04-002-C1: Create a new user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// ICD-04-002-C2: Create a new assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
            timestamp: Some(chrono::Utc::now()),
        }
    }

    /// ICD-04-002-C3: Create a new system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
            timestamp: Some(chrono::Utc::now()),
        }
    }
}
```

---

### ICD-04-003: CacheConfig Struct

**Purpose:** Configuration for provider-specific prompt caching.

```rust
/// ICD-04-003: Prompt caching configuration
///
/// # Provider Support
/// - Anthropic: Explicit cache_control with TTL
/// - OpenAI: Automatic (config ignored, used for metrics only)
/// - DeepSeek: Automatic (config ignored, used for metrics only)
/// - Gemini: Implicit on 2.5 models (config ignored)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// ICD-04-003-F1: Enable caching for this request
    pub enabled: bool,

    /// ICD-04-003-F2: Time-to-live for cached content (Anthropic only)
    #[serde(default)]
    pub ttl: CacheTTL,

    /// ICD-04-003-F3: Cache only system prompt (vs entire conversation)
    #[serde(default)]
    pub system_only: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: CacheTTL::FiveMinutes,
            system_only: false,
        }
    }
}

/// ICD-04-004: Cache time-to-live options (Anthropic-specific)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CacheTTL {
    /// 5 minutes (default) - lower write cost
    #[default]
    FiveMinutes,
    /// 1 hour - 2x write cost, extended caching
    OneHour,
}

impl CacheTTL {
    /// ICD-04-004-M1: Get Anthropic beta header for TTL
    pub fn anthropic_beta_header(&self) -> &'static str {
        match self {
            CacheTTL::FiveMinutes => "prompt-caching-2024-07-31",
            CacheTTL::OneHour => "extended-cache-ttl-2025-04-11",
        }
    }

    /// ICD-04-004-M2: Get TTL string for cache_control
    pub fn to_ttl_string(&self) -> &'static str {
        match self {
            CacheTTL::FiveMinutes => "5m",
            CacheTTL::OneHour => "1h",
        }
    }
}
```

---

### ICD-04-005: MultiTurnRequest Struct

**Purpose:** Complete request for multi-turn LLM generation.

```rust
/// ICD-04-005: Multi-turn conversation request
///
/// # Usage
/// Replaces single-turn `LLMRequest` for conversations with history.
/// Provider-specific serialization handled by transformation functions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTurnRequest {
    /// ICD-04-005-F1: System prompt (optional, separate from messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// ICD-04-005-F2: Conversation message history (chronological order)
    pub messages: Vec<ChatMessage>,

    /// ICD-04-005-F3: Model identifier (e.g., "claude-sonnet-4-5-20250929")
    pub model: String,

    /// ICD-04-005-F4: Caching configuration
    #[serde(default)]
    pub cache_config: CacheConfig,

    /// ICD-04-005-F5: Maximum tokens for response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// ICD-04-005-F6: Temperature for sampling (0.0-2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

fn default_max_tokens() -> u32 {
    4096
}

impl MultiTurnRequest {
    /// ICD-04-005-C1: Create new multi-turn request
    pub fn new(model: impl Into<String>, messages: Vec<ChatMessage>) -> Self {
        Self {
            system: None,
            messages,
            model: model.into(),
            cache_config: CacheConfig::default(),
            max_tokens: 4096,
            temperature: None,
        }
    }

    /// ICD-04-005-M1: Builder - set system prompt
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// ICD-04-005-M2: Builder - set caching config
    pub fn with_caching(mut self, config: CacheConfig) -> Self {
        self.cache_config = config;
        self
    }

    /// ICD-04-005-M3: Builder - disable caching
    pub fn without_caching(mut self) -> Self {
        self.cache_config.enabled = false;
        self
    }

    /// ICD-04-005-M4: Add a message to the conversation
    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    /// ICD-04-005-M5: Validate message ordering (alternating user/assistant)
    pub fn validate_ordering(&self) -> Result<(), String> {
        let mut last_role: Option<ChatRole> = None;

        for (i, msg) in self.messages.iter().enumerate() {
            if msg.role == ChatRole::System && i > 0 {
                return Err("System messages must be at the start".to_string());
            }

            if msg.role != ChatRole::System {
                if let Some(last) = last_role {
                    if last == msg.role && msg.role != ChatRole::System {
                        // Consecutive same roles (some providers auto-merge)
                        // This is a warning, not an error
                    }
                }
                last_role = Some(msg.role);
            }
        }

        Ok(())
    }
}
```

---

## 3. Provider Transformation Contracts

### ICD-04-010: Anthropic Request Format

**Input:** `MultiTurnRequest`
**Output:** Anthropic-compatible JSON

```rust
/// ICD-04-010: Transform to Anthropic Messages API format
///
/// # Format
/// ```json
/// {
///   "model": "claude-sonnet-4-5-20250929",
///   "max_tokens": 4096,
///   "system": "System prompt here",
///   "messages": [
///     {"role": "user", "content": "..."},
///     {"role": "assistant", "content": "..."}
///   ]
/// }
/// ```
///
/// # Caching
/// When enabled, wraps cacheable content in content blocks:
/// ```json
/// {"role": "user", "content": [{"type": "text", "text": "...", "cache_control": {"type": "ephemeral"}}]}
/// ```
pub fn to_anthropic_request(req: &MultiTurnRequest) -> serde_json::Value;

/// Required headers for Anthropic requests
pub struct AnthropicHeaders {
    pub api_key_header: (&'static str, String),     // ("x-api-key", key)
    pub version_header: (&'static str, &'static str), // ("anthropic-version", "2023-06-01")
    pub content_type: (&'static str, &'static str),  // ("content-type", "application/json")
    pub cache_beta: Option<(&'static str, &'static str)>, // ("anthropic-beta", "prompt-caching-...")
}
```

---

### ICD-04-011: OpenAI/DeepSeek Request Format

**Input:** `MultiTurnRequest`
**Output:** OpenAI-compatible JSON (works for both OpenAI and DeepSeek)

```rust
/// ICD-04-011: Transform to OpenAI Chat Completions API format
///
/// # Format
/// ```json
/// {
///   "model": "gpt-4o",
///   "messages": [
///     {"role": "system", "content": "System prompt"},
///     {"role": "user", "content": "..."},
///     {"role": "assistant", "content": "..."}
///   ],
///   "stream": false
/// }
/// ```
///
/// # Caching
/// OpenAI caching is AUTOMATIC for prompts >1024 tokens.
/// No special handling required - cache_config used only for metrics.
pub fn to_openai_request(req: &MultiTurnRequest) -> serde_json::Value;
```

---

### ICD-04-012: Gemini Request Format

**Input:** `MultiTurnRequest`
**Output:** Gemini-compatible JSON

```rust
/// ICD-04-012: Transform to Gemini generateContent API format
///
/// # CRITICAL DIFFERENCES
/// - Uses `contents` (not `messages`)
/// - Uses `parts` array (not `content` string)
/// - Uses `"model"` role (not `"assistant"`)
/// - System instruction is separate field
///
/// # Format
/// ```json
/// {
///   "systemInstruction": {"parts": [{"text": "System prompt"}]},
///   "contents": [
///     {"role": "user", "parts": [{"text": "..."}]},
///     {"role": "model", "parts": [{"text": "..."}]}
///   ]
/// }
/// ```
///
/// # Caching
/// Implicit caching is automatic on Gemini 2.5 models (as of May 2025).
/// Explicit caching would require Context Caching API (out of scope).
pub fn to_gemini_request(req: &MultiTurnRequest) -> serde_json::Value;
```

---

## 4. Response Contracts

### ICD-04-020: CacheMetrics Struct

**Purpose:** Track caching performance for monitoring.

```rust
/// ICD-04-020: Cache usage metrics from provider responses
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheMetrics {
    /// Tokens served from cache (cost savings)
    pub cached_tokens: u32,
    /// Tokens not in cache (full cost)
    pub uncached_tokens: u32,
    /// Cache hit rate (0.0 - 1.0)
    pub hit_rate: f32,
    /// Provider-specific cache identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_id: Option<String>,
}

impl CacheMetrics {
    /// ICD-04-020-M1: Calculate estimated cost savings
    /// Returns (cached_cost, uncached_cost, savings_percent)
    pub fn calculate_savings(&self, cost_per_million: f64) -> (f64, f64, f64) {
        let cached_cost = (self.cached_tokens as f64 / 1_000_000.0) * cost_per_million * 0.1; // 90% off
        let uncached_cost = (self.uncached_tokens as f64 / 1_000_000.0) * cost_per_million;
        let total = cached_cost + uncached_cost;
        let full_cost = ((self.cached_tokens + self.uncached_tokens) as f64 / 1_000_000.0) * cost_per_million;
        let savings = if full_cost > 0.0 { (full_cost - total) / full_cost * 100.0 } else { 0.0 };
        (cached_cost, uncached_cost, savings)
    }
}
```

---

## 5. Error Contracts

### ICD-04-030: Multi-Turn Errors

```rust
/// ICD-04-030: Multi-turn specific errors (extends LLMError)
#[derive(Debug, Error)]
pub enum MultiTurnError {
    #[error("Message history is empty")]
    EmptyHistory,

    #[error("Message ordering invalid: {0}")]
    InvalidOrdering(String),

    #[error("Conversation too long: {0} tokens exceeds {1} limit")]
    ContextLengthExceeded(usize, usize),

    #[error("Role transformation failed for provider {provider}: {role}")]
    RoleTransformError { provider: String, role: String },

    #[error("Provider {0} does not support multi-turn conversations")]
    UnsupportedProvider(String),
}
```

---

## 6. Tauri IPC Contracts

### ICD-04-040: Frontend Commands

```rust
/// ICD-04-040: Tauri command for multi-turn generation
///
/// # Frontend Usage (TypeScript)
/// ```typescript
/// const response = await invoke<string>('generate_multi_turn', {
///   request: {
///     system: "You are a research agent...",
///     messages: [
///       { role: "user", content: "Analyze company X" },
///       { role: "assistant", content: "Previous analysis..." },
///       { role: "user", content: "Now focus on financials" }
///     ],
///     model: "claude-sonnet-4-5-20250929",
///     cache_config: { enabled: true, ttl: "five_minutes" }
///   }
/// });
/// ```
#[tauri::command]
pub async fn generate_multi_turn(
    request: MultiTurnRequest,
    api_key: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String>;
```

---

## 7. Validation Rules

| Field | Validation | Error |
|-------|------------|-------|
| `messages` | Non-empty | `MultiTurnError::EmptyHistory` |
| `messages` | Last must be `User` | Implicit (provider requirement) |
| `model` | Valid provider prefix | `LLMError::UnsupportedModel` |
| `max_tokens` | 1-128000 | Clamped to range |
| `temperature` | 0.0-2.0 | Clamped to range |

---

## 8. Traceability Matrix

| ICD Code | PLAN Reference | Implementation Target |
|----------|----------------|----------------------|
| ICD-04-001 | IM-4002 | `llm.rs:ChatRole` |
| ICD-04-002 | IM-4001 | `llm.rs:ChatMessage` |
| ICD-04-003 | IM-4004 | `llm.rs:CacheConfig` |
| ICD-04-004 | IM-4005 | `llm.rs:CacheTTL` |
| ICD-04-005 | IM-4003 | `llm.rs:MultiTurnRequest` |
| ICD-04-010 | IM-4010 | `llm.rs:to_anthropic_request` |
| ICD-04-011 | IM-4011 | `llm.rs:to_openai_request` |
| ICD-04-012 | IM-4012 | `llm.rs:to_gemini_request` |
| ICD-04-020 | - | `llm.rs:CacheMetrics` |
| ICD-04-030 | - | `llm.rs:MultiTurnError` |
| ICD-04-040 | IM-4020 | `commands.rs:generate_multi_turn` |

---

**Document Version:** 1.0
**Last Updated:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint)
