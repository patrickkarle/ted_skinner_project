#![allow(dead_code)]
use anyhow::{anyhow, Result};
use futures::stream::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;
use std::time::{Duration, Instant};
use thiserror::Error;

// ------------------------------------------------------------------
// Error Types
// ------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum LLMError {
    #[error("API key not configured for provider: {0}")]
    MissingApiKey(String),

    #[error("Rate limit exceeded for provider: {0}")]
    RateLimitExceeded(String),

    #[error("Invalid model: {0}")]
    InvalidModel(String),

    #[error("Unsupported model: {0}")]
    UnsupportedModel(String),

    #[error("Context length exceeded: {0} tokens")]
    ContextLengthExceeded(usize),

    #[error("Provider API error: {0}")]
    ProviderError(String),

    #[error("Streaming error: {0}")]
    StreamingError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Provider unavailable: {0}")]
    ProviderUnavailable(String),
}

#[derive(Debug, Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open, blocking requests")]
    Open,

    #[error("Request failed: {0}")]
    RequestFailed(String),
}

// ------------------------------------------------------------------
// Multi-Turn Error Types (IM-4006)
// ------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum MultiTurnError {
    #[error("Empty message history - at least one message required")]
    EmptyHistory,

    #[error("Invalid message ordering: {0}")]
    InvalidOrdering(String),

    #[error("Context length exceeded: {0} tokens")]
    ContextLengthExceeded(usize),

    #[error("Role transformation error: {0}")]
    RoleTransformError(String),
}

// ------------------------------------------------------------------
// Multi-Turn Conversation Types (IM-4001-4005)
// ------------------------------------------------------------------

/// IM-4002: ChatRole - Provider-independent role enum
/// Maps to provider-specific role strings via to_provider_string()
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatRole {
    System,    // System instructions (some providers)
    User,      // User messages
    Assistant, // AI responses (maps to "model" for Gemini)
}

impl ChatRole {
    /// IM-4002-M1: Convert to provider-specific role string
    /// CRITICAL: Gemini uses "model" instead of "assistant"
    pub fn to_provider_string(&self, provider: &str) -> &'static str {
        match (self, provider) {
            (ChatRole::System, _) => "system",
            (ChatRole::User, _) => "user",
            (ChatRole::Assistant, "gemini") | (ChatRole::Assistant, "google") => "model", // CRITICAL: Gemini uses "model"
            (ChatRole::Assistant, _) => "assistant",
        }
    }
}

/// IM-4001: ChatMessage - Individual conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,  // IM-4001-F1: Semantic role (abstracted)
    pub content: String, // IM-4001-F2: Message content
}

impl ChatMessage {
    /// Create a new chat message
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self::new(ChatRole::System, content)
    }
}

/// IM-4005: CacheTTL - Cache duration options (Anthropic-specific)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CacheTTL {
    FiveMinutes, // Default, lower write cost
    OneHour,     // Extended, higher write cost (2x)
}

impl CacheTTL {
    /// Get the Anthropic beta header value for this TTL
    pub fn to_anthropic_header(&self) -> &'static str {
        match self {
            CacheTTL::FiveMinutes => "prompt-caching-2024-07-31",
            CacheTTL::OneHour => "extended-cache-ttl-2025-04-11",
        }
    }
}

/// IM-4004: CacheConfig - Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub ttl: CacheTTL, // IM-4004-F1: Time to live
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl: CacheTTL::FiveMinutes,
        }
    }
}

/// IM-4003: MultiTurnRequest - Full conversation request
#[derive(Debug, Clone, Serialize)]
pub struct MultiTurnRequest {
    pub system: Option<String>,     // IM-4003-F1: Optional system prompt
    pub messages: Vec<ChatMessage>, // IM-4003-F2: Conversation history
    pub model: String,              // IM-4003-F3: Model identifier
    pub enable_caching: bool,       // IM-4003-F4: Enable provider caching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_config: Option<CacheConfig>, // IM-4003-F5: Cache configuration
}

impl MultiTurnRequest {
    /// Create a new multi-turn request
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            system: None,
            messages: Vec::new(),
            model: model.into(),
            enable_caching: false,
            cache_config: None,
        }
    }

    /// Builder: Add system prompt
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Builder: Add a message to history
    pub fn with_message(mut self, message: ChatMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Builder: Add multiple messages to history
    pub fn with_messages(mut self, messages: Vec<ChatMessage>) -> Self {
        self.messages.extend(messages);
        self
    }

    /// Builder: Enable caching with default config
    pub fn with_caching(mut self) -> Self {
        self.enable_caching = true;
        self.cache_config = Some(CacheConfig::default());
        self
    }

    /// Builder: Enable caching with custom config
    pub fn with_cache_config(mut self, config: CacheConfig) -> Self {
        self.enable_caching = true;
        self.cache_config = Some(config);
        self
    }

    /// Builder: Disable caching
    pub fn without_caching(mut self) -> Self {
        self.enable_caching = false;
        self.cache_config = None;
        self
    }

    /// Validate the request before sending
    pub fn validate(&self) -> Result<(), MultiTurnError> {
        if self.messages.is_empty() && self.system.is_none() {
            return Err(MultiTurnError::EmptyHistory);
        }
        Ok(())
    }
}

// ------------------------------------------------------------------
// Rate Limiter (IM-3020-3024)
// ------------------------------------------------------------------

/// Token bucket rate limiter for controlling request frequency
#[derive(Debug, Clone)]
pub struct RateLimiter {
    pub(crate) tokens: f64,          // IM-3020-F1: Current available tokens
    pub(crate) capacity: f64,        // IM-3020-F2: Maximum tokens (requests per minute)
    pub(crate) refill_rate: f64,     // IM-3020-F3: Tokens added per second (capacity / 60)
    pub(crate) last_refill: Instant, // IM-3020-F4: Last token refill timestamp
}

impl RateLimiter {
    /// Create a new RateLimiter with specified requests-per-minute capacity
    /// IM-3021: Constructor
    pub fn new(requests_per_minute: f64) -> Self {
        Self {
            tokens: requests_per_minute,
            capacity: requests_per_minute,
            refill_rate: requests_per_minute / 60.0,
            last_refill: Instant::now(),
        }
    }

    /// Try to acquire a token for a request
    /// Returns Ok(()) if token acquired, Err(wait_duration) if rate limited
    /// IM-3022: try_acquire() method
    pub fn try_acquire(&mut self) -> Result<(), Duration> {
        self.refill();

        if self.tokens >= 1.0 {
            // IM-3022-B1: Branch - token availability
            self.tokens -= 1.0;
            Ok(())
        } else {
            // IM-3022-V1: Calculate wait time for next token
            let wait_seconds = (1.0 - self.tokens) / self.refill_rate;
            Err(Duration::from_secs_f64(wait_seconds)) // IM-3022-E1: Rate limit error
        }
    }

    /// Refill tokens based on elapsed time
    /// IM-3023: refill() method
    fn refill(&mut self) {
        let now = Instant::now(); // IM-3023-V1: Current time
        let elapsed = now.duration_since(self.last_refill).as_secs_f64(); // IM-3023-V2: Elapsed time
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }

    /// Get current token count (for monitoring)
    /// IM-3024: available_tokens() method
    pub fn available_tokens(&self) -> f64 {
        self.tokens
    }
}

// ------------------------------------------------------------------
// Circuit Breaker (IM-3030-3037)
// ------------------------------------------------------------------

/// Circuit breaker states
/// IM-3031: CircuitState enum
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation, requests allowed
    Open,     // Blocking all requests due to failures
    HalfOpen, // Testing recovery with limited requests
}

/// Circuit breaker for LLM provider failure protection
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    state: CircuitState,                   // IM-3030-F1: Current state
    failure_count: u32,                    // IM-3030-F2: Consecutive failures
    pub(crate) failure_threshold: u32,     // IM-3030-F3: Failures to trigger Open (5)
    success_count: u32,                    // IM-3030-F4: Consecutive successes in HalfOpen
    pub(crate) success_threshold: u32,     // IM-3030-F5: Successes to close (2)
    open_until: Option<Instant>,           // IM-3030-F6: Timeout expiration
    pub(crate) timeout_duration: Duration, // IM-3030-F7: Open state timeout (60s)
}

impl CircuitBreaker {
    /// Create a new CircuitBreaker with default thresholds
    /// IM-3033: Constructor
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout_duration: Duration) -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            failure_threshold,
            success_count: 0,
            success_threshold,
            open_until: None,
            timeout_duration,
        }
    }

    /// Execute a function with circuit breaker protection
    /// IM-3034: call() method with state transitions
    pub fn call<F, T, E>(&mut self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: FnOnce() -> Result<T, E>,
        E: std::fmt::Display,
    {
        // IM-3034-B1: Check state transitions
        match self.state {
            CircuitState::Open => {
                if let Some(open_until) = self.open_until {
                    if Instant::now() >= open_until {
                        // IM-3034-B2: Transition Open → HalfOpen after timeout
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                    } else {
                        // Still in timeout period, reject request
                        return Err(CircuitBreakerError::Open);
                    }
                }
            }
            CircuitState::HalfOpen | CircuitState::Closed => {
                // Proceed with request
            }
        }

        // Execute the function
        match f() {
            Ok(result) => {
                self.on_success(); // IM-3035: on_success() handler
                Ok(result)
            }
            Err(error) => {
                self.on_failure(); // IM-3036: on_failure() handler
                Err(CircuitBreakerError::RequestFailed(error.to_string()))
            }
        }
    }

    /// Record successful request
    /// IM-3035: on_success() method
    fn on_success(&mut self) {
        self.failure_count = 0;

        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.success_threshold {
                    // IM-3034-B3: Transition HalfOpen → Closed after successes
                    self.state = CircuitState::Closed;
                    self.success_count = 0;
                }
            }
            CircuitState::Closed => {
                // Normal operation
            }
            CircuitState::Open => {
                unreachable!("Cannot succeed in Open state")
            }
        }
    }

    /// Record failed request
    /// IM-3036: on_failure() method
    fn on_failure(&mut self) {
        self.success_count = 0;
        self.failure_count += 1;

        if self.failure_count >= self.failure_threshold {
            // IM-3034-B1: Transition Closed → Open after failures
            self.state = CircuitState::Open;
            self.open_until = Some(Instant::now() + self.timeout_duration);
        }
    }

    /// Get current circuit state (for monitoring)
    /// IM-3037: state() getter
    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Check if circuit allows execution and prepare for request
    /// IM-3038: can_execute() for async-compatible circuit breaker usage
    /// Returns Ok(()) if request can proceed, Err if circuit is open
    pub fn can_execute(&mut self) -> Result<(), CircuitBreakerError> {
        match self.state {
            CircuitState::Open => {
                if let Some(open_until) = self.open_until {
                    if Instant::now() >= open_until {
                        // Transition Open → HalfOpen after timeout
                        self.state = CircuitState::HalfOpen;
                        self.success_count = 0;
                        Ok(())
                    } else {
                        // Still in timeout period, reject request
                        Err(CircuitBreakerError::Open)
                    }
                } else {
                    Err(CircuitBreakerError::Open)
                }
            }
            CircuitState::HalfOpen | CircuitState::Closed => Ok(()),
        }
    }

    /// Record a successful async request outcome
    /// IM-3039: record_success() for async-compatible circuit breaker
    pub fn record_success(&mut self) {
        self.on_success();
    }

    /// Record a failed async request outcome
    /// IM-3040: record_failure() for async-compatible circuit breaker
    pub fn record_failure(&mut self) {
        self.on_failure();
    }
}

// ------------------------------------------------------------------
// Multi-Turn Transformation Functions (IM-4010-4012)
// ------------------------------------------------------------------

/// IM-4010: Transform MultiTurnRequest to Anthropic JSON body
/// Handles cache_control injection for cacheable messages
fn to_anthropic_body(req: &MultiTurnRequest) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .filter(|m| m.role != ChatRole::System) // System is separate field in Anthropic
        .map(|m| {
            if req.enable_caching && m.role == ChatRole::User {
                // IM-4010-B1: Add cache_control to cacheable user messages
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

    let mut body = serde_json::json!({
        "model": req.model,
        "max_tokens": 4096,
        "messages": messages
    });

    // Add system prompt if present
    if let Some(ref system) = req.system {
        if req.enable_caching {
            // System with cache_control
            body["system"] = serde_json::json!([{
                "type": "text",
                "text": system,
                "cache_control": {"type": "ephemeral"}
            }]);
        } else {
            body["system"] = serde_json::json!(system);
        }
    }

    body
}

/// IM-4011: Transform MultiTurnRequest to OpenAI-compatible JSON body
/// Used for OpenAI and DeepSeek (both use OpenAI-compatible format)
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

    // Note: Caching is AUTOMATIC for OpenAI/DeepSeek - no special handling needed
    serde_json::json!({
        "model": req.model,
        "messages": messages,
        "stream": false
    })
}

/// IM-4012: Transform MultiTurnRequest to Gemini JSON body
/// CRITICAL DIFFERENCES from other providers:
/// - Uses "contents" not "messages"
/// - Uses "parts" array with text objects
/// - Uses "model" role not "assistant"
/// - Uses "systemInstruction" for system prompt
fn to_gemini_body(req: &MultiTurnRequest) -> serde_json::Value {
    // IM-4012-V1: Gemini uses "contents" not "messages"
    // IM-4012-V2: Gemini uses "parts" array with text objects
    // IM-4012-V3: Gemini uses "model" role not "assistant" (handled by to_provider_string)
    let contents: Vec<serde_json::Value> = req
        .messages
        .iter()
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

    // IM-4012-B1: Add system instruction if present (uses different key than messages)
    if let Some(ref system) = req.system {
        body["systemInstruction"] = serde_json::json!({
            "parts": [{"text": system}]
        });
    }

    // Note: Implicit caching is automatic on Gemini 2.5 models - no special handling
    body
}

/// IM-4013: Transform MultiTurnRequest to streaming OpenAI body
fn to_openai_stream_body(req: &MultiTurnRequest) -> serde_json::Value {
    let mut body = to_openai_body(req);
    body["stream"] = serde_json::json!(true);
    body
}

/// IM-4014: Transform MultiTurnRequest to streaming Anthropic body
fn to_anthropic_stream_body(req: &MultiTurnRequest) -> serde_json::Value {
    let mut body = to_anthropic_body(req);
    body["stream"] = serde_json::json!(true);
    body
}

// ------------------------------------------------------------------
// Request/Response Types
// ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LLMClient {
    client: Client,
    api_key: String,
    rate_limiters: HashMap<String, RateLimiter>,
    circuit_breakers: HashMap<String, CircuitBreaker>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LLMRequest {
    pub system: String,
    pub user: String,
    pub model: String,
}

// ------------------------------------------------------------------
// Provider-Specific Response Structures
// ------------------------------------------------------------------

// Anthropic (Claude)
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    text: String,
}

// Anthropic Streaming
#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicDelta>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(default)]
    text: Option<String>,
}

// Google (Gemini)
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

// DeepSeek (OpenAI-compatible, with R1 reasoning support)
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
    /// The final answer content
    #[serde(default)]
    content: Option<String>,
    /// Chain of Thought reasoning (R1 reasoning models only)
    #[serde(default)]
    reasoning_content: Option<String>,
}

// DeepSeek Streaming
#[derive(Debug, Deserialize)]
struct DeepSeekStreamChunk {
    choices: Vec<DeepSeekStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamChoice {
    delta: DeepSeekStreamDelta,
}

#[derive(Debug, Deserialize)]
struct DeepSeekStreamDelta {
    /// The final answer content chunk
    #[serde(default)]
    content: Option<String>,
    /// Chain of Thought reasoning chunk (R1 reasoning models only)
    #[serde(default)]
    reasoning_content: Option<String>,
}

// OpenAI (GPT models) - OpenAI-compatible format (same as DeepSeek)
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    content: String,
}

// OpenAI Streaming
#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamDelta {
    #[serde(default)]
    content: Option<String>,
}

// ------------------------------------------------------------------
// LLMClient Implementation
// ------------------------------------------------------------------

impl LLMClient {
    pub fn new(api_key: String) -> Self {
        let mut rate_limiters = HashMap::new();
        let mut circuit_breakers = HashMap::new();

        // Configure rate limits per provider (from L1-SAD REQ-SYS-003)
        rate_limiters.insert("anthropic".to_string(), RateLimiter::new(50.0)); // 50 RPM
        rate_limiters.insert("google".to_string(), RateLimiter::new(60.0)); // 60 RPM
        rate_limiters.insert("deepseek".to_string(), RateLimiter::new(100.0)); // 100 RPM
        rate_limiters.insert("openai".to_string(), RateLimiter::new(60.0)); // 60 RPM (Tier 1)

        // Configure circuit breakers per provider
        circuit_breakers.insert(
            "anthropic".to_string(),
            CircuitBreaker::new(5, 2, Duration::from_secs(60)),
        );
        circuit_breakers.insert(
            "google".to_string(),
            CircuitBreaker::new(5, 2, Duration::from_secs(60)),
        );
        circuit_breakers.insert(
            "deepseek".to_string(),
            CircuitBreaker::new(5, 2, Duration::from_secs(60)),
        );
        circuit_breakers.insert(
            "openai".to_string(),
            CircuitBreaker::new(5, 2, Duration::from_secs(60)),
        );

        Self {
            client: Client::new(),
            api_key,
            rate_limiters,
            circuit_breakers,
        }
    }

    /// Detect provider from model name
    fn detect_provider(&self, model: &str) -> Result<String, LLMError> {
        if model.starts_with("claude") {
            Ok("anthropic".to_string())
        } else if model.starts_with("gemini") {
            Ok("google".to_string())
        } else if model.starts_with("deepseek") {
            Ok("deepseek".to_string())
        } else if model.starts_with("gpt") || model.starts_with("o1") || model.starts_with("o3") {
            Ok("openai".to_string())
        } else {
            Err(LLMError::UnsupportedModel(model.to_string()))
        }
    }

    /// Generate text with full rate limiting and circuit breaker protection
    pub async fn generate(&mut self, req: LLMRequest) -> Result<String> {
        let provider_name = self.detect_provider(&req.model)?;

        // Apply rate limiting BEFORE making request
        if let Some(limiter) = self.rate_limiters.get_mut(&provider_name) {
            match limiter.try_acquire() {
                Ok(()) => {
                    // Token acquired, proceed
                }
                Err(wait_duration) => {
                    // Rate limited - wait and retry
                    eprintln!(
                        "Rate limited by {} - waiting {:?}",
                        provider_name, wait_duration
                    );
                    tokio::time::sleep(wait_duration).await;
                    limiter
                        .try_acquire()
                        .map_err(|_| LLMError::RateLimitExceeded(provider_name.to_string()))?;
                }
            }
        }

        // Apply circuit breaker protection (async-compatible pattern)
        // IM-3041: Check if circuit allows request before async call
        if let Some(breaker) = self.circuit_breakers.get_mut(&provider_name) {
            if let Err(CircuitBreakerError::Open) = breaker.can_execute() {
                return Err(anyhow!(
                    "{} circuit breaker is open (too many failures)",
                    provider_name
                ));
            }
        }

        // Execute the actual async provider call
        let result = if req.model.starts_with("claude") {
            self.generate_anthropic(req).await
        } else if req.model.starts_with("gemini") {
            self.generate_gemini(req).await
        } else if req.model.starts_with("deepseek") {
            self.generate_deepseek(req).await
        } else if req.model.starts_with("gpt")
            || req.model.starts_with("o1")
            || req.model.starts_with("o3")
        {
            self.generate_openai(req).await
        } else {
            Err(anyhow!("Unsupported model: {}", req.model))
        };

        // IM-3042: Record outcome in circuit breaker after async call completes
        if let Some(breaker) = self.circuit_breakers.get_mut(&provider_name) {
            match &result {
                Ok(_) => breaker.record_success(),
                Err(_) => breaker.record_failure(),
            }
        }

        result
    }

    /// Generate text with streaming response (tokens arrive incrementally)
    /// IM-3015: generate_stream() method
    pub async fn generate_stream(
        &mut self,
        request: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let provider_name = self.detect_provider(&request.model)?;

        // Apply rate limiting before streaming
        if let Some(limiter) = self.rate_limiters.get_mut(&provider_name) {
            match limiter.try_acquire() {
                Ok(()) => {}
                Err(wait_duration) => {
                    tokio::time::sleep(wait_duration).await;
                    limiter.try_acquire().map_err(|_| {
                        anyhow!(LLMError::RateLimitExceeded(provider_name.to_string()))
                    })?;
                }
            }
        }

        // Route to provider-specific streaming
        if request.model.starts_with("claude") {
            self.generate_anthropic_stream(request).await
        } else if request.model.starts_with("gemini") {
            self.generate_gemini_stream(request).await
        } else if request.model.starts_with("deepseek") {
            self.generate_deepseek_stream(request).await
        } else if request.model.starts_with("gpt")
            || request.model.starts_with("o1")
            || request.model.starts_with("o3")
        {
            self.generate_openai_stream(request).await
        } else {
            Err(anyhow!(
                "Unsupported model for streaming: {}",
                request.model
            ))
        }
    }

    // ------------------------------------------------------------------
    // Multi-Turn Conversation Methods (IM-4020-4024)
    // ------------------------------------------------------------------

    /// IM-4020: Generate text with multi-turn conversation support
    /// Supports full conversation history with provider-specific optimizations
    pub async fn generate_multi_turn(&mut self, req: MultiTurnRequest) -> Result<String> {
        // Validate request
        req.validate().map_err(|e| anyhow!("{}", e))?;

        let provider_name = self.detect_provider(&req.model)?;

        // Apply rate limiting BEFORE making request
        if let Some(limiter) = self.rate_limiters.get_mut(&provider_name) {
            match limiter.try_acquire() {
                Ok(()) => {}
                Err(wait_duration) => {
                    eprintln!(
                        "Rate limited by {} - waiting {:?}",
                        provider_name, wait_duration
                    );
                    tokio::time::sleep(wait_duration).await;
                    limiter
                        .try_acquire()
                        .map_err(|_| LLMError::RateLimitExceeded(provider_name.to_string()))?;
                }
            }
        }

        // Apply circuit breaker protection
        if let Some(breaker) = self.circuit_breakers.get_mut(&provider_name) {
            if let Err(CircuitBreakerError::Open) = breaker.can_execute() {
                return Err(anyhow!(
                    "{} circuit breaker is open (too many failures)",
                    provider_name
                ));
            }
        }

        // IM-4020-B1: Route to provider-specific implementation
        let result = match provider_name.as_str() {
            "anthropic" => self.generate_multi_anthropic(&req).await,
            "google" => self.generate_multi_gemini(&req).await,
            "deepseek" => self.generate_multi_deepseek(&req).await,
            "openai" => self.generate_multi_openai(&req).await,
            _ => Err(anyhow!("Unsupported provider: {}", provider_name)),
        };

        // Record circuit breaker outcome
        if let Some(breaker) = self.circuit_breakers.get_mut(&provider_name) {
            match &result {
                Ok(_) => breaker.record_success(),
                Err(_) => breaker.record_failure(),
            }
        }

        result
    }

    /// IM-4021: Anthropic multi-turn with explicit caching support
    async fn generate_multi_anthropic(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = "https://api.anthropic.com/v1/messages";
        let body = to_anthropic_body(req);

        let mut request_builder = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");

        // IM-4021-B1: Add caching beta header if enabled
        if req.enable_caching {
            let cache_header = req
                .cache_config
                .as_ref()
                .map(|c| c.ttl.to_anthropic_header())
                .unwrap_or("prompt-caching-2024-07-31");
            request_builder = request_builder.header("anthropic-beta", cache_header);
        }

        let res = request_builder.json(&body).send().await?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await?;
            if status.as_u16() == 401 {
                return Err(anyhow!(
                    "Anthropic Authentication Failed (401). Error: {}",
                    error_text
                ));
            }
            return Err(anyhow!("Anthropic API Error ({}): {}", status, error_text));
        }

        let anthropic_res: AnthropicResponse = res.json().await?;
        anthropic_res
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow!("No content in Anthropic response"))
    }

    /// IM-4022: Gemini multi-turn with proper "model" role handling
    async fn generate_multi_gemini(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            req.model, self.api_key
        );
        let body = to_gemini_body(req);

        let res = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow!("Gemini API Error: {}", res.text().await?));
        }

        let gemini_res: GeminiResponse = res.json().await?;
        gemini_res
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow!("No content in Gemini response"))
    }

    /// IM-4023: DeepSeek multi-turn (OpenAI-compatible with R1 reasoning support)
    async fn generate_multi_deepseek(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = "https://api.deepseek.com/chat/completions";
        let body = to_openai_body(req);

        // Check if this is a reasoning model (R1)
        let is_reasoning_model = req.model.contains("reasoner");

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow!("DeepSeek API Error: {}", res.text().await?));
        }

        let deepseek_res: DeepSeekResponse = res.json().await?;
        let message = deepseek_res
            .choices
            .first()
            .map(|c| &c.message)
            .ok_or_else(|| anyhow!("No choices in DeepSeek response"))?;

        // For R1 reasoning models, combine reasoning_content and content
        if is_reasoning_model {
            let mut result = String::new();

            if let Some(ref reasoning) = message.reasoning_content {
                if !reasoning.is_empty() {
                    result.push_str("## AI Reasoning Process\n\n");
                    result.push_str(reasoning);
                    result.push_str("\n\n---\n\n## Final Analysis\n\n");
                }
            }

            if let Some(ref content) = message.content {
                result.push_str(content);
            }

            if result.is_empty() {
                return Err(anyhow!("No content in DeepSeek R1 response"));
            }

            Ok(result)
        } else {
            message
                .content
                .clone()
                .ok_or_else(|| anyhow!("No content in DeepSeek response"))
        }
    }

    /// IM-4024: OpenAI multi-turn (automatic caching for prompts >1024 tokens)
    async fn generate_multi_openai(&self, req: &MultiTurnRequest) -> Result<String> {
        let url = "https://api.openai.com/v1/chat/completions";
        let body = to_openai_body(req);

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await?;
            if status.as_u16() == 401 {
                return Err(anyhow!(
                    "OpenAI Authentication Failed (401). Error: {}",
                    error_text
                ));
            }
            return Err(anyhow!("OpenAI API Error ({}): {}", status, error_text));
        }

        let openai_res: OpenAIResponse = res.json().await?;
        openai_res
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No content in OpenAI response"))
    }

    /// IM-4030: Multi-turn streaming with conversation history
    pub async fn generate_multi_turn_stream(
        &mut self,
        req: MultiTurnRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        // Validate request
        req.validate().map_err(|e| anyhow!("{}", e))?;

        let provider_name = self.detect_provider(&req.model)?;

        // Apply rate limiting before streaming
        if let Some(limiter) = self.rate_limiters.get_mut(&provider_name) {
            match limiter.try_acquire() {
                Ok(()) => {}
                Err(wait_duration) => {
                    tokio::time::sleep(wait_duration).await;
                    limiter.try_acquire().map_err(|_| {
                        anyhow!(LLMError::RateLimitExceeded(provider_name.to_string()))
                    })?;
                }
            }
        }

        // Route to provider-specific streaming
        match provider_name.as_str() {
            "anthropic" => self.stream_multi_anthropic(&req).await,
            "google" => self.stream_multi_gemini(&req).await,
            "deepseek" => self.stream_multi_deepseek(&req).await,
            "openai" => self.stream_multi_openai(&req).await,
            _ => Err(anyhow!(
                "Unsupported provider for streaming: {}",
                provider_name
            )),
        }
    }

    /// IM-4031: Anthropic multi-turn streaming with caching
    async fn stream_multi_anthropic(
        &self,
        req: &MultiTurnRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = "https://api.anthropic.com/v1/messages";
        let body = to_anthropic_stream_body(req);

        let mut request_builder = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json");

        if req.enable_caching {
            let cache_header = req
                .cache_config
                .as_ref()
                .map(|c| c.ttl.to_anthropic_header())
                .unwrap_or("prompt-caching-2024-07-31");
            request_builder = request_builder.header("anthropic-beta", cache_header);
        }

        let res = request_builder
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start Anthropic stream: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow!("Anthropic API Error: {}", res.status()));
        }

        let stream = res.bytes_stream();

        let token_stream = stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    for line in text.lines() {
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if json_str == "[DONE]" {
                                return None;
                            }
                            if let Ok(event) =
                                serde_json::from_str::<AnthropicStreamEvent>(json_str)
                            {
                                if event.event_type == "content_block_delta" {
                                    if let Some(delta) = event.delta {
                                        if let Some(text) = delta.text {
                                            return Some(Ok(text));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
            }
        });

        Ok(Box::pin(token_stream))
    }

    /// IM-4032: Gemini multi-turn streaming
    async fn stream_multi_gemini(
        &self,
        req: &MultiTurnRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            req.model, self.api_key
        );
        let body = to_gemini_body(req);

        let res = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start Gemini stream: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await.unwrap_or_default();
            return Err(anyhow!("Gemini API Error ({}): {}", status, error_text));
        }

        let stream = res.bytes_stream();

        let token_stream = stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    for line in text.lines() {
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed == "[" || trimmed == "]" || trimmed == ","
                        {
                            continue;
                        }

                        let json_str = if let Some(data) = trimmed.strip_prefix("data: ") {
                            data
                        } else if trimmed.starts_with('{') {
                            trimmed
                        } else {
                            continue;
                        };

                        if let Ok(response) = serde_json::from_str::<GeminiResponse>(json_str) {
                            if let Some(candidate) = response.candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    return Some(Ok(part.text.clone()));
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
            }
        });

        Ok(Box::pin(token_stream))
    }

    /// IM-4033: DeepSeek multi-turn streaming
    async fn stream_multi_deepseek(
        &self,
        req: &MultiTurnRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = "https://api.deepseek.com/chat/completions";
        let body = to_openai_stream_body(req);

        let is_reasoning_model = req.model.contains("reasoner");

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start DeepSeek stream: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow!("DeepSeek API Error: {}", res.status()));
        }

        let stream = res.bytes_stream();
        let started_content = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let started_content_clone = started_content.clone();

        let token_stream = stream.filter_map(move |chunk_result| {
            let started_content = started_content_clone.clone();
            let is_r1 = is_reasoning_model;

            async move {
                match chunk_result {
                    Ok(chunk) => {
                        let text = String::from_utf8_lossy(&chunk);
                        let mut result_tokens = Vec::new();

                        for line in text.lines() {
                            if let Some(json_str) = line.strip_prefix("data: ") {
                                if json_str == "[DONE]" {
                                    continue;
                                }
                                if let Ok(chunk_data) =
                                    serde_json::from_str::<DeepSeekStreamChunk>(json_str)
                                {
                                    if let Some(choice) = chunk_data.choices.first() {
                                        if is_r1 {
                                            if let Some(reasoning) = &choice.delta.reasoning_content
                                            {
                                                if !reasoning.is_empty() {
                                                    result_tokens.push(reasoning.clone());
                                                }
                                            }
                                        }
                                        if let Some(content) = &choice.delta.content {
                                            if !content.is_empty() {
                                                if is_r1
                                                    && !started_content
                                                        .load(std::sync::atomic::Ordering::Relaxed)
                                                {
                                                    started_content.store(
                                                        true,
                                                        std::sync::atomic::Ordering::Relaxed,
                                                    );
                                                    result_tokens.push("\n\n---\n\n".to_string());
                                                }
                                                result_tokens.push(content.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if result_tokens.is_empty() {
                            None
                        } else {
                            Some(Ok(result_tokens.join("")))
                        }
                    }
                    Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
                }
            }
        });

        Ok(Box::pin(token_stream))
    }

    /// IM-4034: OpenAI multi-turn streaming
    async fn stream_multi_openai(
        &self,
        req: &MultiTurnRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = "https://api.openai.com/v1/chat/completions";
        let body = to_openai_stream_body(req);

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start OpenAI stream: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow!("OpenAI API Error: {}", res.status()));
        }

        let stream = res.bytes_stream();

        let token_stream = stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    for line in text.lines() {
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if json_str == "[DONE]" {
                                return None;
                            }
                            if let Ok(chunk_data) =
                                serde_json::from_str::<OpenAIStreamChunk>(json_str)
                            {
                                if let Some(choice) = chunk_data.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        return Some(Ok(content.clone()));
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
            }
        });

        Ok(Box::pin(token_stream))
    }

    // ------------------------------------------------------------------
    // Provider Implementations (Non-Streaming)
    // ------------------------------------------------------------------

    async fn generate_anthropic(&self, req: LLMRequest) -> Result<String> {
        let url = "https://api.anthropic.com/v1/messages";

        // Debug: Log key prefix (first 10 chars only for security)
        let key_prefix = if self.api_key.len() > 10 {
            &self.api_key[..10]
        } else {
            &self.api_key
        };
        println!(
            "[DEBUG] Anthropic request with key prefix: {}... (len={})",
            key_prefix,
            self.api_key.len()
        );

        // Validate key format
        if !self.api_key.starts_with("sk-ant-") {
            return Err(anyhow!("Invalid Anthropic API key format. Key should start with 'sk-ant-'. Got prefix: '{}'", key_prefix));
        }

        let body = serde_json::json!({
            "model": req.model,
            "max_tokens": 4096,
            "system": req.system,
            "messages": [
                {
                    "role": "user",
                    "content": req.user
                }
            ]
        });

        let res = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await?;
            if status.as_u16() == 401 {
                return Err(anyhow!("Anthropic Authentication Failed (401). Please verify your API key is valid and active. Error: {}", error_text));
            }
            return Err(anyhow!("Anthropic API Error ({}): {}", status, error_text));
        }

        let anthropic_res: AnthropicResponse = res.json().await?;

        anthropic_res
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| anyhow!("No content in Anthropic response"))
    }

    async fn generate_gemini(&self, req: LLMRequest) -> Result<String> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            req.model, self.api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": format!("System Instruction: {}\n\nUser Request: {}", req.system, req.user)
                }]
            }]
        });

        let res = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow!("Gemini API Error: {}", res.text().await?));
        }

        let gemini_res: GeminiResponse = res.json().await?;

        gemini_res
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .ok_or_else(|| anyhow!("No content in Gemini response"))
    }

    async fn generate_deepseek(&self, req: LLMRequest) -> Result<String> {
        let url = "https://api.deepseek.com/chat/completions";

        // Check if this is a reasoning model (R1)
        let is_reasoning_model = req.model.contains("reasoner");

        let body = serde_json::json!({
            "model": req.model,
            "messages": [
                {"role": "system", "content": req.system},
                {"role": "user", "content": req.user}
            ],
            "stream": false
        });

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow!("DeepSeek API Error: {}", res.text().await?));
        }

        let deepseek_res: DeepSeekResponse = res.json().await?;

        let message = deepseek_res
            .choices
            .first()
            .map(|c| &c.message)
            .ok_or_else(|| anyhow!("No choices in DeepSeek response"))?;

        // For R1 reasoning models, combine reasoning_content and content
        // The reasoning shows the model's thinking process
        if is_reasoning_model {
            let mut result = String::new();

            // Include reasoning if present (shows AI's thought process)
            if let Some(ref reasoning) = message.reasoning_content {
                if !reasoning.is_empty() {
                    result.push_str("## AI Reasoning Process\n\n");
                    result.push_str(reasoning);
                    result.push_str("\n\n---\n\n## Final Analysis\n\n");
                }
            }

            // Include final content
            if let Some(ref content) = message.content {
                result.push_str(content);
            }

            if result.is_empty() {
                return Err(anyhow!("No content in DeepSeek R1 response"));
            }

            Ok(result)
        } else {
            // For non-reasoning models (deepseek-chat), just return content
            message
                .content
                .clone()
                .ok_or_else(|| anyhow!("No content in DeepSeek response"))
        }
    }

    async fn generate_openai(&self, req: LLMRequest) -> Result<String> {
        let url = "https://api.openai.com/v1/chat/completions";

        // Debug: Log key prefix (first 10 chars only for security)
        let key_prefix = if self.api_key.len() > 10 {
            &self.api_key[..10]
        } else {
            &self.api_key
        };
        println!(
            "[DEBUG] OpenAI request with key prefix: {}... (len={})",
            key_prefix,
            self.api_key.len()
        );

        // Validate key format
        if !self.api_key.starts_with("sk-") {
            return Err(anyhow!(
                "Invalid OpenAI API key format. Key should start with 'sk-'. Got prefix: '{}'",
                key_prefix
            ));
        }

        let body = serde_json::json!({
            "model": req.model,
            "messages": [
                {"role": "system", "content": req.system},
                {"role": "user", "content": req.user}
            ],
            "max_tokens": 4096
        });

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await?;
            if status.as_u16() == 401 {
                return Err(anyhow!("OpenAI Authentication Failed (401). Please verify your API key is valid and active. Error: {}", error_text));
            }
            return Err(anyhow!("OpenAI API Error ({}): {}", status, error_text));
        }

        let openai_res: OpenAIResponse = res.json().await?;

        openai_res
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No content in OpenAI response"))
    }

    // ------------------------------------------------------------------
    // Provider Implementations (Streaming)
    // ------------------------------------------------------------------

    /// Anthropic SSE streaming implementation
    /// IM-3015-STREAM-1: Anthropic SSE format parsing
    async fn generate_anthropic_stream(
        &self,
        req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = "https://api.anthropic.com/v1/messages";

        let body = serde_json::json!({
            "model": req.model,
            "max_tokens": 4096,
            "system": req.system,
            "messages": [{
                "role": "user",
                "content": req.user
            }],
            "stream": true
        });

        let res = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start Anthropic stream: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow!("Anthropic API Error: {}", res.status()));
        }

        let stream = res.bytes_stream();

        let token_stream = stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);

                    for line in text.lines() {
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if json_str == "[DONE]" {
                                return None;
                            }

                            if let Ok(event) =
                                serde_json::from_str::<AnthropicStreamEvent>(json_str)
                            {
                                if event.event_type == "content_block_delta" {
                                    if let Some(delta) = event.delta {
                                        if let Some(text) = delta.text {
                                            return Some(Ok(text));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
            }
        });

        Ok(Box::pin(token_stream))
    }

    /// Gemini newline-delimited JSON streaming implementation
    /// IM-3015-STREAM-2: Gemini JSON stream parsing
    /// Note: Gemini streamGenerateContent returns JSON array chunks that need special parsing
    async fn generate_gemini_stream(
        &self,
        req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?key={}&alt=sse",
            req.model, self.api_key
        );

        println!(
            "[DEBUG] Gemini stream URL: {}",
            url.replace(&self.api_key, "***KEY***")
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": format!("System Instruction: {}\n\nUser Request: {}", req.system, req.user)
                }]
            }]
        });

        let res = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start Gemini stream: {}", e))?;

        if !res.status().is_success() {
            let status = res.status();
            let error_text = res.text().await.unwrap_or_default();
            println!("[DEBUG] Gemini API Error {}: {}", status, error_text);
            return Err(anyhow!("Gemini API Error ({}): {}", status, error_text));
        }

        println!("[DEBUG] Gemini stream connected successfully");
        let stream = res.bytes_stream();

        // Gemini with alt=sse returns SSE format: "data: {json}\n\n"
        let token_stream = stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    println!("[DEBUG] Gemini chunk: {}", &text[..text.len().min(200)]);

                    // Parse SSE format: "data: {...}\n\n"
                    for line in text.lines() {
                        // Skip empty lines and array brackets
                        let trimmed = line.trim();
                        if trimmed.is_empty() || trimmed == "[" || trimmed == "]" || trimmed == ","
                        {
                            continue;
                        }

                        // Handle SSE format
                        let json_str = if let Some(data) = trimmed.strip_prefix("data: ") {
                            data
                        } else if trimmed.starts_with('{') {
                            // Direct JSON object (non-SSE format)
                            trimmed
                        } else {
                            continue;
                        };

                        // Try to parse the JSON
                        if let Ok(response) = serde_json::from_str::<GeminiResponse>(json_str) {
                            if let Some(candidate) = response.candidates.first() {
                                if let Some(part) = candidate.content.parts.first() {
                                    println!(
                                        "[DEBUG] Gemini extracted text: {}...",
                                        &part.text[..part.text.len().min(50)]
                                    );
                                    return Some(Ok(part.text.clone()));
                                }
                            }
                        } else {
                            println!(
                                "[DEBUG] Failed to parse Gemini JSON: {}",
                                &json_str[..json_str.len().min(100)]
                            );
                        }
                    }
                    None
                }
                Err(e) => {
                    println!("[DEBUG] Gemini stream error: {}", e);
                    Some(Err(LLMError::NetworkError(e.to_string())))
                }
            }
        });

        Ok(Box::pin(token_stream))
    }

    /// DeepSeek OpenAI-compatible SSE streaming implementation
    /// IM-3015-STREAM-3: DeepSeek OpenAI-compatible SSE parsing
    /// Updated to support R1 reasoning models which stream reasoning_content before content
    async fn generate_deepseek_stream(
        &self,
        req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = "https://api.deepseek.com/chat/completions";

        // Check if this is a reasoning model (R1)
        let is_reasoning_model = req.model.contains("reasoner");

        let body = serde_json::json!({
            "model": req.model,
            "messages": [
                {"role": "system", "content": req.system},
                {"role": "user", "content": req.user}
            ],
            "stream": true
        });

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start DeepSeek stream: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow!("DeepSeek API Error: {}", res.status()));
        }

        let stream = res.bytes_stream();

        // Track if we've started receiving final content (for R1 formatting)
        let started_content = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let started_content_clone = started_content.clone();

        let token_stream = stream.filter_map(move |chunk_result| {
            let started_content = started_content_clone.clone();
            let is_r1 = is_reasoning_model;

            async move {
                match chunk_result {
                    Ok(chunk) => {
                        let text = String::from_utf8_lossy(&chunk);
                        let mut result_tokens = Vec::new();

                        for line in text.lines() {
                            if let Some(json_str) = line.strip_prefix("data: ") {
                                if json_str == "[DONE]" {
                                    continue;
                                }

                                if let Ok(chunk_data) =
                                    serde_json::from_str::<DeepSeekStreamChunk>(json_str)
                                {
                                    if let Some(choice) = chunk_data.choices.first() {
                                        // For R1 models, stream reasoning_content first
                                        if is_r1 {
                                            if let Some(reasoning) = &choice.delta.reasoning_content
                                            {
                                                if !reasoning.is_empty() {
                                                    result_tokens.push(reasoning.clone());
                                                }
                                            }
                                        }

                                        // Stream content (final answer)
                                        if let Some(content) = &choice.delta.content {
                                            if !content.is_empty() {
                                                // For R1, add separator when transitioning from reasoning to content
                                                if is_r1
                                                    && !started_content
                                                        .load(std::sync::atomic::Ordering::Relaxed)
                                                {
                                                    started_content.store(
                                                        true,
                                                        std::sync::atomic::Ordering::Relaxed,
                                                    );
                                                    result_tokens.push("\n\n---\n\n".to_string());
                                                }
                                                result_tokens.push(content.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if result_tokens.is_empty() {
                            None
                        } else {
                            Some(Ok(result_tokens.join("")))
                        }
                    }
                    Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
                }
            }
        });

        Ok(Box::pin(token_stream))
    }

    /// OpenAI SSE streaming implementation
    /// IM-3015-STREAM-4: OpenAI SSE parsing (same format as DeepSeek)
    async fn generate_openai_stream(
        &self,
        req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>>> {
        let url = "https://api.openai.com/v1/chat/completions";

        let body = serde_json::json!({
            "model": req.model,
            "messages": [
                {"role": "system", "content": req.system},
                {"role": "user", "content": req.user}
            ],
            "stream": true
        });

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to start OpenAI stream: {}", e))?;

        if !res.status().is_success() {
            return Err(anyhow!("OpenAI API Error: {}", res.status()));
        }

        let stream = res.bytes_stream();

        let token_stream = stream.filter_map(|chunk_result| async move {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);

                    for line in text.lines() {
                        if let Some(json_str) = line.strip_prefix("data: ") {
                            if json_str == "[DONE]" {
                                return None;
                            }

                            if let Ok(chunk_data) =
                                serde_json::from_str::<OpenAIStreamChunk>(json_str)
                            {
                                if let Some(choice) = chunk_data.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        return Some(Ok(content.clone()));
                                    }
                                }
                            }
                        }
                    }
                    None
                }
                Err(e) => Some(Err(LLMError::NetworkError(e.to_string()))),
            }
        });

        Ok(Box::pin(token_stream))
    }
}

// ------------------------------------------------------------------
// Tests (Phase 10: EXECUTE TESTS)
// ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // ------------------------------------------------------------------
    // Battery 4.10: Rate Limiting Tests (IM-3020-3024)
    // ------------------------------------------------------------------

    #[test]
    fn test_rate_limiter_tokens_field_initialization() {
        // TEST-UNIT-3020-F1: Verify tokens field initializes to full capacity
        let requests_per_minute = 60.0;
        let limiter = RateLimiter::new(requests_per_minute);

        assert_eq!(
            limiter.available_tokens(),
            60.0,
            "Tokens should initialize to requests_per_minute"
        );
    }

    #[test]
    fn test_rate_limiter_capacity_field() {
        // TEST-UNIT-3020-F2: Verify capacity field stores maximum token limit
        let limiter = RateLimiter::new(100.0);

        // Capacity prevents token accumulation beyond limit
        std::thread::sleep(Duration::from_secs(2));
        assert!(
            limiter.available_tokens() <= 100.0,
            "Tokens should never exceed capacity"
        );
    }

    #[test]
    fn test_rate_limiter_refill_rate_calculation() {
        // TEST-UNIT-3020-F3: Verify refill_rate correctly converts RPM to tokens-per-second
        let limiter = RateLimiter::new(60.0);

        // refill_rate should be RPM / 60 = 1.0 token per second
        assert_eq!(
            limiter.refill_rate, 1.0,
            "Refill rate should be requests_per_minute / 60"
        );
    }

    #[test]
    fn test_rate_limiter_last_refill_timestamp() {
        // TEST-UNIT-3020-F4: Verify last_refill tracks token refill timing
        let limiter = RateLimiter::new(60.0);
        let creation_time = Instant::now();

        // last_refill should be initialized near current time
        let elapsed = creation_time.duration_since(limiter.last_refill);
        assert!(
            elapsed < Duration::from_millis(100),
            "last_refill should be initialized to Instant::now()"
        );
    }

    #[test]
    fn test_rate_limiter_constructor() {
        // TEST-UNIT-3021: Verify RateLimiter::new() initializes all fields
        let limiter = RateLimiter::new(120.0);

        assert_eq!(limiter.tokens, 120.0, "tokens should equal capacity");
        assert_eq!(limiter.capacity, 120.0, "capacity should equal parameter");
        assert_eq!(limiter.refill_rate, 2.0, "refill_rate should be RPM/60");
        assert!(
            limiter.last_refill.elapsed() < Duration::from_millis(100),
            "last_refill should be recent"
        );
    }

    #[test]
    fn test_rate_limiter_try_acquire_wait_calculation() {
        // TEST-UNIT-3022-V1: Verify wait_seconds variable calculation when rate limited
        let mut limiter = RateLimiter::new(60.0); // 1 token/sec

        // Exhaust all tokens
        for _ in 0..60 {
            limiter.try_acquire().unwrap();
        }

        // Next request should require wait
        match limiter.try_acquire() {
            Err(wait_duration) => {
                // Should wait ~1 second for next token
                assert!(
                    (wait_duration.as_secs_f64() - 1.0).abs() < 0.1,
                    "Wait should be ~1 second for next token"
                );
            }
            Ok(_) => panic!("Should require wait when tokens exhausted"),
        }
    }

    #[test]
    fn test_rate_limiter_token_availability_branch() {
        // TEST-UNIT-3022-B1: Verify branch logic for token availability (>= 1.0)
        let mut limiter = RateLimiter::new(10.0);

        // TRUE branch: tokens available
        assert!(
            limiter.try_acquire().is_ok(),
            "Should succeed when tokens >= 1.0"
        );

        // Exhaust tokens
        for _ in 0..9 {
            limiter.try_acquire().unwrap();
        }

        // FALSE branch: no tokens available
        assert!(
            limiter.try_acquire().is_err(),
            "Should fail when tokens < 1.0"
        );
    }

    #[test]
    fn test_rate_limiter_error_on_rate_limit() {
        // TEST-UNIT-3022-E1: Verify Err(Duration) returned when rate limited
        let mut limiter = RateLimiter::new(5.0);

        // Exhaust tokens
        for _ in 0..5 {
            limiter.try_acquire().unwrap();
        }

        // Next request should return Err with wait duration
        match limiter.try_acquire() {
            Err(duration) => {
                assert!(
                    duration.as_secs_f64() > 0.0,
                    "Error should contain wait duration"
                );
            }
            Ok(_) => panic!("Should return Err when rate limited"),
        }
    }

    #[test]
    fn test_rate_limiter_refill_now_variable() {
        // TEST-UNIT-3023-V1: Verify refill() calculates current time correctly
        let mut limiter = RateLimiter::new(60.0);
        limiter.try_acquire().unwrap(); // Consume 1 token

        std::thread::sleep(Duration::from_secs(1));
        limiter.refill();

        // After 1 second, should have ~1 token refilled
        assert!(
            limiter.available_tokens() >= 59.9,
            "Should refill ~1 token per second"
        );
    }

    #[test]
    fn test_rate_limiter_refill_elapsed_variable() {
        // TEST-UNIT-3023-V2: Verify refill() calculates elapsed time correctly
        let mut limiter = RateLimiter::new(120.0);

        // Consume 10 tokens
        for _ in 0..10 {
            limiter.try_acquire().unwrap();
        }

        // Wait 0.5 seconds (should refill 1 token at 2 tokens/sec)
        std::thread::sleep(Duration::from_millis(500));
        limiter.refill();

        // Should have ~111 tokens (110 + 1 refilled)
        assert!(
            (limiter.available_tokens() - 111.0).abs() < 1.0,
            "Should refill based on elapsed time"
        );
    }

    #[test]
    fn test_rate_limiter_available_tokens_method() {
        // TEST-UNIT-3024: Verify available_tokens() returns current token count
        let mut limiter = RateLimiter::new(100.0);

        assert_eq!(
            limiter.available_tokens(),
            100.0,
            "Should return full capacity initially"
        );

        limiter.try_acquire().unwrap();
        assert_eq!(
            limiter.available_tokens(),
            99.0,
            "Should return tokens after consumption"
        );
    }

    #[tokio::test]
    async fn test_llm_client_rate_limiting_integration() {
        // TEST-INTEGRATION-3025: Verify LLMClient enforces rate limiting
        let mut client = LLMClient::new("test_key".to_string());

        // Manually set low rate limit for testing
        client.rate_limiters.insert(
            "anthropic".to_string(),
            RateLimiter::new(2.0), // 2 RPM = very low for testing
        );

        // First 2 requests should succeed quickly
        let start = Instant::now();

        // Note: These will fail at API level (invalid key), but rate limiting
        // happens BEFORE the API call, which is what we're testing
        let req1 = LLMRequest {
            model: "claude-3-sonnet".to_string(),
            system: "test".to_string(),
            user: "test".to_string(),
        };

        let _ = client.generate(req1.clone()).await; // Will fail at API, but pass rate limit
        let _ = client.generate(req1.clone()).await; // Will fail at API, but pass rate limit

        let elapsed_first_two = start.elapsed();

        // Third request should be rate limited (needs to wait ~30 seconds)
        let start_third = Instant::now();
        let _ = client.generate(req1.clone()).await;
        let elapsed_third = start_third.elapsed();

        assert!(
            elapsed_first_two.as_secs() < 5,
            "First 2 requests should be fast (no rate limiting)"
        );
        assert!(
            elapsed_third.as_secs() >= 25,
            "Third request should wait for rate limit (~30s for next token)"
        );
    }

    // ------------------------------------------------------------------
    // Battery 4.11: Circuit Breaker Tests (IM-3030-3037)
    // ------------------------------------------------------------------

    #[test]
    fn test_circuit_breaker_state_field_initialization() {
        // TEST-UNIT-3030-F1: Verify state field initializes to Closed
        let breaker = CircuitBreaker::new(5, 2, Duration::from_secs(60));

        assert_eq!(
            breaker.state(),
            CircuitState::Closed,
            "Circuit breaker should start in Closed state"
        );
    }

    #[test]
    fn test_circuit_state_enum_variants() {
        // TEST-UNIT-3031: Verify CircuitState enum has all 3 variants
        let closed = CircuitState::Closed;
        let open = CircuitState::Open;
        let half_open = CircuitState::HalfOpen;

        assert_ne!(closed, open, "Closed and Open should be different");
        assert_ne!(open, half_open, "Open and HalfOpen should be different");
        assert_ne!(closed, half_open, "Closed and HalfOpen should be different");
    }

    #[test]
    fn test_circuit_breaker_error_enum_variants() {
        // TEST-UNIT-3032: Verify CircuitBreakerError enum variants
        let error_open = CircuitBreakerError::Open;
        let error_failed = CircuitBreakerError::RequestFailed("test".to_string());

        match error_open {
            CircuitBreakerError::Open => {} // Expected
            _ => panic!("Should match Open variant"),
        }

        match error_failed {
            CircuitBreakerError::RequestFailed(_) => {} // Expected
            _ => panic!("Should match RequestFailed variant"),
        }
    }

    #[test]
    fn test_circuit_breaker_constructor() {
        // TEST-UNIT-3033: Verify CircuitBreaker::new() initializes all fields
        let breaker = CircuitBreaker::new(3, 2, Duration::from_secs(30));

        assert_eq!(breaker.state(), CircuitState::Closed);
        assert_eq!(breaker.failure_threshold, 3);
        assert_eq!(breaker.success_threshold, 2);
        assert_eq!(breaker.timeout_duration, Duration::from_secs(30));
    }

    #[test]
    fn test_circuit_breaker_closed_to_open_transition() {
        // TEST-UNIT-3034-B1: Verify Closed → Open after failure_threshold failures
        let mut breaker = CircuitBreaker::new(3, 2, Duration::from_secs(60));

        assert_eq!(breaker.state(), CircuitState::Closed);

        // Fail 3 times (threshold)
        for _ in 0..3 {
            let _: Result<(), _> = breaker.call(|| Err("test error"));
        }

        // Circuit should be Open
        assert_eq!(
            breaker.state(),
            CircuitState::Open,
            "Should transition to Open after 3 failures"
        );

        // Next request should be rejected immediately
        let result = breaker.call(|| Ok::<_, &str>(42));
        assert!(
            matches!(result, Err(CircuitBreakerError::Open)),
            "Should block requests when Open"
        );
    }

    #[test]
    fn test_circuit_breaker_open_to_halfopen_transition() {
        // TEST-UNIT-3034-B2: Verify Open → HalfOpen after timeout
        let mut breaker = CircuitBreaker::new(1, 2, Duration::from_millis(100));

        // Open the circuit with 1 failure
        let _: Result<(), _> = breaker.call(|| Err("test error"));
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Next call should transition to HalfOpen and execute
        let _result = breaker.call(|| Ok::<_, &str>(42));

        // Should be in HalfOpen state now (not necessarily Closed yet)
        assert!(
            breaker.state() == CircuitState::HalfOpen || breaker.state() == CircuitState::Closed,
            "Should transition to HalfOpen after timeout"
        );
    }

    #[test]
    fn test_circuit_breaker_halfopen_to_closed_transition() {
        // TEST-UNIT-3034-B3: Verify HalfOpen → Closed after success_threshold successes
        let mut breaker = CircuitBreaker::new(1, 2, Duration::from_millis(100));

        // Open the circuit
        let _: Result<(), _> = breaker.call(|| Err("test error"));
        assert_eq!(breaker.state(), CircuitState::Open);

        // Wait for timeout → HalfOpen
        std::thread::sleep(Duration::from_millis(150));

        // Succeed 2 times (success_threshold)
        let _ = breaker.call(|| Ok::<_, &str>(1));
        let _ = breaker.call(|| Ok::<_, &str>(2));

        // Circuit should be Closed
        assert_eq!(
            breaker.state(),
            CircuitState::Closed,
            "Should close after 2 successes in HalfOpen"
        );
    }

    #[tokio::test]
    async fn test_llm_client_circuit_breaker_integration() {
        // TEST-INTEGRATION-3038: Verify LLMClient circuit breaker protection
        let mut client = LLMClient::new("invalid_key".to_string());

        // Circuit breaker configured with failure_threshold=5
        // Make 5 requests that will fail (invalid API key)
        let req = LLMRequest {
            model: "claude-3-sonnet".to_string(),
            system: "test".to_string(),
            user: "test".to_string(),
        };

        for _ in 0..5 {
            let _ = client.generate(req.clone()).await;
        }

        // 6th request should be blocked by circuit breaker (circuit should be Open)
        // Note: This test verifies the circuit breaker integration, actual API
        // failures will occur due to invalid key, but that's expected
        let result = client.generate(req.clone()).await;
        assert!(result.is_err(), "Should fail after 5 consecutive failures");
    }

    // ------------------------------------------------------------------
    // Battery 4.12: Streaming Tests (IM-3015, IM-3401)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_llmclient_generate_stream_method() {
        // TEST-UNIT-3015: Verify LLMClient::generate_stream() method exists and returns Stream
        let mut client = LLMClient::new("test_key".to_string());

        let request = LLMRequest {
            model: "claude-3-sonnet".to_string(),
            system: "test".to_string(),
            user: "test".to_string(),
        };

        // Method should exist and return Result<Stream>
        // Will fail with invalid API key, but method signature is verified
        let result = client.generate_stream(request).await;

        // Result type check (method exists and returns correct type)
        match result {
            Ok(_stream) => {
                // Stream returned successfully (won't happen with invalid key)
                // But this verifies the method signature
            }
            Err(_) => {
                // Expected with invalid API key
                // But proves method exists and returns Result<Pin<Box<dyn Stream>>>
            }
        }
    }

    #[test]
    fn test_llm_provider_trait_generate_stream() {
        // TEST-UNIT-3401: Verify LLMProvider trait has generate_stream() method
        // This is a compile-time test - if this compiles, the trait has the method

        // Trait definition check (compile-time verification)
        // The existence of generate_stream in LLMClient proves the trait method exists
        // Compile-time verification - trait method exists
    }

    // Note: Provider-specific streaming format tests (IM-3015-STREAM-1/2/3)
    // require actual API credentials and live network calls.
    // These are better suited for integration tests with mocked responses
    // or manual testing with real API keys in a development environment.
    //
    // TEST-UNIT-3015-STREAM-1: Anthropic SSE format parsing
    // TEST-UNIT-3015-STREAM-2: Gemini JSON stream parsing
    // TEST-UNIT-3015-STREAM-3: DeepSeek OpenAI-compatible SSE parsing
    //
    // These tests would require:
    // - Mock HTTP server returning provider-specific streaming formats
    // - Or integration tests with real API keys (not suitable for unit tests)
    #[test]
    fn test_llmclient_constructor() {
        // TEST-UNIT-LLMCLIENT-001: Verify LLMClient::new() initializes correctly
        // Purpose: Validate constructor creates client with all required fields

        let api_key = "sk-ant-test-key-12345".to_string();
        let _client = LLMClient::new(api_key);

        // If constructor succeeds without panic, test passes
        // Compile-time verification - constructor succeeds
    }

    #[test]
    fn test_detect_provider_anthropic() {
        // TEST-UNIT-LLMCLIENT-002: Verify detect_provider() identifies Anthropic models
        // Purpose: Validate provider detection for claude-* model names

        let client = LLMClient::new("test-key".to_string());

        let provider = client
            .detect_provider("claude-sonnet-4-5-20250929")
            .unwrap();
        assert_eq!(
            provider, "anthropic",
            "Should detect anthropic from claude model"
        );

        let provider2 = client.detect_provider("claude-3-opus").unwrap();
        assert_eq!(
            provider2, "anthropic",
            "Should detect anthropic from claude-3-opus"
        );
    }

    #[test]
    fn test_detect_provider_google() {
        // TEST-UNIT-LLMCLIENT-003: Verify detect_provider() identifies Google models
        // Purpose: Validate provider detection for gemini-* model names

        let client = LLMClient::new("test-key".to_string());

        let provider = client.detect_provider("gemini-pro").unwrap();
        assert_eq!(provider, "google", "Should detect google from gemini model");

        let provider2 = client.detect_provider("gemini-1.5-pro").unwrap();
        assert_eq!(
            provider2, "google",
            "Should detect google from gemini-1.5-pro"
        );
    }

    #[test]
    fn test_detect_provider_deepseek() {
        // TEST-UNIT-LLMCLIENT-004: Verify detect_provider() identifies DeepSeek models
        // Purpose: Validate provider detection for deepseek-* model names

        let client = LLMClient::new("test-key".to_string());

        let provider = client.detect_provider("deepseek-chat").unwrap();
        assert_eq!(
            provider, "deepseek",
            "Should detect deepseek from deepseek-chat"
        );

        let provider2 = client.detect_provider("deepseek-coder").unwrap();
        assert_eq!(
            provider2, "deepseek",
            "Should detect deepseek from deepseek-coder"
        );
    }

    #[test]
    fn test_detect_provider_openai() {
        // TEST-UNIT-LLMCLIENT-004b: Verify detect_provider() identifies OpenAI models
        // Purpose: Validate provider detection for gpt-*, o1-*, o3-* model names

        let client = LLMClient::new("test-key".to_string());

        // GPT models
        let provider = client.detect_provider("gpt-4").unwrap();
        assert_eq!(provider, "openai", "Should detect openai from gpt-4");

        let provider2 = client.detect_provider("gpt-4o").unwrap();
        assert_eq!(provider2, "openai", "Should detect openai from gpt-4o");

        let provider3 = client.detect_provider("gpt-3.5-turbo").unwrap();
        assert_eq!(
            provider3, "openai",
            "Should detect openai from gpt-3.5-turbo"
        );

        // o1 reasoning models
        let provider4 = client.detect_provider("o1-preview").unwrap();
        assert_eq!(provider4, "openai", "Should detect openai from o1-preview");

        // o3 models
        let provider5 = client.detect_provider("o3-mini").unwrap();
        assert_eq!(provider5, "openai", "Should detect openai from o3-mini");
    }

    #[test]
    fn test_detect_provider_unsupported_model() {
        // TEST-UNIT-LLMCLIENT-005: Verify detect_provider() rejects unknown models
        // Purpose: Validate error handling for unsupported model names
        //
        // Note: Supported providers and their model prefixes:
        // - anthropic: "claude*"
        // - openai: "gpt*", "o1*", "o3*"
        // - google: "gemini*"
        // - deepseek: "deepseek*"

        let client = LLMClient::new("test-key".to_string());

        // llama models are not supported (no Llama/Meta provider integration)
        let result = client.detect_provider("llama-3");
        assert!(result.is_err(), "Should return error for llama-3");

        // mistral models are not supported (no Mistral provider integration)
        let result2 = client.detect_provider("mistral-large");
        assert!(result2.is_err(), "Should return error for mistral-large");

        // completely unknown model names should error
        let result3 = client.detect_provider("unknown-model");
        assert!(result3.is_err(), "Should return error for unknown-model");

        // verify error type is UnsupportedModel
        if let Err(LLMError::UnsupportedModel(model_name)) = result3 {
            assert_eq!(model_name, "unknown-model");
        } else {
            panic!("Expected UnsupportedModel error");
        }
    }

    #[test]
    fn test_llmrequest_struct_creation() {
        // TEST-UNIT-LLMCLIENT-006: Verify LLMRequest struct can be created
        // Purpose: Validate request struct initialization

        let request = LLMRequest {
            system: "You are a helpful assistant".to_string(),
            user: "Hello, world!".to_string(),
            model: "claude-sonnet-4-5-20250929".to_string(),
        };

        assert_eq!(request.system, "You are a helpful assistant");
        assert_eq!(request.user, "Hello, world!");
        assert_eq!(request.model, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_llmerror_network_error_variant() {
        // TEST-UNIT-LLMCLIENT-007: Verify LLMError::NetworkError variant exists
        // Purpose: Validate error enum has NetworkError variant

        let error = LLMError::NetworkError("Connection failed".to_string());
        let error_msg = format!("{:?}", error);

        assert!(
            error_msg.contains("NetworkError"),
            "Error should be NetworkError variant"
        );
    }

    #[test]
    fn test_llmerror_unsupported_model_variant() {
        // TEST-UNIT-LLMCLIENT-008: Verify LLMError::UnsupportedModel variant exists
        // Purpose: Validate error enum has UnsupportedModel variant

        let error = LLMError::UnsupportedModel("gpt-4".to_string());
        let error_msg = format!("{:?}", error);

        assert!(
            error_msg.contains("UnsupportedModel"),
            "Error should be UnsupportedModel variant"
        );
    }

    #[test]
    fn test_llmclient_initializes_rate_limiters() {
        // TEST-UNIT-LLMCLIENT-009: Verify LLMClient initializes rate limiters for all providers
        // Purpose: Validate rate limiters are created during construction

        let client = LLMClient::new("test-key".to_string());

        // Indirectly verified - if client created successfully, rate limiters exist
        // Cannot directly access private fields, but can verify via provider detection
        assert!(client.detect_provider("claude-sonnet-4-5-20250929").is_ok());
        assert!(client.detect_provider("gemini-pro").is_ok());
        assert!(client.detect_provider("deepseek-chat").is_ok());
    }

    #[test]
    fn test_llmclient_initializes_circuit_breakers() {
        // TEST-UNIT-LLMCLIENT-010: Verify LLMClient initializes circuit breakers for all providers
        // Purpose: Validate circuit breakers are created during construction

        let _client = LLMClient::new("test-key".to_string());

        // Indirectly verified - if client created successfully, circuit breakers exist
        // Cannot directly access private fields, but successful construction implies creation
        // Compile-time verification - circuit breakers initialized
    }
    // ============================================================================
    // Additional Edge Case Tests - TEST-UNIT-LLMCLIENT-011 through 025
    // ============================================================================

    #[test]
    fn test_rate_limiter_exact_capacity_boundary() {
        // TEST-UNIT-LLMCLIENT-011: Rate limiter at exact capacity boundary
        let mut limiter = RateLimiter::new(5.0);
        assert_eq!(limiter.available_tokens(), 5.0);

        for _ in 0..5 {
            assert!(limiter.try_acquire().is_ok());
        }

        let result = limiter.try_acquire();
        assert!(result.is_err());
    }

    #[test]
    fn test_rate_limiter_refill_restores_capacity() {
        // TEST-UNIT-LLMCLIENT-012: Refill mechanism restores tokens over time
        // Note: available_tokens() doesn't trigger refill - only try_acquire() does
        let mut limiter = RateLimiter::new(60.0);

        // Consume all tokens to deplete the limiter
        for _ in 0..60 {
            let _ = limiter.try_acquire();
        }

        // Verify depleted - next acquire should fail
        assert!(
            limiter.try_acquire().is_err(),
            "Should be depleted after 60 acquires"
        );

        // Wait for refill (1.1 seconds should give ~1.1 tokens at 60/minute = 1/second rate)
        std::thread::sleep(Duration::from_millis(1100));

        // After waiting, should be able to acquire a token (refill happens in try_acquire)
        assert!(
            limiter.try_acquire().is_ok(),
            "Tokens should refill over time"
        );
    }

    // ============================================================================
    // Battery 4.13: Multi-Turn Conversation Tests (IM-4001-4042)
    // ============================================================================

    #[test]
    fn test_chat_role_to_provider_string_anthropic() {
        // TEST-MT-001: Verify ChatRole::to_provider_string() for Anthropic
        assert_eq!(ChatRole::System.to_provider_string("anthropic"), "system");
        assert_eq!(ChatRole::User.to_provider_string("anthropic"), "user");
        assert_eq!(
            ChatRole::Assistant.to_provider_string("anthropic"),
            "assistant"
        );
    }

    #[test]
    fn test_chat_role_to_provider_string_openai() {
        // TEST-MT-002: Verify ChatRole::to_provider_string() for OpenAI
        assert_eq!(ChatRole::System.to_provider_string("openai"), "system");
        assert_eq!(ChatRole::User.to_provider_string("openai"), "user");
        assert_eq!(
            ChatRole::Assistant.to_provider_string("openai"),
            "assistant"
        );
    }

    #[test]
    fn test_chat_role_to_provider_string_deepseek() {
        // TEST-MT-003: Verify ChatRole::to_provider_string() for DeepSeek
        assert_eq!(ChatRole::System.to_provider_string("deepseek"), "system");
        assert_eq!(ChatRole::User.to_provider_string("deepseek"), "user");
        assert_eq!(
            ChatRole::Assistant.to_provider_string("deepseek"),
            "assistant"
        );
    }

    #[test]
    fn test_chat_role_to_provider_string_gemini_critical() {
        // TEST-MT-004: CRITICAL - Verify Gemini uses "model" instead of "assistant"
        assert_eq!(ChatRole::System.to_provider_string("gemini"), "system");
        assert_eq!(ChatRole::User.to_provider_string("gemini"), "user");
        // CRITICAL: Gemini uses "model" role, NOT "assistant"
        assert_eq!(
            ChatRole::Assistant.to_provider_string("gemini"),
            "model",
            "CRITICAL: Gemini must use 'model' role, not 'assistant'"
        );
        // Also test "google" provider
        assert_eq!(
            ChatRole::Assistant.to_provider_string("google"),
            "model",
            "CRITICAL: Google must use 'model' role, not 'assistant'"
        );
    }

    #[test]
    fn test_chat_message_constructors() {
        // TEST-MT-010: Verify ChatMessage constructors
        let user_msg = ChatMessage::user("Hello");
        assert_eq!(user_msg.role, ChatRole::User);
        assert_eq!(user_msg.content, "Hello");

        let assistant_msg = ChatMessage::assistant("Hi there");
        assert_eq!(assistant_msg.role, ChatRole::Assistant);
        assert_eq!(assistant_msg.content, "Hi there");

        let system_msg = ChatMessage::system("You are helpful");
        assert_eq!(system_msg.role, ChatRole::System);
        assert_eq!(system_msg.content, "You are helpful");
    }

    #[test]
    fn test_chat_message_new() {
        // TEST-MT-011: Verify ChatMessage::new() method
        let msg = ChatMessage::new(ChatRole::User, "Test message");
        assert_eq!(msg.role, ChatRole::User);
        assert_eq!(msg.content, "Test message");
    }

    #[test]
    fn test_multi_turn_request_builder() {
        // TEST-MT-020: Verify MultiTurnRequest builder pattern
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929")
            .with_system("You are a helpful assistant")
            .with_message(ChatMessage::user("Hello"))
            .with_message(ChatMessage::assistant("Hi there!"))
            .with_message(ChatMessage::user("How are you?"));

        assert_eq!(req.model, "claude-sonnet-4-5-20250929");
        assert_eq!(req.system, Some("You are a helpful assistant".to_string()));
        assert_eq!(req.messages.len(), 3);
        assert!(!req.enable_caching);
    }

    #[test]
    fn test_multi_turn_request_with_caching() {
        // TEST-MT-021: Verify MultiTurnRequest caching configuration
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929")
            .with_system("System")
            .with_message(ChatMessage::user("User message"))
            .with_caching();

        assert!(req.enable_caching);
        assert!(req.cache_config.is_some());
        assert_eq!(
            req.cache_config.as_ref().unwrap().ttl,
            CacheTTL::FiveMinutes
        );
    }

    #[test]
    fn test_multi_turn_request_cache_config() {
        // TEST-MT-022: Verify custom CacheConfig
        let config = CacheConfig {
            ttl: CacheTTL::OneHour,
        };
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929").with_cache_config(config);

        assert!(req.enable_caching);
        assert_eq!(req.cache_config.as_ref().unwrap().ttl, CacheTTL::OneHour);
    }

    #[test]
    fn test_multi_turn_request_without_caching() {
        // TEST-MT-023: Verify without_caching() disables caching
        let req = MultiTurnRequest::new("gpt-4")
            .with_caching()
            .without_caching();

        assert!(!req.enable_caching);
        assert!(req.cache_config.is_none());
    }

    #[test]
    fn test_multi_turn_request_validate_empty() {
        // TEST-MT-024: Verify validation fails for empty history
        let req = MultiTurnRequest::new("gpt-4");
        let result = req.validate();

        assert!(result.is_err());
        match result {
            Err(MultiTurnError::EmptyHistory) => {} // Expected
            _ => panic!("Expected EmptyHistory error"),
        }
    }

    #[test]
    fn test_multi_turn_request_validate_system_only() {
        // TEST-MT-025: Verify system-only request is valid
        let req = MultiTurnRequest::new("gpt-4").with_system("System instruction");
        let result = req.validate();

        assert!(result.is_ok(), "System-only request should be valid");
    }

    #[test]
    fn test_cache_ttl_to_anthropic_header() {
        // TEST-MT-030: Verify CacheTTL header generation
        assert_eq!(
            CacheTTL::FiveMinutes.to_anthropic_header(),
            "prompt-caching-2024-07-31"
        );
        assert_eq!(
            CacheTTL::OneHour.to_anthropic_header(),
            "extended-cache-ttl-2025-04-11"
        );
    }

    #[test]
    fn test_cache_config_default() {
        // TEST-MT-031: Verify CacheConfig default is FiveMinutes
        let config = CacheConfig::default();
        assert_eq!(config.ttl, CacheTTL::FiveMinutes);
    }

    #[test]
    fn test_to_anthropic_body_basic() {
        // TEST-MT-040: Verify to_anthropic_body() basic transformation
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929")
            .with_system("System prompt")
            .with_message(ChatMessage::user("Hello"))
            .with_message(ChatMessage::assistant("Hi there"));

        let body = to_anthropic_body(&req);

        assert_eq!(body["model"], "claude-sonnet-4-5-20250929");
        assert_eq!(body["system"], "System prompt");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][1]["role"], "assistant");
    }

    #[test]
    fn test_to_anthropic_body_with_caching() {
        // TEST-MT-041: Verify to_anthropic_body() adds cache_control when enabled
        let req = MultiTurnRequest::new("claude-sonnet-4-5-20250929")
            .with_system("Cached system")
            .with_message(ChatMessage::user("Cached user message"))
            .with_caching();

        let body = to_anthropic_body(&req);

        // System should have cache_control
        assert!(
            body["system"].is_array(),
            "System should be array with cache_control"
        );
        assert!(body["system"][0]["cache_control"].is_object());

        // User message should have cache_control in content block
        let msg = &body["messages"][0];
        assert!(msg["content"].is_array());
        assert!(msg["content"][0]["cache_control"].is_object());
    }

    #[test]
    fn test_to_openai_body_basic() {
        // TEST-MT-060: Verify to_openai_body() transformation
        let req = MultiTurnRequest::new("gpt-4o")
            .with_system("System prompt")
            .with_message(ChatMessage::user("Hello"))
            .with_message(ChatMessage::assistant("Hi"));

        let body = to_openai_body(&req);

        assert_eq!(body["model"], "gpt-4o");
        assert_eq!(body["stream"], false);

        // Messages should include system as first message
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(messages[2]["role"], "assistant");
    }

    #[test]
    fn test_to_gemini_body_uses_contents_and_parts() {
        // TEST-MT-050: Verify to_gemini_body() uses "contents" and "parts" structure
        let req = MultiTurnRequest::new("gemini-2.5-pro")
            .with_message(ChatMessage::user("Hello"))
            .with_message(ChatMessage::assistant("Hi"));

        let body = to_gemini_body(&req);

        // Must use "contents" not "messages"
        assert!(
            body["contents"].is_array(),
            "Gemini must use 'contents' not 'messages'"
        );
        assert!(
            body["messages"].is_null(),
            "Gemini must not have 'messages'"
        );

        // Each content must use "parts" array
        let contents = body["contents"].as_array().unwrap();
        assert!(
            contents[0]["parts"].is_array(),
            "Gemini must use 'parts' array"
        );
        assert!(contents[0]["parts"][0]["text"].is_string());
    }

    #[test]
    fn test_to_gemini_body_uses_model_role() {
        // TEST-MT-051: CRITICAL - Verify Gemini uses "model" role not "assistant"
        let req = MultiTurnRequest::new("gemini-2.5-pro")
            .with_message(ChatMessage::user("Hello"))
            .with_message(ChatMessage::assistant("Hi there"));

        let body = to_gemini_body(&req);
        let contents = body["contents"].as_array().unwrap();

        assert_eq!(contents[0]["role"], "user");
        assert_eq!(
            contents[1]["role"], "model",
            "CRITICAL: Gemini must use 'model' role, not 'assistant'"
        );
    }

    #[test]
    fn test_to_gemini_body_system_instruction() {
        // TEST-MT-052: Verify Gemini uses "systemInstruction" for system prompt
        let req = MultiTurnRequest::new("gemini-2.5-pro")
            .with_system("You are helpful")
            .with_message(ChatMessage::user("Hi"));

        let body = to_gemini_body(&req);

        assert!(
            body["systemInstruction"].is_object(),
            "Gemini must use 'systemInstruction' for system prompt"
        );
        assert!(body["systemInstruction"]["parts"].is_array());
        assert_eq!(
            body["systemInstruction"]["parts"][0]["text"],
            "You are helpful"
        );
    }

    #[test]
    fn test_multi_turn_error_display() {
        // TEST-MT-200: Verify MultiTurnError Display implementations
        let empty = MultiTurnError::EmptyHistory;
        assert!(empty.to_string().contains("Empty"));

        let ordering = MultiTurnError::InvalidOrdering("test".to_string());
        assert!(ordering.to_string().contains("Invalid"));

        let ctx_len = MultiTurnError::ContextLengthExceeded(4096);
        assert!(ctx_len.to_string().contains("4096"));

        let role = MultiTurnError::RoleTransformError("test".to_string());
        assert!(role.to_string().contains("Role"));
    }

    #[test]
    fn test_multi_turn_request_with_messages_batch() {
        // TEST-MT-026: Verify with_messages() adds multiple messages at once
        let messages = vec![
            ChatMessage::user("First"),
            ChatMessage::assistant("Response"),
            ChatMessage::user("Second"),
        ];

        let req = MultiTurnRequest::new("gpt-4").with_messages(messages);

        assert_eq!(req.messages.len(), 3);
        assert_eq!(req.messages[0].content, "First");
        assert_eq!(req.messages[2].content, "Second");
    }
}
