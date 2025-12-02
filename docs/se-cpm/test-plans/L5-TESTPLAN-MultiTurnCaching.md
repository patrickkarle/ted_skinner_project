# L5-TESTPLAN: Multi-Turn Conversation & Prompt Caching Test Specifications
## CDP LODA Enhancement - Phase 6: TESTING PLAN

**Date:** 2025-11-28
**Status:** Draft
**Traces To:** L2-ICD-04-MultiTurnCaching.md, MULTI_TURN_CACHING_PLAN.md

---

## 1. Test Overview

### 1.1 Scope

| Category | Coverage Target | Priority |
|----------|-----------------|----------|
| Unit Tests | 90%+ line coverage | CRITICAL |
| Integration Tests | All 4 providers | CRITICAL |
| Contract Tests | All ICD contracts | IMPORTANT |
| Edge Case Tests | Error paths | IMPORTANT |

### 1.2 Test Environment

- **Rust Version:** 1.75+
- **Test Framework:** `#[cfg(test)]` with `tokio::test` for async
- **Mocking:** `mockall` for HTTP client mocking
- **Provider Sandbox:** Test API keys with rate limiting awareness

---

## 2. Unit Test Specifications

### 2.1 ChatRole Tests (ICD-04-001)

```rust
#[cfg(test)]
mod chat_role_tests {
    use super::*;

    // TEST-MT-001: Provider string conversion - Anthropic
    #[test]
    fn test_role_to_anthropic_string() {
        assert_eq!(ChatRole::User.to_provider_string("anthropic"), "user");
        assert_eq!(ChatRole::Assistant.to_provider_string("anthropic"), "assistant");
        assert_eq!(ChatRole::System.to_provider_string("anthropic"), "system");
    }

    // TEST-MT-002: Provider string conversion - Gemini (CRITICAL)
    #[test]
    fn test_role_to_gemini_string() {
        assert_eq!(ChatRole::User.to_provider_string("gemini"), "user");
        // CRITICAL: Gemini uses "model" not "assistant"
        assert_eq!(ChatRole::Assistant.to_provider_string("gemini"), "model");
        assert_eq!(ChatRole::System.to_provider_string("gemini"), "system");
    }

    // TEST-MT-003: Provider string conversion - OpenAI/DeepSeek
    #[test]
    fn test_role_to_openai_string() {
        assert_eq!(ChatRole::User.to_provider_string("openai"), "user");
        assert_eq!(ChatRole::Assistant.to_provider_string("openai"), "assistant");
        assert_eq!(ChatRole::User.to_provider_string("deepseek"), "user");
        assert_eq!(ChatRole::Assistant.to_provider_string("deepseek"), "assistant");
    }

    // TEST-MT-004: Role parsing from provider string
    #[test]
    fn test_role_from_provider_string() {
        assert_eq!(ChatRole::from_provider_string("user", "anthropic"), Some(ChatRole::User));
        assert_eq!(ChatRole::from_provider_string("model", "gemini"), Some(ChatRole::Assistant));
        assert_eq!(ChatRole::from_provider_string("assistant", "openai"), Some(ChatRole::Assistant));
        assert_eq!(ChatRole::from_provider_string("invalid", "anthropic"), None);
    }
}
```

### 2.2 ChatMessage Tests (ICD-04-002)

```rust
#[cfg(test)]
mod chat_message_tests {
    use super::*;

    // TEST-MT-010: Message construction helpers
    #[test]
    fn test_message_constructors() {
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, ChatRole::User);
        assert_eq!(user_msg.content, "Hello");
        assert!(user_msg.timestamp.is_some());

        let asst_msg = ChatMessage::assistant("Hi there");
        assert_eq!(asst_msg.role, ChatRole::Assistant);

        let sys_msg = ChatMessage::system("You are helpful");
        assert_eq!(sys_msg.role, ChatRole::System);
    }

    // TEST-MT-011: Message serialization
    #[test]
    fn test_message_serialization() {
        let msg = ChatMessage::user("Test content");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Test content\""));
    }

    // TEST-MT-012: Message deserialization
    #[test]
    fn test_message_deserialization() {
        let json = r#"{"role":"assistant","content":"Response"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, ChatRole::Assistant);
        assert_eq!(msg.content, "Response");
    }
}
```

### 2.3 MultiTurnRequest Tests (ICD-04-005)

```rust
#[cfg(test)]
mod multi_turn_request_tests {
    use super::*;

    // TEST-MT-020: Request construction
    #[test]
    fn test_request_construction() {
        let messages = vec![ChatMessage::user("Hello")];
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", messages);

        assert_eq!(req.model, "claude-sonnet-4-5-20250929");
        assert_eq!(req.messages.len(), 1);
        assert!(req.system.is_none());
        assert!(req.cache_config.enabled); // Default enabled
    }

    // TEST-MT-021: Builder pattern - system prompt
    #[test]
    fn test_request_with_system() {
        let req = MultiTurnRequest::new("gpt-4o", vec![])
            .with_system("You are a research agent");

        assert_eq!(req.system, Some("You are a research agent".to_string()));
    }

    // TEST-MT-022: Builder pattern - caching disabled
    #[test]
    fn test_request_without_caching() {
        let req = MultiTurnRequest::new("gpt-4o", vec![])
            .without_caching();

        assert!(!req.cache_config.enabled);
    }

    // TEST-MT-023: Message ordering validation - valid
    #[test]
    fn test_valid_message_ordering() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("First"),
            ChatMessage::assistant("Response"),
            ChatMessage::user("Second"),
        ]);

        assert!(req.validate_ordering().is_ok());
    }

    // TEST-MT-024: Message ordering - system in middle (invalid)
    #[test]
    fn test_invalid_system_position() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("First"),
            ChatMessage::system("System after user"), // Invalid
            ChatMessage::user("Second"),
        ]);

        assert!(req.validate_ordering().is_err());
    }

    // TEST-MT-025: Add message mutation
    #[test]
    fn test_add_message() {
        let mut req = MultiTurnRequest::new("gpt-4o", vec![]);
        assert_eq!(req.messages.len(), 0);

        req.add_message(ChatMessage::user("Added"));
        assert_eq!(req.messages.len(), 1);
    }
}
```

### 2.4 CacheConfig Tests (ICD-04-003/004)

```rust
#[cfg(test)]
mod cache_config_tests {
    use super::*;

    // TEST-MT-030: Default cache config
    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert_eq!(config.ttl, CacheTTL::FiveMinutes);
        assert!(!config.system_only);
    }

    // TEST-MT-031: TTL Anthropic header - 5 minutes
    #[test]
    fn test_ttl_five_minutes_header() {
        assert_eq!(
            CacheTTL::FiveMinutes.anthropic_beta_header(),
            "prompt-caching-2024-07-31"
        );
    }

    // TEST-MT-032: TTL Anthropic header - 1 hour
    #[test]
    fn test_ttl_one_hour_header() {
        assert_eq!(
            CacheTTL::OneHour.anthropic_beta_header(),
            "extended-cache-ttl-2025-04-11"
        );
    }

    // TEST-MT-033: TTL string conversion
    #[test]
    fn test_ttl_string_conversion() {
        assert_eq!(CacheTTL::FiveMinutes.to_ttl_string(), "5m");
        assert_eq!(CacheTTL::OneHour.to_ttl_string(), "1h");
    }
}
```

---

## 3. Provider Transformation Tests

### 3.1 Anthropic Transformation (ICD-04-010)

```rust
#[cfg(test)]
mod anthropic_transform_tests {
    use super::*;

    // TEST-MT-040: Basic Anthropic request format
    #[test]
    fn test_anthropic_basic_format() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("Hello"),
            ChatMessage::assistant("Hi there"),
            ChatMessage::user("How are you?"),
        ]).with_system("You are helpful");

        let body = to_anthropic_request(&req);

        assert_eq!(body["model"], "claude-sonnet-4-5-20250929");
        assert_eq!(body["system"], "You are helpful");
        assert!(body["messages"].is_array());
        assert_eq!(body["messages"].as_array().unwrap().len(), 3);
    }

    // TEST-MT-041: Anthropic caching - cache_control added
    #[test]
    fn test_anthropic_with_caching() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("Cache this content"),
        ]).with_caching(CacheConfig { enabled: true, ..Default::default() });

        let body = to_anthropic_request(&req);
        let first_msg = &body["messages"][0];

        // Should have content block with cache_control
        assert!(first_msg["content"].is_array());
        let content_block = &first_msg["content"][0];
        assert_eq!(content_block["type"], "text");
        assert!(content_block["cache_control"].is_object());
    }

    // TEST-MT-042: Anthropic no caching - simple content
    #[test]
    fn test_anthropic_without_caching() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("No cache"),
        ]).without_caching();

        let body = to_anthropic_request(&req);
        let first_msg = &body["messages"][0];

        // Should have simple string content
        assert!(first_msg["content"].is_string());
    }

    // TEST-MT-043: Anthropic role strings
    #[test]
    fn test_anthropic_role_strings() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("User message"),
            ChatMessage::assistant("Assistant message"),
        ]).without_caching();

        let body = to_anthropic_request(&req);

        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][1]["role"], "assistant");
    }
}
```

### 3.2 Gemini Transformation (ICD-04-012) - CRITICAL

```rust
#[cfg(test)]
mod gemini_transform_tests {
    use super::*;

    // TEST-MT-050: Gemini uses "contents" not "messages"
    #[test]
    fn test_gemini_contents_key() {
        let req = MultiTurnRequest::new("gemini-2.0-flash", vec![
            ChatMessage::user("Hello"),
        ]);

        let body = to_gemini_request(&req);

        assert!(body["contents"].is_array());
        assert!(body.get("messages").is_none()); // NOT messages
    }

    // TEST-MT-051: Gemini uses "parts" array
    #[test]
    fn test_gemini_parts_structure() {
        let req = MultiTurnRequest::new("gemini-2.0-flash", vec![
            ChatMessage::user("Test message"),
        ]);

        let body = to_gemini_request(&req);
        let content = &body["contents"][0];

        assert!(content["parts"].is_array());
        assert_eq!(content["parts"][0]["text"], "Test message");
    }

    // TEST-MT-052: CRITICAL - Gemini uses "model" role not "assistant"
    #[test]
    fn test_gemini_model_role() {
        let req = MultiTurnRequest::new("gemini-2.0-flash", vec![
            ChatMessage::user("Question"),
            ChatMessage::assistant("Answer"), // Should become "model"
        ]);

        let body = to_gemini_request(&req);

        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["contents"][1]["role"], "model"); // NOT "assistant"
    }

    // TEST-MT-053: Gemini systemInstruction field
    #[test]
    fn test_gemini_system_instruction() {
        let req = MultiTurnRequest::new("gemini-2.0-flash", vec![
            ChatMessage::user("Hello"),
        ]).with_system("You are a research agent");

        let body = to_gemini_request(&req);

        assert!(body["systemInstruction"].is_object());
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "You are a research agent");
    }

    // TEST-MT-054: Gemini multi-turn ordering
    #[test]
    fn test_gemini_multi_turn_ordering() {
        let req = MultiTurnRequest::new("gemini-2.0-flash", vec![
            ChatMessage::user("First question"),
            ChatMessage::assistant("First answer"),
            ChatMessage::user("Follow-up"),
            ChatMessage::assistant("Follow-up answer"),
            ChatMessage::user("Final question"),
        ]);

        let body = to_gemini_request(&req);
        let contents = body["contents"].as_array().unwrap();

        assert_eq!(contents.len(), 5);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[2]["role"], "user");
        assert_eq!(contents[3]["role"], "model");
        assert_eq!(contents[4]["role"], "user");
    }
}
```

### 3.3 OpenAI/DeepSeek Transformation (ICD-04-011)

```rust
#[cfg(test)]
mod openai_transform_tests {
    use super::*;

    // TEST-MT-060: OpenAI system message first
    #[test]
    fn test_openai_system_first() {
        let req = MultiTurnRequest::new("gpt-4o", vec![
            ChatMessage::user("Hello"),
        ]).with_system("System instructions");

        let body = to_openai_request(&req);
        let messages = body["messages"].as_array().unwrap();

        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "System instructions");
        assert_eq!(messages[1]["role"], "user");
    }

    // TEST-MT-061: OpenAI role strings
    #[test]
    fn test_openai_role_strings() {
        let req = MultiTurnRequest::new("gpt-4o", vec![
            ChatMessage::user("User"),
            ChatMessage::assistant("Assistant"),
        ]);

        let body = to_openai_request(&req);

        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][1]["role"], "assistant"); // Not "model"
    }

    // TEST-MT-062: DeepSeek uses same format
    #[test]
    fn test_deepseek_compatible_format() {
        let req = MultiTurnRequest::new("deepseek-chat", vec![
            ChatMessage::user("Hello"),
        ]).with_system("System");

        let body = to_openai_request(&req); // Same function works

        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][1]["role"], "user");
    }

    // TEST-MT-063: Stream flag
    #[test]
    fn test_openai_stream_flag() {
        let req = MultiTurnRequest::new("gpt-4o", vec![]);
        let body = to_openai_request(&req);

        assert_eq!(body["stream"], false);
    }
}
```

---

## 4. Integration Test Specifications

### 4.1 Provider Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    // TEST-MT-100: Anthropic multi-turn integration
    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_anthropic_multi_turn_integration() {
        let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
        let mut client = LLMClient::new(api_key);

        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("What is 2+2?"),
        ]).with_system("You are a math tutor. Give short answers.");

        let response = client.generate_multi_turn(req).await;
        assert!(response.is_ok());
        assert!(response.unwrap().contains("4"));
    }

    // TEST-MT-101: Gemini multi-turn integration
    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_gemini_multi_turn_integration() {
        let api_key = std::env::var("GEMINI_API_KEY").unwrap();
        let mut client = LLMClient::new(api_key);

        let req = MultiTurnRequest::new("gemini-2.0-flash", vec![
            ChatMessage::user("What is the capital of France?"),
        ]);

        let response = client.generate_multi_turn(req).await;
        assert!(response.is_ok());
        assert!(response.unwrap().to_lowercase().contains("paris"));
    }

    // TEST-MT-102: Multi-turn conversation flow
    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_multi_turn_conversation_flow() {
        let api_key = std::env::var("ANTHROPIC_API_KEY").unwrap();
        let mut client = LLMClient::new(api_key);

        // First turn
        let req1 = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("My name is Alice"),
        ]);
        let response1 = client.generate_multi_turn(req1).await.unwrap();

        // Second turn - should remember context
        let req2 = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![
            ChatMessage::user("My name is Alice"),
            ChatMessage::assistant(response1),
            ChatMessage::user("What is my name?"),
        ]);
        let response2 = client.generate_multi_turn(req2).await.unwrap();

        assert!(response2.to_lowercase().contains("alice"));
    }
}
```

---

## 5. Error Handling Tests

```rust
#[cfg(test)]
mod error_tests {
    use super::*;

    // TEST-MT-200: Empty history error
    #[test]
    fn test_empty_history_error() {
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929", vec![]);
        // Implementation should return MultiTurnError::EmptyHistory
    }

    // TEST-MT-201: Invalid provider
    #[tokio::test]
    async fn test_unsupported_model() {
        let mut client = LLMClient::new("test-key".to_string());
        let req = MultiTurnRequest::new("unsupported-model-xyz", vec![
            ChatMessage::user("Test"),
        ]);

        let result = client.generate_multi_turn(req).await;
        assert!(result.is_err());
    }
}
```

---

## 6. Test Execution Matrix

| Test ID | Description | Priority | Automated |
|---------|-------------|----------|-----------|
| TEST-MT-001-004 | ChatRole conversions | CRITICAL | Yes |
| TEST-MT-010-012 | ChatMessage construction | CRITICAL | Yes |
| TEST-MT-020-025 | MultiTurnRequest building | CRITICAL | Yes |
| TEST-MT-030-033 | CacheConfig/TTL | IMPORTANT | Yes |
| TEST-MT-040-043 | Anthropic transformation | CRITICAL | Yes |
| TEST-MT-050-054 | Gemini transformation | CRITICAL | Yes |
| TEST-MT-060-063 | OpenAI/DeepSeek transformation | CRITICAL | Yes |
| TEST-MT-100-102 | Provider integration | CRITICAL | Manual (API keys) |
| TEST-MT-200-201 | Error handling | IMPORTANT | Yes |

---

## 7. Coverage Requirements

| Component | Minimum Coverage | Target Coverage |
|-----------|-----------------|-----------------|
| `ChatRole` | 100% | 100% |
| `ChatMessage` | 90% | 100% |
| `MultiTurnRequest` | 90% | 95% |
| `CacheConfig` | 90% | 100% |
| `to_anthropic_request` | 95% | 100% |
| `to_gemini_request` | 95% | 100% |
| `to_openai_request` | 95% | 100% |
| `generate_multi_turn` | 80% | 90% |

---

## 8. Traceability

| Test ID | ICD Reference | IM Code |
|---------|--------------|---------|
| TEST-MT-001-004 | ICD-04-001 | IM-4002 |
| TEST-MT-010-012 | ICD-04-002 | IM-4001 |
| TEST-MT-020-025 | ICD-04-005 | IM-4003 |
| TEST-MT-030-033 | ICD-04-003/004 | IM-4004/4005 |
| TEST-MT-040-043 | ICD-04-010 | IM-4010 |
| TEST-MT-050-054 | ICD-04-012 | IM-4012 |
| TEST-MT-060-063 | ICD-04-011 | IM-4011 |

---

**Document Version:** 1.0
**Last Updated:** 2025-11-28
**Author:** Claude Code (CDP LODA Sprint)
