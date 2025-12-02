# Multi-Turn Conversation & Prompt Caching Research
## CDP LODA Enhancement - Phase 2-3 Documentation

**Date:** 2025-11-28
**Status:** Research Complete
**Sprint:** CDP LODA Decomposition Implementation

---

## Executive Summary

This document captures research findings for implementing multi-turn conversation support and prompt caching across all four LLM providers (Anthropic, OpenAI, DeepSeek, Google Gemini). Each provider has distinct API formats and caching mechanisms requiring provider-specific implementations.

---

## 1. Anthropic (Claude)

### 1.1 Multi-Turn Conversation Format

**Message Structure:**
```json
{
  "messages": [
    {"role": "user", "content": "Hello there."},
    {"role": "assistant", "content": "Hi, I'm Claude. How can I help you?"},
    {"role": "user", "content": "Can you explain LLMs?"}
  ]
}
```

**Key Points:**
- Roles: `"user"` and `"assistant"` (alternating)
- Content can be string OR array of content blocks
- Consecutive same-role messages are auto-combined

**Content Block Format:**
```json
{"role": "user", "content": [{"type": "text", "text": "Hello, Claude"}]}
```

### 1.2 Prompt Caching

**Implementation:** Explicit via `cache_control` parameter

**Syntax:**
```json
{
  "type": "text",
  "text": "your cacheable content",
  "cache_control": {
    "type": "ephemeral",
    "ttl": "5m"
  }
}
```

**Requirements:**
- Header: `"anthropic-beta": "prompt-caching-2024-07-31"`
- Extended TTL header: `"anthropic-beta": "extended-cache-ttl-2025-04-11"` (for 1-hour)
- Up to 4 cache breakpoints per request
- 5-minute default TTL (inactivity expiry)

**Pricing:**
- Write to cache: 25% MORE than base input price
- Read from cache: 10% OF base input price (90% savings)

**Sources:**
- [Messages API Reference](https://platform.claude.com/docs/en/api/messages)
- [Prompt Caching Announcement](https://www.anthropic.com/news/prompt-caching)

---

## 2. OpenAI (GPT-4o, o1)

### 2.1 Multi-Turn Conversation Format

**Message Structure:**
```json
{
  "messages": [
    {"role": "system", "content": "You are a helpful assistant."},
    {"role": "user", "content": "Hello"},
    {"role": "assistant", "content": "Hi there!"},
    {"role": "user", "content": "How are you?"}
  ]
}
```

**Key Points:**
- Roles: `"system"`, `"user"`, `"assistant"`
- System message at beginning (optional)
- Append new messages to end of array

### 2.2 Prompt Caching

**Implementation:** AUTOMATIC - no code changes required

**How It Works:**
- Activates automatically for prompts >1,024 tokens
- Caches longest matching prefix
- Increments in 128-token chunks

**Requirements:**
- Minimum 1,024 tokens
- First 1,024 tokens must be IDENTICAL
- Static content at BEGINNING, variable at END

**Monitoring:**
```json
{
  "usage": {
    "prompt_tokens_details": {
      "cached_tokens": 1280
    }
  }
}
```

**Pricing:**
- 50% discount on cached input tokens

**Sources:**
- [Prompt Caching 101 - OpenAI Cookbook](https://cookbook.openai.com/examples/prompt_caching101)
- [Prompt Caching Announcement](https://openai.com/index/api-prompt-caching/)

---

## 3. DeepSeek

### 3.1 Multi-Turn Conversation Format

**Message Structure:**
```json
{
  "messages": [
    {"role": "user", "content": "Hello"},
    {"role": "assistant", "content": "Hi there!"},
    {"role": "user", "content": "What is Rust?"}
  ]
}
```

**Key Points:**
- Roles: `"user"` and `"assistant"`
- OpenAI-compatible format
- Full history must be sent each request (stateless API)

### 3.2 Chat Prefix Completion (Beta)

**Special Feature:** Force model to continue from a prefix

```json
{
  "messages": [
    {"role": "user", "content": "Write Python code for quicksort"},
    {"role": "assistant", "content": "```python\n", "prefix": true}
  ]
}
```

**Requirements:**
- Base URL: `https://api.deepseek.com/beta`
- Last message must have `role: "assistant"` with `prefix: true`

### 3.3 Prompt Caching

**Implementation:** AUTOMATIC Context Caching on Disk

**How It Works:**
- Caches identical prefixes automatically
- 64-token minimum storage unit
- Best-effort (not guaranteed 100% hit rate)

**Monitoring:**
```json
{
  "prompt_cache_hit_tokens": 1024,
  "prompt_cache_miss_tokens": 256
}
```

**Pricing:**
- Cache hit: $0.014 per million tokens
- Cache miss: $0.14 per million tokens (90% savings on hits)

**Sources:**
- [Context Caching Guide](https://api-docs.deepseek.com/guides/kv_cache)
- [Chat Prefix Completion](https://api-docs.deepseek.com/guides/chat_prefix_completion)

---

## 4. Google Gemini

### 4.1 Multi-Turn Conversation Format

**CRITICAL DIFFERENCE: Uses `"model"` role, NOT `"assistant"`!**

**Message Structure:**
```json
{
  "contents": [
    {
      "role": "user",
      "parts": [{"text": "Hello"}]
    },
    {
      "role": "model",
      "parts": [{"text": "Hi there! How can I help?"}]
    },
    {
      "role": "user",
      "parts": [{"text": "What is Rust?"}]
    }
  ]
}
```

**Key Points:**
- Roles: `"user"` and `"model"` (NOT "assistant"!)
- Uses `contents` array (NOT `messages`)
- Uses `parts` array for content (NOT `content` string)
- History must be chronologically ordered (oldest first)
- Last element is the current prompt

### 4.2 Prompt Caching

**Implementation:** Two types available

#### Implicit Caching (Automatic)
- Enabled by default on Gemini 2.5 models (as of May 8, 2025)
- No code changes required
- Automatic cost savings on cache hits

#### Explicit Caching (Manual)
**Step 1: Create Cache**
```python
cache = client.caches.create(
    model=model,
    config=types.CreateCachedContentConfig(
        display_name='my_cache',
        system_instruction='Instructions here',
        contents=[content],
        ttl="300s"
    )
)
```

**Step 2: Use Cache in Request**
```python
response = client.models.generate_content(
    model=model,
    contents='User query',
    config=types.GenerateContentConfig(
        cached_content=cache.name
    )
)
```

**Minimum Token Requirements:**
| Model | Minimum Tokens |
|-------|---------------|
| Gemini 2.5 Flash | 1,024 |
| Gemini 2.5 Pro | 4,096 |
| Gemini 3 Pro Preview | 2,048 |

**Pricing:**
- Gemini 2.5: 90% discount on cached tokens
- Gemini 2.0: 75% discount on cached tokens

**Sources:**
- [Context Caching Guide](https://ai.google.dev/gemini-api/docs/caching)
- [Multi-turn Chat - Firebase](https://firebase.google.com/docs/ai-logic/chat)

---

## 5. Comparison Matrix

### 5.1 Multi-Turn Format Comparison

| Provider | Messages Key | Role Names | Content Structure |
|----------|-------------|------------|-------------------|
| Anthropic | `messages` | user, assistant | `content` (string or blocks) |
| OpenAI | `messages` | system, user, assistant | `content` (string) |
| DeepSeek | `messages` | user, assistant | `content` (string) |
| Gemini | `contents` | user, **model** | `parts: [{text: "..."}]` |

### 5.2 Caching Comparison

| Provider | Type | Effort Required | Min Tokens | Savings |
|----------|------|----------------|------------|---------|
| Anthropic | Explicit | Add cache_control | Context-dependent | 90% read |
| OpenAI | Automatic | None | 1,024 | 50% |
| DeepSeek | Automatic | None | 64 | 90% |
| Gemini | Both | None (implicit) or API call (explicit) | 1,024-4,096 | 75-90% |

---

## 6. Implementation Implications for llm.rs

### 6.1 Current State (from ULTRATHINK)
- `LLMRequest` only supports single-turn: `system`, `user`, `model` strings
- No message history support
- No caching implementation

### 6.2 Required Changes

#### A. New Message Types
```rust
#[derive(Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,  // Provider-specific role name
    pub content: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MultiTurnRequest {
    pub system: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub enable_caching: bool,
}
```

#### B. Provider-Specific Role Mapping
```rust
fn map_role_for_provider(role: &str, provider: &str) -> String {
    match (role, provider) {
        ("assistant", "gemini") => "model".to_string(),
        _ => role.to_string(),
    }
}
```

#### C. Gemini-Specific Content Transformation
```rust
// Transform messages to Gemini "contents" with "parts" structure
fn to_gemini_contents(messages: &[ChatMessage]) -> Vec<GeminiContent> {
    messages.iter().map(|m| GeminiContent {
        role: map_role_for_provider(&m.role, "gemini"),
        parts: vec![GeminiPart { text: m.content.clone() }],
    }).collect()
}
```

#### D. Caching Strategy by Provider
| Provider | Implementation |
|----------|---------------|
| Anthropic | Add `cache_control` to system message content block |
| OpenAI | No changes (automatic) |
| DeepSeek | No changes (automatic) |
| Gemini | For explicit: Use Context Caching API |

---

## 7. Risk Assessment

### 7.1 High Risk Items
1. **Gemini Role Name Difference** - Must transform "assistant" → "model"
2. **Gemini Content Structure** - Requires different JSON structure entirely
3. **Anthropic Cache TTL** - Must manage expiration and headers

### 7.2 Medium Risk Items
1. **Mixed History** - Need to validate alternating user/assistant roles
2. **Token Counting** - May need to estimate for cache eligibility

### 7.3 Low Risk Items
1. **OpenAI Caching** - Automatic, no implementation needed
2. **DeepSeek Format** - OpenAI-compatible

---

## 8. Next Steps (PHASE 4: PLAN)

1. Design unified `MultiTurnRequest` struct
2. Create provider-specific serialization traits
3. Add caching configuration options
4. Design cache monitoring/metrics
5. Create abstraction layer for role/content transformation

---

**Document Version:** 1.0
**Last Updated:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint)
