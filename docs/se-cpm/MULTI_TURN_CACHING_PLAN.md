# Multi-Turn Conversation & Prompt Caching Implementation Plan
## CDP LODA Enhancement - Phase 4: PLAN

**Date:** 2025-11-28
**Status:** Draft - Pending Review
**Sprint:** CDP LODA Decomposition Implementation
**Based On:** MULTI_TURN_AND_CACHING_RESEARCH.md

---

## 1. Executive Summary

This plan details the technical implementation for adding multi-turn conversation support and prompt caching to the Ted Skinner Fullintel Agent's LLM client. The implementation must handle four providers (Anthropic, OpenAI, DeepSeek, Gemini) with distinct API formats and caching mechanisms.

### 1.1 Current State (llm.rs Analysis)

| Component | Current | Target |
|-----------|---------|--------|
| `LLMRequest` | Single-turn (system + user strings) | Multi-turn (message history array) |
| Message format | Provider-agnostic single message | Provider-specific transformations |
| Caching | None | Provider-optimized caching |
| API call pattern | One user message per call | Full conversation context |

### 1.2 Scope

**In Scope:**
- Multi-turn message history for all 4 providers
- Prompt caching for Anthropic (explicit cache_control)
- Automatic caching utilization for OpenAI/DeepSeek
- Gemini implicit caching (2.5 models)

**Out of Scope (Future Enhancement):**
- Gemini explicit Context Caching API (requires separate API calls)
- Token counting for cache eligibility estimation
- Persistent conversation storage (database)

---

## 2. Technical Architecture

### 2.1 New Type Definitions

```rust
// IM-4001: ChatMessage - Individual conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,     // IM-4001-F1: Semantic role (abstracted)
    pub content: String,    // IM-4001-F2: Message content
}

// IM-4002: ChatRole - Provider-independent role enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatRole {
    System,     // System instructions (some providers)
    User,       // User messages
    Assistant,  // AI responses (maps to "model" for Gemini)
}

impl ChatRole {
    // IM-4002-M1: Convert to provider-specific role string
    pub fn to_provider_string(&self, provider: &str) -> &'static str {
        match (self, provider) {
            (ChatRole::System, _) => "system",
            (ChatRole::User, _) => "user",
            (ChatRole::Assistant, "gemini") => "model",  // CRITICAL: Gemini uses "model"
            (ChatRole::Assistant, _) => "assistant",
        }
    }
}

// IM-4003: MultiTurnRequest - Full conversation request
#[derive(Debug, Clone, Serialize)]
pub struct MultiTurnRequest {
    pub system: Option<String>,      // IM-4003-F1: Optional system prompt
    pub messages: Vec<ChatMessage>,  // IM-4003-F2: Conversation history
    pub model: String,               // IM-4003-F3: Model identifier
    pub enable_caching: bool,        // IM-4003-F4: Enable provider caching
}

// IM-4004: CacheConfig - Caching configuration (Anthropic-specific)
#[derive(Debug, Clone, Serialize)]
pub struct CacheConfig {
    pub ttl: CacheTTL,              // IM-4004-F1: Time to live
}

// IM-4005: CacheTTL - Cache duration options
#[derive(Debug, Clone, Serialize)]
pub enum CacheTTL {
    FiveMinutes,  // Default, lower write cost
    OneHour,      // Extended, higher write cost (2x)
}
```

### 2.2 Provider-Specific Transformations

#### 2.2.1 Anthropic Transformation

```rust
// IM-4010: Transform MultiTurnRequest to Anthropic JSON
fn to_anthropic_body(req: &MultiTurnRequest) -> serde_json::Value {
    let mut messages: Vec<serde_json::Value> = req.messages.iter()
        .filter(|m| m.role != ChatRole::System) // System is separate field
        .map(|m| {
            if req.enable_caching && m.role == ChatRole::User {
                // IM-4010-B1: Add cache_control to cacheable messages
                serde_json::json!({
                    "role": m.role.to_provider_string("anthropic"),
                    "content": [{
                        "type": "text",
                        "text": m.content,
                        "cache_control": {"type": "ephemeral"}
                    }]
                })
            } else {
                serde_json::json!({
                    "role": m.role.to_provider_string("anthropic"),
                    "content": m.content
                })
            }
        })
        .collect();

    serde_json::json!({
        "model": req.model,
        "max_tokens": 4096,
        "system": req.system,
        "messages": messages
    })
}
```

#### 2.2.2 OpenAI/DeepSeek Transformation

```rust
// IM-4011: Transform MultiTurnRequest to OpenAI-compatible JSON
fn to_openai_body(req: &MultiTurnRequest) -> serde_json::Value {
    let mut messages: Vec<serde_json::Value> = Vec::new();

    // IM-4011-B1: Add system message first if present
    if let Some(ref system) = req.system {
        messages.push(serde_json::json!({
            "role": "system",
            "content": system
        }));
    }

    // IM-4011-B2: Add conversation history
    for msg in &req.messages {
        messages.push(serde_json::json!({
            "role": msg.role.to_provider_string("openai"),
            "content": msg.content
        }));
    }

    // Note: Caching is AUTOMATIC - no special handling needed
    serde_json::json!({
        "model": req.model,
        "messages": messages,
        "stream": false
    })
}
```

#### 2.2.3 Gemini Transformation (CRITICAL DIFFERENCES)

```rust
// IM-4012: Transform MultiTurnRequest to Gemini JSON
fn to_gemini_body(req: &MultiTurnRequest) -> serde_json::Value {
    // IM-4012-V1: Gemini uses "contents" not "messages"
    // IM-4012-V2: Gemini uses "parts" array with text objects
    // IM-4012-V3: Gemini uses "model" role not "assistant"

    let contents: Vec<serde_json::Value> = req.messages.iter()
        .filter(|m| m.role != ChatRole::System) // Handle system separately
        .map(|m| {
            serde_json::json!({
                "role": m.role.to_provider_string("gemini"),
                "parts": [{"text": m.content}]
            })
        })
        .collect();

    let mut body = serde_json::json!({
        "contents": contents
    });

    // IM-4012-B1: Add system instruction if present
    if let Some(ref system) = req.system {
        body["systemInstruction"] = serde_json::json!({
            "parts": [{"text": system}]
        });
    }

    // Note: Implicit caching is automatic on Gemini 2.5 models
    body
}
```

### 2.3 Updated LLMClient Methods

```rust
impl LLMClient {
    // IM-4020: New multi-turn generate method
    pub async fn generate_multi_turn(&mut self, req: MultiTurnRequest) -> Result<String> {
        let provider_name = self.detect_provider(&req.model)?;

        // Apply rate limiting (existing IM-3020-3024)
        self.apply_rate_limiting(&provider_name).await?;

        // Apply circuit breaker (existing IM-3030-3040)
        self.check_circuit_breaker(&provider_name)?;

        // IM-4020-B1: Route to provider-specific implementation
        let result = match provider_name.as_str() {
            "anthropic" => self.generate_multi_anthropic(&req).await,
            "google" => self.generate_multi_gemini(&req).await,
            "deepseek" => self.generate_multi_deepseek(&req).await,
            "openai" => self.generate_multi_openai(&req).await,
            _ => Err(anyhow!("Unsupported provider: {}", provider_name)),
        };

        // Record circuit breaker outcome
        self.record_circuit_outcome(&provider_name, result.is_ok());

        result
    }

    // IM-4021: Anthropic multi-turn with caching
    async fn generate_multi_anthropic(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = "https://api.anthropic.com/v1/messages";
        let body = to_anthropic_body(req);

        let mut request_builder = self.client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");

        // IM-4021-B1: Add caching beta header if enabled
        if req.enable_caching {
            request_builder = request_builder
                .header("anthropic-beta", "prompt-caching-2024-07-31");
        }

        let res = request_builder.json(&body).send().await?;
        // ... response handling (existing pattern)
    }

    // IM-4022: Gemini multi-turn
    async fn generate_multi_gemini(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            req.model, self.api_key
        );
        let body = to_gemini_body(req);

        let res = self.client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;
        // ... response handling (existing pattern)
    }

    // IM-4023: DeepSeek multi-turn (OpenAI-compatible)
    async fn generate_multi_deepseek(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = "https://api.deepseek.com/chat/completions";
        let body = to_openai_body(req);
        // ... (uses existing DeepSeek handling with R1 reasoning support)
    }

    // IM-4024: OpenAI multi-turn
    async fn generate_multi_openai(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = "https://api.openai.com/v1/chat/completions";
        let body = to_openai_body(req);
        // ... (uses existing OpenAI handling)
    }
}
```

---

## 3. Streaming Support Extension

### 3.1 Multi-Turn Streaming

```rust
// IM-4030: Multi-turn streaming request
pub async fn generate_multi_turn_stream(
    &mut self,
    req: MultiTurnRequest,
) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
    let provider_name = self.detect_provider(&req.model)?;

    // Apply rate limiting and circuit breaker
    self.apply_rate_limiting(&provider_name).await?;
    self.check_circuit_breaker(&provider_name)?;

    match provider_name.as_str() {
        "anthropic" => self.stream_multi_anthropic(&req).await,
        "google" => self.stream_multi_gemini(&req).await,
        "deepseek" => self.stream_multi_deepseek(&req).await,
        "openai" => self.stream_multi_openai(&req).await,
        _ => Err(anyhow!("Unsupported provider for streaming: {}", provider_name)),
    }
}
```

---

## 4. Agent Integration

### 4.1 Changes to agent.rs

```rust
// IM-4040: Updated Agent to maintain conversation history
pub struct Agent {
    manifest: Manifest,
    state: AgentState,
    llm_client: LLMClient,
    app_handle: Option<AppHandle>,
    model_override: Option<String>,
    session_id: Option<i64>,
    conversation_history: Vec<ChatMessage>,  // NEW: Multi-turn history
    enable_caching: bool,                    // NEW: Caching preference
}

impl Agent {
    // IM-4041: Execute phase with conversation context
    async fn execute_phase(&mut self, phase: &Phase) -> Result<String> {
        // Build multi-turn request with history
        let request = MultiTurnRequest {
            system: Some(self.build_system_prompt(phase)),
            messages: self.build_phase_messages(phase)?,
            model: self.get_model(phase),
            enable_caching: self.enable_caching,
        };

        // Execute with multi-turn support
        let result = if self.should_stream() {
            self.execute_streaming(&request).await?
        } else {
            self.llm_client.generate_multi_turn(request).await?
        };

        // IM-4041-B1: Add assistant response to history for next phase
        self.conversation_history.push(ChatMessage {
            role: ChatRole::Assistant,
            content: result.clone(),
        });

        Ok(result)
    }

    // IM-4042: Build messages including history for phase
    fn build_phase_messages(&self, phase: &Phase) -> Result<Vec<ChatMessage>> {
        let mut messages = self.conversation_history.clone();

        // Add current phase input as new user message
        let input = self.get_phase_input(phase)?;
        messages.push(ChatMessage {
            role: ChatRole::User,
            content: input,
        });

        Ok(messages)
    }
}
```

---

## 5. Implementation Order

### Phase 1: Core Types (Estimated: 30 min)
1. Add `ChatRole` enum with `to_provider_string()` method
2. Add `ChatMessage` struct
3. Add `MultiTurnRequest` struct
4. Add `CacheConfig` and `CacheTTL` types

### Phase 2: Transformation Functions (Estimated: 45 min)
1. Implement `to_anthropic_body()` with cache_control support
2. Implement `to_openai_body()` for OpenAI/DeepSeek
3. Implement `to_gemini_body()` with parts/model role handling

### Phase 3: LLMClient Methods (Estimated: 60 min)
1. Add `generate_multi_turn()` main entry point
2. Add `generate_multi_anthropic()` with caching header
3. Add `generate_multi_gemini()` with contents format
4. Add `generate_multi_deepseek()` (OpenAI-compatible)
5. Add `generate_multi_openai()`

### Phase 4: Streaming Extensions (Estimated: 45 min)
1. Add `generate_multi_turn_stream()` entry point
2. Update each provider's stream method for multi-turn

### Phase 5: Agent Integration (Estimated: 30 min)
1. Add `conversation_history` to Agent struct
2. Add `enable_caching` configuration
3. Update `execute_phase()` for multi-turn
4. Add history management methods

---

## 6. Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Gemini role name mismatch | `ChatRole::to_provider_string()` handles "model" conversion |
| Gemini content structure | Dedicated `to_gemini_body()` with parts array |
| Anthropic caching header | Conditional header injection based on `enable_caching` |
| Message ordering | Validate alternating user/assistant before API call |
| History growth | Implement max history limit with sliding window |

---

## 7. Testing Requirements

### 7.1 Unit Tests
- [ ] `ChatRole::to_provider_string()` for all providers
- [ ] `to_anthropic_body()` with/without caching
- [ ] `to_openai_body()` message ordering
- [ ] `to_gemini_body()` parts structure validation

### 7.2 Integration Tests
- [ ] Multi-turn conversation with each provider
- [ ] Caching header verification (Anthropic)
- [ ] Streaming with multi-turn context

### 7.3 Edge Cases
- [ ] Empty conversation history
- [ ] System-only message (no user input)
- [ ] Very long conversation history (token limits)

---

## 8. Manifest Codes (IM Inventory)

| Code | Component | Description |
|------|-----------|-------------|
| IM-4001 | ChatMessage | Individual conversation message struct |
| IM-4002 | ChatRole | Provider-independent role enum |
| IM-4003 | MultiTurnRequest | Full conversation request struct |
| IM-4004 | CacheConfig | Caching configuration |
| IM-4005 | CacheTTL | Cache duration enum |
| IM-4010 | to_anthropic_body | Anthropic JSON transformation |
| IM-4011 | to_openai_body | OpenAI/DeepSeek JSON transformation |
| IM-4012 | to_gemini_body | Gemini JSON transformation |
| IM-4020 | generate_multi_turn | Main multi-turn entry point |
| IM-4021 | generate_multi_anthropic | Anthropic multi-turn with caching |
| IM-4022 | generate_multi_gemini | Gemini multi-turn |
| IM-4023 | generate_multi_deepseek | DeepSeek multi-turn |
| IM-4024 | generate_multi_openai | OpenAI multi-turn |
| IM-4030 | generate_multi_turn_stream | Streaming multi-turn entry |
| IM-4040 | Agent.conversation_history | Agent history field |
| IM-4041 | Agent.execute_phase | Updated phase execution |
| IM-4042 | Agent.build_phase_messages | History-aware message building |

---

## 9. Success Criteria

1. **Multi-Turn Works**: All 4 providers accept multi-turn requests
2. **Gemini Role Correct**: "model" role used instead of "assistant"
3. **Caching Active**: Anthropic requests include cache_control when enabled
4. **Backward Compatible**: Existing single-turn code still functions
5. **Agent History**: Phase outputs accumulate in conversation history
6. **Streaming Preserved**: Multi-turn streaming works for all providers

---

**Document Version:** 1.0
**Last Updated:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint)
**Review Status:** PENDING PRE-IMPLEMENTATION REVIEW
