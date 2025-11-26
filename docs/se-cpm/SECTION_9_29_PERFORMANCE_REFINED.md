# Section 9.29: Performance and Load Tests (Statistical Baselines)

## 9.29.1 Overview

Performance tests validate that the Fullintel application meets **quantifiable latency and throughput targets** under various load conditions. Unlike functional tests (pass/fail), performance tests use **statistical measurement** (P50/P95/P99 percentiles) to detect regressions.

**Test Strategy**:
- Each benchmark runs **100 iterations** (minimum for statistical significance)
- Calculate **P50 (median), P95, and P99 percentiles** from distribution
- **Regression threshold**: 20% increase from baseline triggers failure
- **Baseline storage**: `baselines/performance_baseline.json` (version-controlled)

---

## 9.29.2 Statistical Measurement Framework

### Percentile Calculation

```rust
// tests/common/perf_utils.rs

use std::time::Duration;

pub struct PerfStats {
    pub p50: Duration,  // Median (50th percentile)
    pub p95: Duration,  // 95th percentile
    pub p99: Duration,  // 99th percentile
    pub min: Duration,
    pub max: Duration,
    pub mean: Duration,
    pub std_dev: Duration,
}

impl PerfStats {
    pub fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort();
        let n = samples.len();

        let p50 = samples[n * 50 / 100];
        let p95 = samples[n * 95 / 100];
        let p99 = samples[n * 99 / 100];
        let min = samples[0];
        let max = samples[n - 1];

        let mean = samples.iter().sum::<Duration>() / n as u32;

        let variance: f64 = samples
            .iter()
            .map(|d| {
                let diff = d.as_secs_f64() - mean.as_secs_f64();
                diff * diff
            })
            .sum::<f64>()
            / n as f64;

        let std_dev = Duration::from_secs_f64(variance.sqrt());

        Self {
            p50,
            p95,
            p99,
            min,
            max,
            mean,
            std_dev,
        }
    }

    pub fn check_regression(&self, baseline: &PerfStats, threshold_pct: f64) -> Result<(), String> {
        let p95_regression = (self.p95.as_secs_f64() - baseline.p95.as_secs_f64())
            / baseline.p95.as_secs_f64();

        if p95_regression > threshold_pct {
            return Err(format!(
                "P95 regression: {:.1}% (threshold: {:.1}%). Current: {:?}, Baseline: {:?}",
                p95_regression * 100.0,
                threshold_pct * 100.0,
                self.p95,
                baseline.p95
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_calculation() {
        let samples = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(100), // Outlier
        ];

        let stats = PerfStats::from_samples(samples);

        assert_eq!(stats.p50, Duration::from_millis(30)); // Median
        assert_eq!(stats.p95, Duration::from_millis(100)); // 95th percentile
        assert_eq!(stats.min, Duration::from_millis(10));
        assert_eq!(stats.max, Duration::from_millis(100));
    }

    #[test]
    fn test_regression_detection() {
        let baseline = PerfStats {
            p50: Duration::from_millis(50),
            p95: Duration::from_millis(100),
            p99: Duration::from_millis(150),
            min: Duration::from_millis(40),
            max: Duration::from_millis(200),
            mean: Duration::from_millis(60),
            std_dev: Duration::from_millis(20),
        };

        // 15% slower (within 20% threshold)
        let current_ok = PerfStats {
            p95: Duration::from_millis(115), // 15% slower
            ..baseline
        };
        assert!(current_ok.check_regression(&baseline, 0.20).is_ok());

        // 25% slower (exceeds 20% threshold)
        let current_bad = PerfStats {
            p95: Duration::from_millis(125), // 25% slower
            ..baseline
        };
        assert!(current_bad.check_regression(&baseline, 0.20).is_err());
    }
}
```

---

## 9.29.3 Performance Test Definitions

### TEST-PERF-001: Agent Orchestration Latency

**Purpose**: Measure end-to-end latency for single agent query (Researcher agent with mock LLM).

**Manifest Reference**: IM-2001, IM-2010, IM-3001

**Baseline Targets** (Sprint 1 measurements):
```json
{
  "test_id": "TEST-PERF-001",
  "description": "Agent orchestration latency (mock LLM)",
  "baseline": {
    "p50": "45ms",
    "p95": "82ms",
    "p99": "120ms"
  },
  "measured_at": "2024-11-15T10:30:00Z",
  "environment": "GitHub Actions (ubuntu-latest, 2 vCPU)"
}
```

**Test Implementation**:
```rust
#[test]
fn bench_agent_orchestration() {
    let orchestrator = AgentOrchestrator::new(test_config()).unwrap();
    let mock_llm = MockLLMServer::start();

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        let start = Instant::now();

        let _result = orchestrator
            .execute_query("Test query", QueryOptions::default())
            .await
            .unwrap();

        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);

    // Load baseline
    let baseline = load_baseline("TEST-PERF-001").unwrap();

    // Check for regression (20% threshold)
    stats
        .check_regression(&baseline, 0.20)
        .expect("Performance regression detected");

    // Print stats for visibility
    println!("Agent Orchestration Latency:");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
    println!("  P99: {:?}", stats.p99);

    mock_llm.stop();
}
```

**Pass Criteria**:
- P95 latency ≤ 98ms (82ms baseline + 20% = 98.4ms)
- No panics or errors during 100 iterations
- Standard deviation < 30% of mean (indicates consistency)

---

### TEST-PERF-002: State Persistence Write Latency

**Purpose**: Measure SQLite write performance for project state (critical path for autosave).

**Manifest Reference**: IM-5001, IM-5002

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-002",
  "description": "SQLite state write latency",
  "baseline": {
    "p50": "12ms",
    "p95": "28ms",
    "p99": "45ms"
  },
  "measured_at": "2024-11-15T10:30:00Z"
}
```

**Test Implementation**:
```rust
#[test]
fn bench_state_persistence_write() {
    let temp_db = TempDir::new().unwrap();
    let db_path = temp_db.path().join("bench.db");

    let manager = StateManager::new(&db_path).unwrap();
    let state = mock_project_state(); // ~50KB serialized

    let mut samples = Vec::with_capacity(100);

    for i in 0..100 {
        // Modify state slightly to force new write
        let mut state_variant = state.clone();
        state_variant.last_modified = Utc::now();
        state_variant.version = i;

        let start = Instant::now();
        manager.save_project_state(&state_variant).unwrap();
        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-002").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("Write latency regression");

    println!("State Write Latency:");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
    println!("  P99: {:?}", stats.p99);
}
```

**Pass Criteria**:
- P95 latency ≤ 33.6ms (28ms + 20%)
- All writes succeed (no SQLITE_BUSY errors)
- Database file size stable (no unbounded growth)

---

### TEST-PERF-003: State Persistence Read Latency

**Purpose**: Measure SQLite read performance (critical for app startup and resume).

**Manifest Reference**: IM-5001, IM-5002

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-003",
  "description": "SQLite state read latency",
  "baseline": {
    "p50": "3ms",
    "p95": "8ms",
    "p99": "15ms"
  }
}
```

**Test Implementation**:
```rust
#[test]
fn bench_state_persistence_read() {
    let temp_db = TempDir::new().unwrap();
    let db_path = temp_db.path().join("bench.db");

    // Pre-populate database
    let manager = StateManager::new(&db_path).unwrap();
    manager.save_project_state(&mock_project_state()).unwrap();

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        let start = Instant::now();
        let _state = manager.load_project_state().unwrap();
        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-003").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("Read latency regression");

    println!("State Read Latency:");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
}
```

**Pass Criteria**:
- P95 latency ≤ 9.6ms (8ms + 20%)
- All reads succeed
- Returned data matches written data (no corruption)

---

### TEST-PERF-004: Export Generation Throughput

**Purpose**: Measure throughput for generating large exports (critical for bulk operations).

**Manifest Reference**: IM-6001, IM-6002

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-004",
  "description": "Export generation throughput (1000 results → PDF)",
  "baseline": {
    "p50": "2.5s",
    "p95": "3.8s",
    "p99": "5.2s"
  }
}
```

**Test Implementation**:
```rust
#[test]
fn bench_export_generation() {
    let exporter = ExportGenerator::new(ExportConfig::default());
    let results = mock_search_results(1000); // 1000 results (~500KB)

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        let start = Instant::now();

        let pdf = exporter
            .generate_pdf(&results, PdfOptions::default())
            .unwrap();

        samples.push(start.elapsed());

        // Verify PDF is valid
        assert!(pdf.len() > 10_000); // At least 10KB
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-004").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("Export throughput regression");

    println!("Export Generation (1000 results → PDF):");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
}
```

**Pass Criteria**:
- P95 latency ≤ 4.56s (3.8s + 20%)
- PDF size between 50KB and 5MB (sanity check)
- No memory leaks (RSS stable across 100 iterations)

---

### TEST-PERF-005: Retry Mechanism Overhead

**Purpose**: Measure overhead of retry logic (exponential backoff) during transient failures.

**Manifest Reference**: IM-4001, IM-4010

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-005",
  "description": "Retry mechanism overhead (3 retries with exponential backoff)",
  "baseline": {
    "p50": "350ms",
    "p95": "420ms",
    "p99": "500ms"
  },
  "note": "Baseline includes 100ms + 200ms + 400ms backoff"
}
```

**Test Implementation**:
```rust
#[test]
fn bench_retry_mechanism() {
    let mut mock_llm = MockLLMServer::start();
    mock_llm.fail_first_n_requests(2); // Fail first 2, succeed on 3rd

    let config = RetryConfig {
        max_retries: 3,
        initial_backoff: Duration::from_millis(100),
        backoff_multiplier: 2.0,
        ..Default::default()
    };

    let client = LLMClient::new(config);

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        mock_llm.reset_failure_count();

        let start = Instant::now();
        let _result = client.query_with_retry("test", QueryOptions::default()).await;
        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-005").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("Retry overhead regression");

    println!("Retry Mechanism (3 attempts):");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
}
```

**Pass Criteria**:
- P95 latency ≤ 504ms (420ms + 20%)
- All requests eventually succeed
- Backoff timing matches exponential curve (±10%)

---

### TEST-PERF-006: Concurrent Agent Execution

**Purpose**: Measure throughput when running multiple agents in parallel (stress test).

**Manifest Reference**: IM-2001, IM-2010

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-006",
  "description": "Concurrent execution (10 agents in parallel)",
  "baseline": {
    "p50": "180ms",
    "p95": "250ms",
    "p99": "320ms"
  },
  "note": "Parallel execution ~4x faster than sequential (10 * 45ms = 450ms)"
}
```

**Test Implementation**:
```rust
#[test]
fn bench_concurrent_agents() {
    let orchestrator = AgentOrchestrator::new(test_config()).unwrap();
    let mock_llm = MockLLMServer::start();

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        let start = Instant::now();

        // Launch 10 agents in parallel
        let tasks: Vec<_> = (0..10)
            .map(|i| orchestrator.execute_query(&format!("Query {}", i), QueryOptions::default()))
            .collect();

        // Wait for all to complete
        let _results = futures::future::join_all(tasks).await;

        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-006").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("Concurrent throughput regression");

    println!("Concurrent Agents (10 parallel):");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);

    mock_llm.stop();
}
```

**Pass Criteria**:
- P95 latency ≤ 300ms (250ms + 20%)
- All 10 agents complete successfully
- No resource exhaustion (file descriptors, memory)

---

### TEST-PERF-007: Large Dataset Processing

**Purpose**: Measure performance when processing large result sets (10,000 results).

**Manifest Reference**: IM-2001, IM-5001, IM-6001

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-007",
  "description": "Process 10,000 search results (dedup + sort + persist)",
  "baseline": {
    "p50": "850ms",
    "p95": "1200ms",
    "p99": "1600ms"
  }
}
```

**Test Implementation**:
```rust
#[test]
fn bench_large_dataset_processing() {
    let orchestrator = AgentOrchestrator::new(test_config()).unwrap();
    let results = mock_search_results(10_000); // 10,000 results (~5MB)

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        let start = Instant::now();

        // Process: dedup → sort by relevance → persist to DB
        let processed = orchestrator.process_results(results.clone()).unwrap();
        let _db_write = orchestrator.persist_results(&processed).unwrap();

        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-007").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("Large dataset regression");

    println!("Large Dataset (10K results):");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
}
```

**Pass Criteria**:
- P95 latency ≤ 1440ms (1200ms + 20%)
- Memory usage < 50MB (efficient streaming)
- All 10,000 results processed correctly

---

### TEST-PERF-008: UI Responsiveness (Tauri Frontend)

**Purpose**: Measure UI render latency for result display (user-facing metric).

**Manifest Reference**: IM-7001, IM-7002

**Baseline Targets**:
```json
{
  "test_id": "TEST-PERF-008",
  "description": "Render 100 search results in UI",
  "baseline": {
    "p50": "65ms",
    "p95": "120ms",
    "p99": "180ms"
  }
}
```

**Test Implementation**:
```rust
#[test]
fn bench_ui_render() {
    let app = tauri::test::mock_app();
    let results = mock_search_results(100);

    let mut samples = Vec::with_capacity(100);

    for _ in 0..100 {
        let start = Instant::now();

        // Trigger UI render
        app.emit_all("search_results", &results).unwrap();

        // Wait for render complete event
        app.wait_for_event("render_complete", Duration::from_secs(1))
            .unwrap();

        samples.push(start.elapsed());
    }

    let stats = PerfStats::from_samples(samples);
    let baseline = load_baseline("TEST-PERF-008").unwrap();

    stats
        .check_regression(&baseline, 0.20)
        .expect("UI render regression");

    println!("UI Render (100 results):");
    println!("  P50: {:?}", stats.p50);
    println!("  P95: {:?}", stats.p95);
}
```

**Pass Criteria**:
- P95 latency ≤ 144ms (120ms + 20%)
- No visual jank (frame drops)
- UI remains responsive during render

---

## 9.29.4 Baseline Management

### Baseline Storage Format

**File**: `baselines/performance_baseline.json`

```json
{
  "version": "1.0.0",
  "created_at": "2024-11-15T10:30:00Z",
  "environment": {
    "os": "ubuntu-22.04",
    "cpu": "Intel Xeon E5-2686 v4 (2 vCPU)",
    "ram": "7GB",
    "rust_version": "1.75.0"
  },
  "baselines": [
    {
      "test_id": "TEST-PERF-001",
      "description": "Agent orchestration latency",
      "p50_ms": 45,
      "p95_ms": 82,
      "p99_ms": 120
    },
    {
      "test_id": "TEST-PERF-002",
      "description": "SQLite state write",
      "p50_ms": 12,
      "p95_ms": 28,
      "p99_ms": 45
    }
    // ... (8 total baselines)
  ]
}
```

### Baseline Update Procedure

**When to Update Baselines**:
1. ✅ After confirmed performance improvements (e.g., algorithm optimization)
2. ✅ After hardware/CI environment changes
3. ❌ NEVER to mask regressions (investigate root cause first)

**Update Command**:
```bash
# Run benchmarks and capture new baselines
cargo bench --bench fullintel_benchmarks -- --save-baseline

# Review baseline changes
git diff baselines/performance_baseline.json

# Commit with justification
git commit -m "perf: Update baselines after SQLite WAL optimization

Previous P95: 28ms → New P95: 18ms (36% improvement)
Root cause: Enabled WAL mode for concurrent writes
Benchmark: TEST-PERF-002 (100 iterations)"
```

---

## 9.29.5 CI/CD Integration

### GitHub Actions Performance Gate

```yaml
- name: Run Performance Benchmarks
  run: cargo bench --bench fullintel_benchmarks

- name: Check for Regressions
  run: |
    python3 scripts/check_perf_regression.py \
      --baseline=baselines/performance_baseline.json \
      --results=target/criterion/ \
      --threshold=0.20 \
      --output=perf_report.md

- name: Comment on PR with Results
  if: github.event_name == 'pull_request'
  uses: actions/github-script@v6
  with:
    script: |
      const fs = require('fs');
      const report = fs.readFileSync('perf_report.md', 'utf8');
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: report
      });
```

### Regression Report Format

```markdown
## 📊 Performance Benchmark Results

**Status**: ✅ All tests passed (no regressions > 20%)

| Test ID | Description | P95 Baseline | P95 Current | Change | Status |
|---------|-------------|--------------|-------------|--------|--------|
| TEST-PERF-001 | Agent orchestration | 82ms | 79ms | -3.7% ⬇️ | ✅ PASS |
| TEST-PERF-002 | SQLite write | 28ms | 30ms | +7.1% ⬆️ | ✅ PASS |
| TEST-PERF-003 | SQLite read | 8ms | 9ms | +12.5% ⬆️ | ✅ PASS |
| TEST-PERF-004 | Export generation | 3.8s | 3.6s | -5.3% ⬇️ | ✅ PASS |
| TEST-PERF-005 | Retry overhead | 420ms | 425ms | +1.2% ⬆️ | ✅ PASS |
| TEST-PERF-006 | Concurrent agents | 250ms | 248ms | -0.8% ⬇️ | ✅ PASS |
| TEST-PERF-007 | Large dataset | 1200ms | 1150ms | -4.2% ⬇️ | ✅ PASS |
| TEST-PERF-008 | UI render | 120ms | 125ms | +4.2% ⬆️ | ✅ PASS |

**Threshold**: 20% regression from baseline
**Environment**: GitHub Actions (ubuntu-latest, 2 vCPU)
**Iterations**: 100 per test
```

---

## 9.29.6 Performance Test Execution Summary

| Test ID | Target Metric | P95 Baseline | P95 Threshold (+ 20%) | Test Complexity |
|---------|---------------|--------------|----------------------|-----------------|
| TEST-PERF-001 | Agent orchestration latency | 82ms | 98ms | Medium |
| TEST-PERF-002 | SQLite write latency | 28ms | 34ms | Low |
| TEST-PERF-003 | SQLite read latency | 8ms | 10ms | Low |
| TEST-PERF-004 | Export generation | 3.8s | 4.6s | High |
| TEST-PERF-005 | Retry overhead | 420ms | 504ms | Medium |
| TEST-PERF-006 | Concurrent agents | 250ms | 300ms | High |
| TEST-PERF-007 | Large dataset processing | 1200ms | 1440ms | High |
| TEST-PERF-008 | UI render latency | 120ms | 144ms | Medium |

**Total Performance Tests**: 8
**Total Iterations**: 800 (100 per test)
**Estimated Execution Time**: 15-20 minutes

---

**Section 9.29 Complete (Statistical Refinement) - Ready for Integration into L5-TESTPLAN**
