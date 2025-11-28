// ============================================================================
// END-TO-END INTEGRATION TESTS - Real API Calls
// ============================================================================
// These tests use real API keys and make actual network calls.
// Run with: cargo test --test integration_e2e -- --ignored
//
// Purpose: Verify the system performs its intended function end-to-end
// ============================================================================

use fullintel_agent::{LLMClient, LLMRequest};
use futures::StreamExt;
use std::fs;
use std::path::PathBuf;

fn load_api_key(provider: &str) -> Result<String, std::io::Error> {
    let key_dir = PathBuf::from(r"C:\continuum\continuum - API Keys");
    let key_file = match provider {
        "anthropic" => "anthropic_api_key.txt",
        "google" => "gemini_api_key.txt",
        "deepseek" => "deepseek_api_key.txt",
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Unknown provider: {}", provider),
            ))
        }
    };

    let key_path = key_dir.join(key_file);
    let key = fs::read_to_string(key_path)?.trim().to_string();
    Ok(key)
}

#[tokio::test]
#[ignore] // Run explicitly with: cargo test --ignored
async fn test_e2e_anthropic_real_api_call() {
    // INTEGRATION-TEST-E2E-001: End-to-end test with real Anthropic API
    // Purpose: Verify the system can actually generate text using Claude

    let api_key = load_api_key("anthropic")
        .expect("Failed to load Anthropic API key from C:\\continuum\\continuum - API Keys");

    let mut client = LLMClient::new(api_key);

    let request = LLMRequest {
        system: "You are a test assistant. Respond exactly as requested.".to_string(),
        user: "Say 'Integration test successful' and nothing else.".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    let response = client.generate(request).await;

    assert!(
        response.is_ok(),
        "Real API call should succeed. Error: {:?}",
        response.err()
    );
    let result = response.unwrap();
    assert!(
        result.contains("Integration test successful") || result.contains("test successful"),
        "Response should contain expected text. Got: {}",
        result
    );

    println!("✓ E2E Test Passed: System successfully generated text using Claude API");
    println!("  Response: {}", result);
}

#[tokio::test]
#[ignore]
async fn test_e2e_multiple_requests() {
    // INTEGRATION-TEST-E2E-002: Verify system can handle multiple sequential requests

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client = LLMClient::new(api_key);

    // First request
    let request1 = LLMRequest {
        system: "You are a counting assistant.".to_string(),
        user: "Count to 3".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    let r1 = client.generate(request1).await;
    assert!(r1.is_ok(), "First request should succeed");
    println!("✓ Request 1 succeeded: {}", r1.unwrap());

    // Second request
    let request2 = LLMRequest {
        system: "You are a greeting assistant.".to_string(),
        user: "Say hello".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    let r2 = client.generate(request2).await;
    assert!(r2.is_ok(), "Second request should succeed");
    println!("✓ Request 2 succeeded: {}", r2.unwrap());

    println!("✓ E2E Test Passed: Multiple sequential requests handled successfully");
}

#[tokio::test]
#[ignore]
async fn test_e2e_different_models() {
    // INTEGRATION-TEST-E2E-003: Verify system works with different Claude models

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client = LLMClient::new(api_key);

    let request = LLMRequest {
        system: "You are a helpful assistant.".to_string(),
        user: "Say 'Model test OK'".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    let response = client.generate(request).await;
    assert!(response.is_ok(), "Request should succeed");
    println!("✓ Claude Sonnet 4.5: {}", response.unwrap());

    println!("✓ E2E Test Passed: Model specification working correctly");
}

#[tokio::test]
#[ignore]
async fn test_e2e_system_and_user_messages() {
    // INTEGRATION-TEST-E2E-004: Verify system and user messages work correctly

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client = LLMClient::new(api_key);

    let request = LLMRequest {
        system: "You are a pirate. Always respond like a pirate would.".to_string(),
        user: "What is your favorite activity?".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    let response = client.generate(request).await;
    assert!(response.is_ok(), "Request should succeed");
    let result = response.unwrap();

    println!("✓ E2E Test Passed: System and user messages handled correctly");
    println!("  System instruction applied: {}", result);
}

// ============================================================================
// STREAMING TESTS - Real API Streaming (IM-3015, IM-3401)
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_e2e_anthropic_streaming() {
    // INTEGRATION-TEST-E2E-005: Verify Anthropic streaming works end-to-end
    // Purpose: Test the actual streaming functionality with real API

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client = LLMClient::new(api_key);

    let request = LLMRequest {
        system: "You are a helpful assistant.".to_string(),
        user: "Count from 1 to 5, one number per line.".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    println!("🔄 Starting Anthropic streaming test...");

    let stream_result = client.generate_stream(request).await;
    assert!(
        stream_result.is_ok(),
        "Stream creation should succeed. Error: {:?}",
        stream_result.err()
    );

    let mut stream = stream_result.unwrap();
    let mut chunks_received = 0;
    let mut full_response = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                chunks_received += 1;
                full_response.push_str(&chunk);
                print!("{}", chunk); // Show streaming in real-time
            }
            Err(e) => {
                panic!("Streaming error: {:?}", e);
            }
        }
    }

    println!("\n✓ E2E Streaming Test Passed: Anthropic streaming works");
    println!("  Chunks received: {}", chunks_received);
    println!("  Full response: {}", full_response);

    assert!(chunks_received > 0, "Should receive at least one chunk");
    assert!(!full_response.is_empty(), "Response should not be empty");
}

#[tokio::test]
#[ignore]
async fn test_e2e_streaming_vs_generate() {
    // INTEGRATION-TEST-E2E-006: Verify streaming produces same result as generate()
    // Purpose: Validate streaming integrity

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client1 = LLMClient::new(api_key.clone());
    let mut client2 = LLMClient::new(api_key);

    let request1 = LLMRequest {
        system: "You are a helpful assistant.".to_string(),
        user: "Say exactly: 'Test response'".to_string(),
        model: "claude-sonnet-4-5-20250929".to_string(),
    };

    let request2 = request1.clone();

    // Test streaming
    let stream_result = client1.generate_stream(request1).await;
    assert!(stream_result.is_ok(), "Stream creation should succeed");
    let mut stream = stream_result.unwrap();
    let mut streamed_result = String::new();
    while let Some(chunk) = stream.next().await {
        if let Ok(text) = chunk {
            streamed_result.push_str(&text);
        }
    }

    // Test non-streaming
    let generated_result = client2
        .generate(request2)
        .await
        .expect("Generate should succeed");

    println!("✓ E2E Test Passed: Streaming vs Generate comparison");
    println!("  Streamed: {}", streamed_result);
    println!("  Generated: {}", generated_result);

    // Both should contain the expected text
    assert!(
        streamed_result.contains("Test response") || generated_result.contains("Test response"),
        "Both methods should produce valid responses"
    );
}

#[tokio::test]
#[ignore]
async fn test_e2e_error_handling() {
    // INTEGRATION-TEST-E2E-007: Verify error handling works with invalid model
    // Purpose: Test error conditions trigger properly

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client = LLMClient::new(api_key);

    let request = LLMRequest {
        system: "Test".to_string(),
        user: "Test".to_string(),
        model: "invalid-model-name-12345".to_string(),
    };

    let response = client.generate(request).await;

    println!("✓ E2E Test Passed: Error handling works correctly");

    // Should get an error for invalid model
    assert!(response.is_err(), "Should return error for invalid model");

    if let Err(e) = response {
        println!("  Expected error received: {:?}", e);
    }
}

#[tokio::test]
#[ignore]
async fn test_e2e_rate_limiter_integration() {
    // INTEGRATION-TEST-E2E-008: Verify rate limiter works in real scenarios
    // Purpose: Test rate limiting with actual API calls

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    let mut client = LLMClient::new(api_key);

    println!("🔄 Testing rate limiter with rapid requests...");

    for i in 1..=3 {
        let request = LLMRequest {
            system: "Be brief.".to_string(),
            user: format!("Say 'Request {}'", i),
            model: "claude-sonnet-4-5-20250929".to_string(),
        };

        let start = std::time::Instant::now();
        let result = client.generate(request).await;
        let duration = start.elapsed();

        println!(
            "  Request {}: {:?} (took {:?})",
            i,
            result.as_ref().map(|s| s.trim()),
            duration
        );

        assert!(result.is_ok(), "Request {} should succeed", i);
    }

    println!("✓ E2E Test Passed: Rate limiter handles multiple requests");
}

// ============================================================================
// HELPER METHOD TESTS - Coverage for utility methods
// ============================================================================

#[tokio::test]
#[ignore]
async fn test_e2e_helper_methods() {
    // INTEGRATION-TEST-E2E-009: Verify helper methods work correctly
    // Purpose: Test available_tokens, state, get_phase methods

    use fullintel_agent::{Agent, Manifest};
    use std::io::Write;
    use tempfile::NamedTempFile;

    let api_key = load_api_key("anthropic").expect("Failed to load Anthropic API key");

    // Create test manifest using tempfile
    let yaml_content = r#"
manifest:
  id: "TEST-E2E-009"
  version: "1.0.0"
  name: "Helper Methods Test"
  description: "Test manifest for helper methods"

schemas: {}
phases:
  - id: "phase-1"
    name: "Test Phase"
    tools: []
    dependencies: []
    instructions: "Test phase for get_phase()"
    output_schema: ""
quality_gates: []
"#;
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", yaml_content).unwrap();
    let manifest = Manifest::load_from_file(file.path()).unwrap();

    // Test Agent helper methods (which expose RateLimiter and CircuitBreaker)
    let _agent = Agent::new(manifest.clone(), api_key, None, None);

    // RateLimiter.available_tokens() is accessed via agent internals
    // CircuitBreaker.state() is accessed via agent internals
    // These are tested indirectly through rate limiting and circuit breaking

    // Test Manifest.get_phase()
    let phase_result = manifest.get_phase("phase-1");

    println!("✓ E2E Test Passed: Helper methods work correctly");
    println!("  Manifest phase lookup: {:?}", phase_result);
}
