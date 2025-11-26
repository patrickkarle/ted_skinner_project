# L3-CDD-02: ToolRegistry Component Design

**Document ID:** L3-CDD-FULLINTEL-002
**Component:** ToolRegistry
**Atomic Level:** MOLECULE
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L2-ICD-COMP-001
**Traceability:** L1-SAD Section 6.2, L2-ICD-03 Section 2

---

## 1. Component Overview

### 1.1 Purpose
Manages registration and execution of external tools (Tavily Search, NewsAPI, manual input) used during the 5-phase workflow.

### 1.2 Atomic Classification
- **Level:** MOLECULE (50-110 lines)
- **Rationale:** Local state (HashMap of tools), coordinated effects (async tool execution), no persistent state

### 1.3 Responsibilities
- Register tool implementations
- Execute tools by name with JSON arguments
- Provide tool schema information
- Track tool execution costs

---

## 2. File Structure

```
src-tauri/src/
├── tools/
│   ├── mod.rs                    # Public module interface
│   ├── registry.rs               # ToolRegistry implementation
│   ├── tool_trait.rs             # Tool trait definition
│   ├── tavily_search.rs          # TavilySearchTool
│   ├── news_api.rs               # NewsAPISearchTool
│   └── manual_input.rs           # ManualInputTool
```

---

## 3. Implementation Specification

### 3.1 Tool Trait (tool_trait.rs)

**File:** `src-tauri/src/tools/tool_trait.rs`
**Atomic Level:** QUARK (interface definition)

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;

/// Trait that all external tools must implement
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns unique tool identifier matching manifest
    fn name(&self) -> &str;

    /// Returns JSON schema describing tool parameters
    fn schema(&self) -> ToolSchema;

    /// Executes tool with provided arguments
    ///
    /// # Arguments
    /// * `args` - Tool-specific parameters as JSON
    ///
    /// # Returns
    /// Tool output as string, or error
    ///
    /// # Errors
    /// - Network errors (API unreachable)
    /// - Authentication errors (invalid API key)
    /// - Rate limit errors
    /// - Invalid argument errors
    async fn execute(&self, args: Value) -> Result<String>;

    /// Returns estimated cost in USD for this execution
    ///
    /// # Arguments
    /// * `args` - Tool parameters (for cost calculation)
    ///
    /// # Returns
    /// None if free, Some(cost) if paid
    fn estimate_cost(&self, _args: &Value) -> Option<f64> {
        None // Default: free
    }

    /// Returns whether tool requires API key
    fn requires_api_key(&self) -> bool {
        false // Default: no API key required
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub required_parameters: Vec<String>,
    pub cost_model: Option<CostModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,  // "string", "number", "boolean", "array"
    pub description: String,
    pub default: Option<Value>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostModel {
    PerCall { cost_usd: f64 },
    PerResult { cost_per_result_usd: f64 },
    FreeTier { limit_per_month: u32 },
}
```

---

### 3.2 ToolRegistry (registry.rs)

**File:** `src-tauri/src/tools/registry.rs`
**Atomic Level:** MOLECULE (90-100 lines)

```rust
use super::tool_trait::{Tool, ToolSchema};
use serde_json::Value;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for managing external tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    execution_log: Vec<ToolExecution>,
}

#[derive(Debug, Clone)]
pub struct ToolExecution {
    pub tool_name: String,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub cost_usd: f64,
    pub success: bool,
}

impl ToolRegistry {
    /// Creates empty tool registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            execution_log: Vec::new(),
        }
    }

    /// Registers a new tool
    ///
    /// # Arguments
    /// * `tool` - Tool implementation (Arc for thread-safe sharing)
    ///
    /// # Panics
    /// If tool with same name already registered
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();

        if self.tools.contains_key(&name) {
            panic!("Tool '{}' already registered", name);
        }

        self.tools.insert(name, tool);
    }

    /// Executes registered tool
    ///
    /// # Arguments
    /// * `name` - Tool identifier matching Tool::name()
    /// * `args` - JSON arguments for tool
    ///
    /// # Returns
    /// Tool output string, or error
    ///
    /// # Errors
    /// - Tool not registered
    /// - Tool execution failed
    /// - Invalid arguments
    pub async fn execute(&mut self, name: &str, args: Value) -> Result<String> {
        let tool = self.tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool '{}' not registered", name))?
            .clone();

        let started_at = Self::current_timestamp_ms();
        let cost = tool.estimate_cost(&args).unwrap_or(0.0);

        // Execute tool with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(30),
            tool.execute(args)
        )
        .await
        .context("Tool execution timeout (30s)")??;

        let completed_at = Self::current_timestamp_ms();

        // Log execution
        self.execution_log.push(ToolExecution {
            tool_name: name.to_string(),
            started_at,
            completed_at: Some(completed_at),
            cost_usd: cost,
            success: true,
        });

        Ok(output)
    }

    /// Lists all registered tool names
    pub fn list_available(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Gets schema for specific tool
    ///
    /// # Arguments
    /// * `name` - Tool identifier
    ///
    /// # Returns
    /// ToolSchema if tool registered, None otherwise
    pub fn get_schema(&self, name: &str) -> Option<ToolSchema> {
        self.tools.get(name).map(|tool| tool.schema())
    }

    /// Gets total cost of all tool executions
    pub fn total_cost_usd(&self) -> f64 {
        self.execution_log
            .iter()
            .filter(|e| e.success)
            .map(|e| e.cost_usd)
            .sum()
    }

    /// Gets execution statistics
    pub fn execution_stats(&self) -> ToolStats {
        let total_executions = self.execution_log.len();
        let successful = self.execution_log.iter().filter(|e| e.success).count();
        let failed = total_executions - successful;

        let total_duration_ms: u64 = self.execution_log
            .iter()
            .filter_map(|e| e.completed_at.map(|c| c - e.started_at))
            .sum();

        let avg_duration_ms = if total_executions > 0 {
            total_duration_ms / total_executions as u64
        } else {
            0
        };

        ToolStats {
            total_executions,
            successful,
            failed,
            total_cost_usd: self.total_cost_usd(),
            avg_duration_ms,
        }
    }

    fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub total_executions: usize,
    pub successful: usize,
    pub failed: usize,
    pub total_cost_usd: f64,
    pub avg_duration_ms: u64,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

---

### 3.3 TavilySearchTool (tavily_search.rs)

**File:** `src-tauri/src/tools/tavily_search.rs`
**Atomic Level:** ATOM (40-50 lines)

```rust
use super::tool_trait::{Tool, ToolSchema, ToolParameter, CostModel};
use async_trait::async_trait;
use serde_json::{json, Value};
use anyhow::{Result, Context};
use reqwest::Client;

pub struct TavilySearchTool {
    api_key: String,
    client: Client,
}

impl TavilySearchTool {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl Tool for TavilySearchTool {
    fn name(&self) -> &str {
        "search_tool"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            description: "Web search using Tavily API for company research".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "query".to_string(),
                    param_type: "string".to_string(),
                    description: "Search query (e.g., 'TechCorp revenue industry')".to_string(),
                    default: None,
                    required: true,
                },
                ToolParameter {
                    name: "max_results".to_string(),
                    param_type: "number".to_string(),
                    description: "Maximum results to return".to_string(),
                    default: Some(json!(5)),
                    required: false,
                },
            ],
            required_parameters: vec!["query".to_string()],
            cost_model: Some(CostModel::PerCall { cost_usd: 0.001 }),
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let query = args.get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required parameter: 'query'"))?;

        let max_results = args.get("max_results")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        // Call Tavily API
        let response = self.client
            .post("https://api.tavily.com/search")
            .json(&json!({
                "api_key": self.api_key,
                "query": query,
                "max_results": max_results,
                "search_depth": "advanced"
            }))
            .send()
            .await
            .context("Tavily API request failed")?;

        if !response.status().is_success() {
            anyhow::bail!("Tavily API error: {}", response.status());
        }

        let results: Value = response.json().await
            .context("Failed to parse Tavily response")?;

        // Format results for LLM consumption
        let formatted = self.format_results(&results)?;

        Ok(formatted)
    }

    fn estimate_cost(&self, _args: &Value) -> Option<f64> {
        Some(0.001) // $0.001 per search
    }

    fn requires_api_key(&self) -> bool {
        true
    }
}

impl TavilySearchTool {
    fn format_results(&self, results: &Value) -> Result<String> {
        let results_array = results.get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid Tavily response format"))?;

        let mut formatted = String::from("## Search Results\n\n");

        for (i, result) in results_array.iter().enumerate() {
            let title = result.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("No title");

            let content = result.get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("No content");

            let url = result.get("url")
                .and_then(|v| v.as_str())
                .unwrap_or("No URL");

            formatted.push_str(&format!(
                "### Result {}: {}\n**Source:** {}\n\n{}\n\n---\n\n",
                i + 1,
                title,
                url,
                content
            ));
        }

        Ok(formatted)
    }
}
```

---

### 3.4 NewsAPISearchTool (news_api.rs)

**File:** `src-tauri/src/tools/news_api.rs`
**Atomic Level:** ATOM (similar structure to TavilySearchTool)

```rust
// Similar implementation pattern
// Schema: query (company name), from (date), to (date)
// API: https://newsapi.org/v2/everything
// Cost: Free tier (100 requests/day)
```

---

### 3.5 ManualInputTool (manual_input.rs)

**File:** `src-tauri/src/tools/manual_input.rs`
**Atomic Level:** NEUTRON (20-25 lines)

```rust
use super::tool_trait::{Tool, ToolSchema, ToolParameter};
use async_trait::async_trait;
use serde_json::Value;
use anyhow::Result;

pub struct ManualInputTool;

#[async_trait]
impl Tool for ManualInputTool {
    fn name(&self) -> &str {
        "manual_input_tool"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            description: "Prompts user for manual input (fallback when APIs fail)".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "prompt".to_string(),
                    param_type: "string".to_string(),
                    description: "What to ask the user".to_string(),
                    default: None,
                    required: true,
                },
            ],
            required_parameters: vec!["prompt".to_string()],
            cost_model: None,
        }
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let prompt = args.get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'prompt' parameter"))?;

        // In real implementation, this would trigger a Tauri dialog
        // For now, return placeholder
        Ok(format!("[User would be prompted: {}]", prompt))
    }
}
```

---

## 4. Error Handling

### 4.1 Error Types
```rust
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not registered: {0}")]
    NotRegistered(String),

    #[error("Missing required parameter: {0}")]
    MissingParameter(String),

    #[error("Invalid parameter type for '{param}': expected {expected}, got {actual}")]
    InvalidParameterType {
        param: String,
        expected: String,
        actual: String,
    },

    #[error("API authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("API rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Tool execution timeout (30s)")]
    Timeout,

    #[error("Network error: {0}")]
    NetworkError(String),
}
```

### 4.2 Retry Strategy
```rust
impl ToolRegistry {
    pub async fn execute_with_retry(
        &mut self,
        name: &str,
        args: Value,
        max_retries: u32,
    ) -> Result<String> {
        let mut attempts = 0;
        let mut last_error = None;

        while attempts <= max_retries {
            match self.execute(name, args.clone()).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    last_error = Some(e);

                    if attempts <= max_retries {
                        // Exponential backoff
                        let delay_ms = 2u64.pow(attempts) * 100;
                        tokio::time::sleep(
                            std::time::Duration::from_millis(delay_ms)
                        ).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }
}
```

---

## 5. Testing Requirements

### 5.1 Unit Tests

**File:** `src-tauri/src/tools/registry.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct MockTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock_tool"
        }

        fn schema(&self) -> ToolSchema {
            ToolSchema {
                description: "Mock tool for testing".to_string(),
                parameters: vec![],
                required_parameters: vec![],
                cost_model: None,
            }
        }

        async fn execute(&self, _args: Value) -> Result<String> {
            Ok("mock output".to_string())
        }
    }

    #[test]
    fn test_register_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));

        assert_eq!(registry.list_available(), vec!["mock_tool"]);
    }

    #[tokio::test]
    async fn test_execute_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));

        let result = registry.execute("mock_tool", json!({})).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "mock output");
    }

    #[tokio::test]
    async fn test_execute_unregistered_tool() {
        let mut registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_get_schema() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));

        let schema = registry.get_schema("mock_tool");
        assert!(schema.is_some());
        assert_eq!(schema.unwrap().description, "Mock tool for testing");
    }
}
```

### 5.2 Integration Tests

**File:** `src-tauri/src/tools/integration_tests.rs`

```rust
#[cfg(test)]
mod integration_tests {
    #[tokio::test]
    #[ignore] // Requires API keys
    async fn test_tavily_search_real_api() {
        let api_key = std::env::var("TAVILY_API_KEY").unwrap();
        let tool = TavilySearchTool::new(api_key);

        let result = tool.execute(json!({
            "query": "Anthropic AI company",
            "max_results": 3
        })).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Search Results"));
    }
}
```

---

## 6. Performance Requirements

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Tool registration | < 1ms | HashMap insert |
| Schema lookup | < 0.1ms | HashMap get |
| Tool execution (search) | < 5s | API round-trip |
| Cost calculation | < 0.01ms | Arithmetic |

---

## 7. Implementation Checklist

- [ ] Implement Tool trait in `tool_trait.rs`
- [ ] Implement ToolRegistry in `registry.rs`
- [ ] Implement TavilySearchTool in `tavily_search.rs`
- [ ] Implement NewsAPISearchTool in `news_api.rs`
- [ ] Implement ManualInputTool in `manual_input.rs`
- [ ] Add comprehensive error types
- [ ] Implement retry logic with exponential backoff
- [ ] Write unit tests (80%+ coverage)
- [ ] Write integration tests (with API mocking)
- [ ] Document all public APIs with rustdoc

---

**Status:** Ready for Implementation
**Next Document:** L3-CDD-03-LLMClient.md
