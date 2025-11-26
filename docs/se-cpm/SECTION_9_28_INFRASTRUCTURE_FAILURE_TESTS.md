# Section 9.28: Infrastructure Failure Tests

## Overview

Infrastructure failure tests validate system resilience against environmental failures that are **outside application control** but can cause cascading failures if not handled properly.

**Distinction from Error Tests**:
- **Error Tests** (Section 9.21-9.27): Application-level errors (invalid input, API failures, business logic errors)
- **Infrastructure Tests** (Section 9.28): Environmental failures (DB corruption, resource exhaustion, system-level issues)

**Total Tests**: 5 infrastructure failure scenarios

---

## TEST-INFRA-001: Database Corruption Recovery

### Purpose
Validate system behavior when SQLite database file becomes corrupted (simulating disk errors, power loss, or filesystem failures).

### Test Specification

**Manifest Reference**: IM-5001, IM-5002, IM-5010

**Setup**:
```rust
#[test]
fn test_database_corruption_recovery() {
    // 1. Create valid database with active state
    let db_path = temp_dir().join("corrupt_test.db");
    let mut state_manager = StateManager::new(&db_path).unwrap();
    state_manager.save_project_state(&valid_project_state()).unwrap();

    // 2. Corrupt database file
    let mut file = OpenOptions::new()
        .write(true)
        .open(&db_path)
        .unwrap();
    file.seek(SeekFrom::Start(512)).unwrap();
    file.write_all(&[0xFF; 1024]).unwrap(); // Corrupt SQLite header
    drop(file);

    // 3. Attempt to load state
    let result = StateManager::new(&db_path);

    // Expected behavior: Detect corruption, return error
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        StateError::DatabaseCorruption { .. }
    ));
}
```

**Expected Behavior**:
1. ✅ System detects corruption via SQLite integrity check failure
2. ✅ Returns `StateError::DatabaseCorruption` with diagnostic info
3. ✅ Does NOT attempt to read corrupted data (prevents undefined behavior)
4. ✅ Logs corruption event with database path and error details
5. ✅ User receives actionable error message: "Database corrupted. Restore from backup: [path]"

**Recovery Procedure Test**:
```rust
#[test]
fn test_database_corruption_recovery_with_backup() {
    // Verify backup restoration procedure
    let db_path = temp_dir().join("corrupt_test.db");
    let backup_path = temp_dir().join("corrupt_test.db.backup");

    // 1. Create valid database and backup
    let state = valid_project_state();
    let mut manager = StateManager::new(&db_path).unwrap();
    manager.save_project_state(&state).unwrap();
    manager.create_backup(&backup_path).unwrap();

    // 2. Corrupt primary database
    corrupt_database_file(&db_path);

    // 3. Restore from backup
    let restored = StateManager::restore_from_backup(&backup_path, &db_path);
    assert!(restored.is_ok());

    // 4. Verify restored data matches original
    let manager = StateManager::new(&db_path).unwrap();
    let loaded = manager.load_project_state().unwrap();
    assert_eq!(loaded.project_id, state.project_id);
}
```

**Pass Criteria**:
- Corruption detected within 100ms
- Error message includes recovery instructions
- Backup restoration succeeds in < 5 seconds

---

## TEST-INFRA-002: Disk Full Scenario

### Purpose
Validate system behavior when disk space is exhausted during write operations (state persistence, export generation, log writing).

### Test Specification

**Manifest Reference**: IM-5001, IM-5002, IM-6001, IM-6002

**Setup** (Linux/macOS - uses loop device):
```rust
#[test]
#[cfg(unix)]
fn test_disk_full_handling() {
    // 1. Create 10MB virtual disk
    let mount_point = setup_limited_disk(10 * 1024 * 1024); // 10MB
    let db_path = mount_point.join("test.db");

    // 2. Fill disk to 95% capacity
    fill_disk_to_threshold(&mount_point, 0.95);

    // 3. Attempt large state save
    let mut manager = StateManager::new(&db_path).unwrap();
    let large_state = create_state_with_size(5 * 1024 * 1024); // 5MB

    let result = manager.save_project_state(&large_state);

    // Expected: Graceful failure with disk space error
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        StateError::DiskFull { available, required }
            if available < required
    ));

    teardown_limited_disk(mount_point);
}
```

**Windows Alternative** (uses quota limits):
```rust
#[test]
#[cfg(windows)]
fn test_disk_full_handling_windows() {
    // Use NTFS quota limits to simulate disk full
    let test_dir = setup_quota_limited_dir(10 * 1024 * 1024);

    // Fill to capacity and attempt write
    fill_to_quota(&test_dir);

    let db_path = test_dir.join("test.db");
    let manager = StateManager::new(&db_path).unwrap();
    let result = manager.save_project_state(&large_state());

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StateError::DiskFull { .. }));
}
```

**Expected Behavior**:
1. ✅ Pre-write disk space check (before attempting write)
2. ✅ Returns `StateError::DiskFull` with available/required bytes
3. ✅ No partial writes (atomic operation - all or nothing)
4. ✅ Database remains in consistent state (no corruption from failed write)
5. ✅ User receives actionable message: "Insufficient disk space. Need 5MB, have 2MB. Free space and retry."

**Pass Criteria**:
- Disk space check completes in < 10ms
- No database corruption after failed write
- Error message includes specific byte counts

---

## TEST-INFRA-003: Memory Exhaustion Handling

### Purpose
Validate system behavior under memory pressure (simulating low-memory environments, memory leaks, or large dataset processing).

### Test Specification

**Manifest Reference**: IM-2001, IM-2010, IM-3001, IM-5001

**Setup**:
```rust
#[test]
fn test_memory_exhaustion_handling() {
    // 1. Set memory limit for test (using rlimit on Unix)
    #[cfg(unix)]
    {
        use libc::{setrlimit, rlimit, RLIMIT_AS};
        let limit = rlimit {
            rlim_cur: 100 * 1024 * 1024, // 100MB soft limit
            rlim_max: 100 * 1024 * 1024,
        };
        unsafe { setrlimit(RLIMIT_AS, &limit) };
    }

    // 2. Attempt to process large dataset that exceeds limit
    let orchestrator = AgentOrchestrator::new(config()).unwrap();

    // Create 200MB worth of search results (exceeds 100MB limit)
    let huge_results = generate_mock_results(200_000); // 1KB per result

    let result = orchestrator.process_results(huge_results);

    // Expected: Graceful degradation or chunked processing
    match result {
        Ok(_) => {
            // System used chunked processing to stay under limit
            assert!(get_current_memory_usage() < 100 * 1024 * 1024);
        }
        Err(e) => {
            // System detected memory pressure and failed gracefully
            assert!(matches!(e, OrchestratorError::OutOfMemory { .. }));
        }
    }
}
```

**Chunked Processing Variant**:
```rust
#[test]
fn test_chunked_processing_under_memory_pressure() {
    let config = AgentConfig {
        max_memory_per_batch: 10 * 1024 * 1024, // 10MB per chunk
        ..Default::default()
    };

    let orchestrator = AgentOrchestrator::new(config).unwrap();

    // Process 100MB dataset in 10MB chunks
    let large_dataset = generate_mock_results(100_000);
    let result = orchestrator.process_results_chunked(large_dataset);

    assert!(result.is_ok());

    // Verify memory stayed under limit throughout
    let peak_memory = get_peak_memory_usage();
    assert!(peak_memory < 15 * 1024 * 1024); // 10MB + 5MB overhead
}
```

**Expected Behavior**:
1. ✅ Memory usage monitoring detects pressure at 80% threshold
2. ✅ Triggers chunked processing or backpressure mechanism
3. ✅ Does NOT crash with OOM (out of memory) panic
4. ✅ Logs memory pressure warnings before failure
5. ✅ Returns `OrchestratorError::OutOfMemory` if chunking impossible

**Pass Criteria**:
- System stays under configured memory limit
- Chunked processing completes successfully for datasets 10x memory limit
- Memory pressure detected within 1 second

---

## TEST-INFRA-004: Network Partition Handling

### Purpose
Validate system behavior when network connectivity to external LLM APIs is lost (simulating firewall blocks, DNS failures, or ISP outages).

### Test Specification

**Manifest Reference**: IM-3001, IM-3010, IM-3011, IM-4001

**Setup**:
```rust
#[test]
fn test_network_partition_handling() {
    // 1. Start with working network
    let llm_client = LLMClient::new(test_config()).unwrap();

    // Verify initial connectivity
    let health = llm_client.health_check().await;
    assert!(health.is_ok());

    // 2. Simulate network partition (redirect to blackhole)
    #[cfg(test)]
    {
        // Override DNS to point to non-routable address
        std::env::set_var("LLM_API_HOST", "10.255.255.1"); // TEST-NET-3 (RFC 5737)
    }

    // 3. Attempt LLM query during partition
    let result = llm_client
        .query("Test query", QueryOptions::default())
        .await;

    // Expected: Timeout with network error, NOT panic
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        LLMError::NetworkError { .. }
    ));
}
```

**Timeout Behavior Test**:
```rust
#[test]
fn test_network_timeout_respects_deadline() {
    let config = LLMConfig {
        request_timeout: Duration::from_secs(5),
        max_retries: 0, // Disable retries for this test
        ..Default::default()
    };

    let client = LLMClient::new(config).unwrap();

    // Point to blackhole address
    std::env::set_var("LLM_API_HOST", "10.255.255.1");

    let start = Instant::now();
    let result = client.query("test", QueryOptions::default()).await;
    let elapsed = start.elapsed();

    // Verify timeout occurred within tolerance window
    assert!(result.is_err());
    assert!(elapsed >= Duration::from_secs(5));
    assert!(elapsed < Duration::from_secs(6)); // Max 1s overshoot
}
```

**Retry Mechanism Test**:
```rust
#[test]
fn test_network_partition_retry_backoff() {
    let config = LLMConfig {
        request_timeout: Duration::from_secs(2),
        max_retries: 3,
        retry_backoff: ExponentialBackoff::new(Duration::from_millis(100)),
        ..Default::default()
    };

    let client = LLMClient::new(config).unwrap();

    // Simulate transient network issue
    let mut mock_server = start_flaky_mock_server();
    mock_server.fail_first_n_requests(2); // Fail first 2, succeed on 3rd

    let result = client.query("test", QueryOptions::default()).await;

    // Should succeed on 3rd retry
    assert!(result.is_ok());

    // Verify retry count
    assert_eq!(mock_server.request_count(), 3);
}
```

**Expected Behavior**:
1. ✅ Request timeout enforced (default: 30s)
2. ✅ Exponential backoff retry (100ms, 200ms, 400ms)
3. ✅ Max retries respected (default: 3)
4. ✅ Returns `LLMError::NetworkError` after exhausting retries
5. ✅ Circuit breaker opens after 5 consecutive failures (prevents thundering herd)

**Pass Criteria**:
- Timeout fires within ±10% of configured duration
- Retry backoff follows exponential pattern (±5%)
- Circuit breaker prevents cascading failures

---

## TEST-INFRA-005: System Clock Skew Handling

### Purpose
Validate system behavior when system clock jumps (simulating NTP corrections, DST changes, or manual clock adjustments).

### Test Specification

**Manifest Reference**: IM-5001, IM-5002, IM-5020, IM-6001

**Setup**:
```rust
#[test]
fn test_clock_skew_handling() {
    // 1. Initialize state manager with current time
    let manager = StateManager::new(":memory:").unwrap();
    let initial_state = create_project_state();
    manager.save_project_state(&initial_state).unwrap();

    let t1 = manager.get_last_modified_time().unwrap();

    // 2. Simulate clock jump backward (e.g., NTP correction)
    #[cfg(test)]
    {
        use mock_time::MockClock;
        MockClock::set_time(t1 - Duration::from_hours(2));
    }

    // 3. Attempt another save operation
    let updated_state = modify_project_state(&initial_state);
    let result = manager.save_project_state(&updated_state);

    // Expected: Detect clock skew, handle gracefully
    match result {
        Ok(_) => {
            // System used monotonic clock instead of wall clock
            let t2 = manager.get_last_modified_time().unwrap();
            assert!(t2 > t1); // Monotonic time always increases
        }
        Err(e) => {
            // System detected clock skew and warned
            assert!(matches!(e, StateError::ClockSkew { .. }));
        }
    }
}
```

**Monotonic Clock Test**:
```rust
#[test]
fn test_monotonic_timestamps_resist_clock_skew() {
    let manager = StateManager::new(":memory:").unwrap();

    // Save state at T0
    let state_v1 = create_project_state();
    manager.save_project_state(&state_v1).unwrap();
    let mono_t1 = manager.get_monotonic_timestamp();

    // Simulate clock jump backward
    MockClock::jump_backward(Duration::from_hours(5));

    // Save state at T1 (wall clock is now T0 - 5 hours)
    let state_v2 = modify_project_state(&state_v1);
    manager.save_project_state(&state_v2).unwrap();
    let mono_t2 = manager.get_monotonic_timestamp();

    // Verify monotonic timestamps are strictly increasing
    assert!(mono_t2 > mono_t1);

    // Verify version ordering is correct
    let versions = manager.list_state_versions().unwrap();
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[1].version, 2);
}
```

**Expected Behavior**:
1. ✅ Uses monotonic clock (Rust's `Instant`) for duration measurements
2. ✅ Uses wall clock (Rust's `SystemTime`) only for display timestamps
3. ✅ Detects clock jumps > 1 hour and logs warning
4. ✅ Does NOT corrupt version ordering due to clock skew
5. ✅ Export timestamps use monotonic-based sequence numbers

**Pass Criteria**:
- Monotonic clock unaffected by wall clock changes
- Version ordering preserved despite clock jumps
- Clock skew > 1 hour logged with severity: WARN

---

## 9.28.6 Infrastructure Test Execution

### Execution Command
```bash
# Run all infrastructure failure tests
cargo test --test infrastructure_failures -- --test-threads=1

# Run specific infrastructure test
cargo test test_database_corruption_recovery
cargo test test_disk_full_handling
cargo test test_memory_exhaustion_handling
cargo test test_network_partition_handling
cargo test test_clock_skew_handling
```

### Test Environment Requirements

| Test | Special Setup | Teardown |
|------|---------------|----------|
| TEST-INFRA-001 | Temporary database file | Delete temp file |
| TEST-INFRA-002 | Loop device (Linux) or quota (Windows) | Unmount/remove quota |
| TEST-INFRA-003 | rlimit (Unix) or job object (Windows) | Restore limits |
| TEST-INFRA-004 | Mock server or network namespace | Stop mock/restore DNS |
| TEST-INFRA-005 | Mock clock library | Restore system time |

### CI/CD Considerations

**Linux CI**:
```yaml
- name: Run Infrastructure Tests
  run: |
    # Requires privileged mode for loop devices
    docker run --privileged \
      -v $PWD:/workspace \
      rust:latest \
      cargo test --test infrastructure_failures
```

**Windows CI**:
```yaml
- name: Run Infrastructure Tests
  run: |
    # Requires admin for quota management
    cargo test --test infrastructure_failures
  # Run in elevated PowerShell context
```

---

## 9.28.7 Traceability Matrix

| Test ID | Failure Type | IM Codes | Related Components |
|---------|--------------|----------|-------------------|
| TEST-INFRA-001 | Database Corruption | IM-5001, IM-5002, IM-5010 | StateManager, SQLite persistence |
| TEST-INFRA-002 | Disk Full | IM-5001, IM-5002, IM-6001, IM-6002 | StateManager, Export generation |
| TEST-INFRA-003 | Memory Exhaustion | IM-2001, IM-2010, IM-3001, IM-5001 | AgentOrchestrator, LLM client |
| TEST-INFRA-004 | Network Partition | IM-3001, IM-3010, IM-3011, IM-4001 | LLM client, Retry logic |
| TEST-INFRA-005 | Clock Skew | IM-5001, IM-5002, IM-5020, IM-6001 | StateManager, Timestamps |

---

## 9.28.8 Success Criteria Summary

All infrastructure tests MUST:
1. ✅ Detect failure condition within specified time (< 1s for critical failures)
2. ✅ Return typed error (not panic) with diagnostic information
3. ✅ Maintain data consistency (no corruption from failed operations)
4. ✅ Log actionable error messages with recovery instructions
5. ✅ Execute in < 30 seconds per test (some require privileged setup)

**Aggregate Pass Criteria**: 5/5 infrastructure tests pass (100%)

---

**Section 9.28 Complete - Ready for Integration into L5-TESTPLAN**
