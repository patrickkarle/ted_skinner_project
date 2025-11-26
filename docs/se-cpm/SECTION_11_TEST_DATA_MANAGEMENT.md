# Section 11: Test Data Management

## 11.1 Overview

Test data management defines how test data is **created, stored, secured, and destroyed** throughout the test lifecycle. Proper test data management prevents:

- ❌ Secrets leakage (API keys, credentials in version control)
- ❌ Test brittleness (hardcoded values, environmental dependencies)
- ❌ Data pollution (test data affecting production systems)
- ❌ Non-deterministic tests (random data, timing dependencies)

**Security Sensitivity**: Fullintel handles sensitive data (API keys for Claude, Perplexity, Tavily). Test fixtures MUST NOT contain real credentials.

---

## 11.2 Test Data Classification

Test data falls into **4 categories** with different security and lifecycle requirements:

### 11.2.1 Category 1: Mock Data (In-Memory, Synthetic)

**Definition**: Synthetic data generated programmatically, never persisted, discarded after test.

**Characteristics**:
- ✅ No external dependencies
- ✅ Fully deterministic (same input → same output)
- ✅ Fast generation (< 1ms)
- ✅ No cleanup required (garbage collected)

**Use Cases**:
- Unit tests for pure functions
- Configuration validation tests
- Data structure tests

**Example**:
```rust
// tests/common/mock_data.rs

/// Generate mock API key configuration (NOT a real key)
pub fn mock_api_key_config() -> ApiKeyConfig {
    ApiKeyConfig {
        claude_api_key: "sk-ant-api03-mock-key-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        perplexity_api_key: "pplx-mock-key-BBBBBBBBBBBBBBBBBBBBBBBB".to_string(),
        tavily_api_key: "tvly-mock-key-CCCCCCCCCCCCCCCCCCCCCCCC".to_string(),
        models: vec![
            ModelConfig {
                name: "claude-3-5-sonnet-20241022".to_string(),
                provider: "anthropic".to_string(),
                max_tokens: 8192,
            },
        ],
    }
}

/// Generate mock search results for testing
pub fn mock_search_results(count: usize) -> Vec<SearchResult> {
    (0..count)
        .map(|i| SearchResult {
            id: format!("result-{:04}", i),
            title: format!("Mock Result {}", i),
            snippet: format!("This is mock content for result {}", i),
            url: format!("https://example.com/result/{}", i),
            relevance_score: 0.95 - (i as f64 * 0.01),
        })
        .collect()
}
```

**Storage**: None (ephemeral)
**Cleanup**: Automatic (Rust's RAII)

---

### 11.2.2 Category 2: Fixture Data (Version-Controlled, Static)

**Definition**: Static test data files stored in version control, representing known-good inputs/outputs.

**Characteristics**:
- ✅ Version-controlled (tests stay reproducible over time)
- ✅ Shared across tests (reusable)
- ✅ Read-only (immutable during test execution)
- ✅ Contains NO sensitive data (safe for public repos)

**Use Cases**:
- Integration tests with predefined inputs
- Regression tests (expected output snapshots)
- Configuration file parsing tests

**Example**:
```rust
// tests/fixtures/sample_project_state.json
{
  "project_id": "TEST-PROJECT-001",
  "name": "Sample Test Project",
  "target_company": "ACME Corp (Test)",
  "search_queries": [
    "ACME Corp sales strategy",
    "ACME Corp key decision makers"
  ],
  "created_at": "2024-01-15T10:00:00Z",
  "last_modified": "2024-01-15T10:00:00Z"
}
```

**Loading Fixtures**:
```rust
// tests/common/fixtures.rs

use std::path::PathBuf;

pub fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn load_project_state_fixture() -> ProjectState {
    let path = fixture_path("sample_project_state.json");
    let content = std::fs::read_to_string(path)
        .expect("Failed to read fixture file");
    serde_json::from_str(&content)
        .expect("Failed to parse fixture JSON")
}

#[test]
fn test_load_project_state_from_fixture() {
    let state = load_project_state_fixture();
    assert_eq!(state.project_id, "TEST-PROJECT-001");
    assert_eq!(state.name, "Sample Test Project");
}
```

**Storage**: `tests/fixtures/` directory
**Cleanup**: None required (read-only)

**Security Rule**: NEVER commit real API keys, credentials, or PII to fixtures. Use placeholder values like `"MOCK_KEY_..."` or `"test@example.com"`.

---

### 11.2.3 Category 3: Anonymized Data (Derived from Production, Sanitized)

**Definition**: Real production-like data that has been anonymized/sanitized to remove sensitive information.

**Characteristics**:
- ✅ Realistic data structure (same schema as production)
- ✅ PII removed (names, emails, API keys replaced)
- ✅ Maintains statistical properties (useful for performance testing)
- ⚠️ May be large (requires compression or sampling)

**Use Cases**:
- End-to-end tests with realistic data volume
- Performance benchmarking
- Edge case discovery (real-world data patterns)

**Example**:
```rust
// tests/fixtures/anonymized_search_results.jsonl (JSON Lines format)
{"id":"anon-001","title":"Sales Strategy Presentation","snippet":"...","url":"https://example.com/anon/001","score":0.95}
{"id":"anon-002","title":"Q4 Revenue Report","snippet":"...","url":"https://example.com/anon/002","score":0.92}
// ... (1000 more lines)
```

**Anonymization Script** (NOT committed to repo):
```rust
// scripts/anonymize_production_data.rs (NOT in version control)

use rand::Rng;

fn anonymize_search_results(input: &Path, output: &Path) {
    let results: Vec<SearchResult> = read_json_lines(input);

    let anonymized: Vec<_> = results
        .into_iter()
        .map(|mut r| {
            // Replace real URLs with example.com
            r.url = format!("https://example.com/anon/{}", r.id);

            // Replace real company names with ACME Corp variants
            r.title = r.title.replace(|c: char| !c.is_ascii_alphanumeric(), "X");
            r.snippet = anonymize_text(&r.snippet);

            // Preserve relevance score distribution
            r.score = (r.score * 100.0).round() / 100.0;

            r
        })
        .collect();

    write_json_lines(output, &anonymized);
}
```

**Storage**: `tests/fixtures/anonymized/` (gitignored if large)
**Cleanup**: Delete after test run (for large datasets)

**Security Rule**: Manually review anonymized data before committing. Use `git-secrets` or `truffleHog` to scan for accidental credential leaks.

---

### 11.2.4 Category 4: Encrypted Test Secrets (API Keys for Integration Tests)

**Definition**: Real API keys/credentials required for integration tests, stored encrypted in version control.

**Characteristics**:
- ✅ Encrypted at rest (AES-256-GCM)
- ✅ Decrypted only in CI/CD environment (via environment variable)
- ✅ Never logged or printed
- ⚠️ Limited to test-only accounts (not production credentials)

**Use Cases**:
- Integration tests calling real LLM APIs (with test accounts)
- E2E tests requiring actual authentication

**Encryption Setup**:
```rust
// tests/common/test_secrets.rs

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as base64, Engine};

/// Decrypt test API keys using encryption key from environment
pub fn load_encrypted_api_keys() -> Result<ApiKeyConfig, TestError> {
    // 1. Get encryption key from environment (set in CI/CD only)
    let encryption_key = std::env::var("TEST_SECRETS_KEY")
        .map_err(|_| TestError::MissingEncryptionKey)?;

    // 2. Read encrypted file
    let encrypted_path = fixture_path("encrypted/api_keys.enc");
    let encrypted_data = std::fs::read(&encrypted_path)?;

    // 3. Decrypt
    let key = base64.decode(encryption_key)?;
    let cipher = Aes256Gcm::new_from_slice(&key)?;

    let nonce = Nonce::from_slice(&encrypted_data[0..12]);
    let ciphertext = &encrypted_data[12..];

    let decrypted = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| TestError::DecryptionFailed)?;

    // 4. Parse JSON
    let config: ApiKeyConfig = serde_json::from_slice(&decrypted)?;

    Ok(config)
}

#[test]
#[ignore] // Only run when TEST_SECRETS_KEY is set
fn test_real_claude_api_call() {
    let keys = load_encrypted_api_keys().expect("Failed to load test API keys");

    let client = LLMClient::new(LLMConfig {
        api_key: keys.claude_api_key,
        model: "claude-3-5-sonnet-20241022".to_string(),
        ..Default::default()
    });

    let response = client.query("Say 'test'", QueryOptions::default()).await;
    assert!(response.is_ok());
}
```

**Encryption Script** (run once to create encrypted file):
```bash
#!/bin/bash
# scripts/encrypt_test_secrets.sh

# Generate encryption key (32 bytes for AES-256)
ENCRYPTION_KEY=$(openssl rand -base64 32)

# Encrypt test API keys
openssl enc -aes-256-gcm \
  -in tests/fixtures/plain/api_keys.json \
  -out tests/fixtures/encrypted/api_keys.enc \
  -K $(echo -n "$ENCRYPTION_KEY" | base64 -d | xxd -p) \
  -iv $(openssl rand -hex 12)

echo "Encryption key (store in CI/CD environment):"
echo "$ENCRYPTION_KEY"
```

**CI/CD Configuration**:
```yaml
# .github/workflows/test.yml

env:
  TEST_SECRETS_KEY: ${{ secrets.TEST_SECRETS_ENCRYPTION_KEY }}

jobs:
  integration-tests:
    steps:
      - name: Run Integration Tests with Real APIs
        run: cargo test --test integration_* -- --ignored
```

**Storage**:
- Encrypted file: `tests/fixtures/encrypted/api_keys.enc` (committed)
- Encryption key: GitHub Secrets (NOT in version control)

**Cleanup**: None required (encrypted data is safe at rest)

**Security Rules**:
1. ✅ Use **test-only API accounts** (not production)
2. ✅ Set low rate limits on test accounts (prevent abuse)
3. ✅ Rotate test keys quarterly
4. ✅ Monitor test account usage for anomalies
5. ❌ NEVER log decrypted API keys (even in debug mode)

---

## 11.3 Test Data Lifecycle

### 11.3.1 Setup Phase (Before Test Execution)

**Operations**:
1. Load fixtures from disk
2. Decrypt test secrets (if needed)
3. Generate mock data
4. Initialize temporary databases/files

**Example**:
```rust
// tests/common/setup.rs

pub struct TestContext {
    pub temp_dir: TempDir,
    pub db_path: PathBuf,
    pub api_keys: Option<ApiKeyConfig>,
}

impl TestContext {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Load encrypted API keys (if available)
        let api_keys = load_encrypted_api_keys().ok();

        Self {
            temp_dir,
            db_path,
            api_keys,
        }
    }

    pub fn with_mock_data(mut self) -> Self {
        // Pre-populate database with mock data
        let manager = StateManager::new(&self.db_path).unwrap();
        manager.save_project_state(&mock_project_state()).unwrap();
        self
    }
}

#[test]
fn test_with_setup_context() {
    let ctx = TestContext::new().with_mock_data();

    // Test uses pre-populated database
    let manager = StateManager::new(&ctx.db_path).unwrap();
    let state = manager.load_project_state().unwrap();

    assert_eq!(state.project_id, "MOCK-001");
}
```

### 11.3.2 Execution Phase (During Test)

**Best Practices**:
1. ✅ Use immutable references to shared data
2. ✅ Clone data if test needs to mutate it
3. ✅ Avoid test interdependencies (each test fully independent)
4. ✅ Use deterministic ordering (sort before assertions)

**Example**:
```rust
#[test]
fn test_search_results_sorting() {
    let mut results = mock_search_results(10);

    // Shuffle to ensure test doesn't rely on insertion order
    results.shuffle(&mut thread_rng());

    // Sort by relevance score
    sort_by_relevance(&mut results);

    // Assert deterministic order
    for i in 0..results.len() - 1 {
        assert!(results[i].score >= results[i + 1].score);
    }
}
```

### 11.3.3 Teardown Phase (After Test Completion)

**Operations**:
1. Delete temporary files/databases
2. Close database connections
3. Clear in-memory caches
4. (Optional) Verify no test data leaked to production

**Automatic Teardown** (using Rust's Drop trait):
```rust
impl Drop for TestContext {
    fn drop(&mut self) {
        // TempDir automatically deleted when TestContext goes out of scope
        // No manual cleanup needed!
    }
}
```

**Manual Teardown** (for external resources):
```rust
#[test]
fn test_with_manual_teardown() {
    let mock_server = start_mock_llm_server();

    // Test uses mock server
    let client = LLMClient::new(mock_config());
    let response = client.query("test", QueryOptions::default()).await;
    assert!(response.is_ok());

    // Manual cleanup
    mock_server.stop();
}
```

---

## 11.4 Test Data Management Utilities

### 11.4.1 Centralized Test Data Manager

**Location**: `tests/common/test_data_manager.rs`

```rust
// tests/common/test_data_manager.rs

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

/// Centralized test data manager for sharing fixtures across tests
pub struct TestDataManager {
    fixtures: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl TestDataManager {
    pub fn new() -> Self {
        Self {
            fixtures: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Load fixture file (cached for performance)
    pub fn load_fixture(&self, name: &str) -> Vec<u8> {
        let mut cache = self.fixtures.lock().unwrap();

        if let Some(data) = cache.get(name) {
            return data.clone();
        }

        let path = fixture_path(name);
        let data = std::fs::read(&path)
            .unwrap_or_else(|_| panic!("Fixture not found: {}", name));

        cache.insert(name.to_string(), data.clone());
        data
    }

    /// Load JSON fixture
    pub fn load_json_fixture<T: serde::de::DeserializeOwned>(&self, name: &str) -> T {
        let data = self.load_fixture(name);
        serde_json::from_slice(&data)
            .unwrap_or_else(|e| panic!("Failed to parse JSON fixture {}: {}", name, e))
    }

    /// Generate deterministic mock data (seeded for reproducibility)
    pub fn generate_mock_results(&self, count: usize, seed: u64) -> Vec<SearchResult> {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);

        (0..count)
            .map(|i| SearchResult {
                id: format!("mock-{:04}", i),
                title: format!("Result {}", i),
                snippet: generate_random_text(&mut rng, 200),
                url: format!("https://example.com/{}", i),
                relevance_score: rng.gen_range(0.5..1.0),
            })
            .collect()
    }
}

// Global singleton for test data manager
lazy_static! {
    pub static ref TEST_DATA: TestDataManager = TestDataManager::new();
}

#[test]
fn test_fixture_caching() {
    let data1 = TEST_DATA.load_fixture("sample_project_state.json");
    let data2 = TEST_DATA.load_fixture("sample_project_state.json");

    // Same data returned (cached)
    assert_eq!(data1, data2);
}
```

### 11.4.2 Deterministic Random Data Generator

**Purpose**: Generate random-looking data that is reproducible across test runs.

```rust
// tests/common/deterministic_random.rs

use rand::{Rng, SeedableRng};
use rand::rngs::StdRng;

pub struct DeterministicRandom {
    rng: StdRng,
}

impl DeterministicRandom {
    /// Create generator with fixed seed (same seed = same output)
    pub fn new(seed: u64) -> Self {
        Self {
            rng: StdRng::seed_from_u64(seed),
        }
    }

    pub fn random_company_name(&mut self) -> String {
        let prefixes = ["Tech", "Global", "Advanced", "Future", "Smart"];
        let suffixes = ["Corp", "Industries", "Solutions", "Systems", "Dynamics"];

        let prefix = prefixes[self.rng.gen_range(0..prefixes.len())];
        let suffix = suffixes[self.rng.gen_range(0..suffixes.len())];

        format!("{} {}", prefix, suffix)
    }

    pub fn random_email(&mut self) -> String {
        let usernames = ["john.doe", "jane.smith", "bob.jones", "alice.wang"];
        let username = usernames[self.rng.gen_range(0..usernames.len())];

        format!("{}@example.com", username)
    }
}

#[test]
fn test_deterministic_random() {
    let mut gen1 = DeterministicRandom::new(42);
    let mut gen2 = DeterministicRandom::new(42);

    // Same seed produces same sequence
    assert_eq!(gen1.random_company_name(), gen2.random_company_name());
    assert_eq!(gen1.random_email(), gen2.random_email());
}
```

---

## 11.5 Security Best Practices

### 11.5.1 Preventing Secret Leakage

**Pre-Commit Hook** (`.git/hooks/pre-commit`):
```bash
#!/bin/bash
# Prevent accidental commit of real API keys

# Patterns that indicate real secrets (NOT test mocks)
FORBIDDEN_PATTERNS=(
  "sk-ant-api03-[^m]"  # Real Claude keys (NOT "sk-ant-api03-mock")
  "pplx-[^m]"          # Real Perplexity keys (NOT "pplx-mock")
  "Bearer [A-Za-z0-9]{32,}"  # Real OAuth tokens
)

for pattern in "${FORBIDDEN_PATTERNS[@]}"; do
  if git diff --cached | grep -E "$pattern"; then
    echo "❌ ERROR: Detected potential secret in staged changes"
    echo "Pattern: $pattern"
    echo "Aborting commit. Please remove secrets and use mock values."
    exit 1
  fi
done

echo "✅ No secrets detected"
exit 0
```

### 11.5.2 Test Secret Rotation

**Quarterly Rotation Procedure**:
1. Generate new test API keys on provider platforms
2. Re-encrypt using `scripts/encrypt_test_secrets.sh`
3. Update `TEST_SECRETS_KEY` in GitHub Secrets
4. Revoke old test API keys
5. Run full integration test suite to verify new keys work

### 11.5.3 Logging Safety

**Rule**: NEVER log API keys, even in debug/test mode.

```rust
// ❌ BAD: Logs full API key
log::debug!("Using API key: {}", config.api_key);

// ✅ GOOD: Logs truncated key for debugging
log::debug!("Using API key: {}...", &config.api_key[..12]);

// ✅ BETTER: No logging at all
log::debug!("API client initialized");
```

---

## 11.6 Traceability to Test Categories

| Data Category | Test Types | IM Codes |
|---------------|------------|----------|
| Mock Data | Unit tests (TEST-UNIT-*) | All IM codes (1001-7201) |
| Fixture Data | Integration tests (TEST-INTEGRATION-*) | IM-2001, IM-3001, IM-5001, IM-6001 |
| Anonymized Data | E2E tests (TEST-E2E-*), Performance tests (TEST-PERF-*) | IM-2001, IM-3001, IM-6001 |
| Encrypted Secrets | Integration tests with real APIs | IM-1002, IM-3001, IM-3010, IM-3011 |

---

## 11.7 Test Data Management Checklist

Before committing test code, verify:

- [ ] No real API keys in version control (use `git-secrets` scan)
- [ ] Fixtures contain only mock/anonymized data
- [ ] Encrypted secrets use test-only accounts (not production)
- [ ] Tests use deterministic data (same seed = same output)
- [ ] Temporary files cleaned up after test (use `TempDir`)
- [ ] No hardcoded paths (use `fixture_path()` helper)
- [ ] Large datasets compressed or gitignored
- [ ] Test data documented in this section (add new categories if needed)

---

**Section 11 Complete - Ready for Integration into L5-TESTPLAN**
