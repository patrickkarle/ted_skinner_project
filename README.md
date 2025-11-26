# Ted Skinner Project - FullIntel Agent

A sophisticated AI agent system built with Rust and Tauri, implementing advanced workflow orchestration, rate limiting, and circuit breaker patterns for reliable LLM integration.

## Project Overview

This project implements a comprehensive agent-based system with:
- **Multi-phase workflow execution** from YAML manifests
- **Advanced protective mechanisms** (rate limiting, circuit breakers)
- **Multi-LLM provider support** (Anthropic, Google, DeepSeek)
- **Robust error handling** and graceful degradation
- **Comprehensive test suite** (60 tests across 3 batteries)

## Architecture

```
fullintel-agent/
├── src/
│   ├── lib.rs          # Library exports
│   ├── main.rs         # Application entry point
│   ├── agent.rs        # Agent and workflow orchestration (IM-2000-2019)
│   ├── manifest.rs     # YAML manifest parsing (IM-2020-2029)
│   └── llm.rs          # LLM client, rate limiting, circuit breakers (IM-3000-3037)
└── tests/
    ├── battery1_unit_strategic.rs        # 30 unit tests
    ├── battery2_integration_strategic.rs # 20 integration tests
    └── battery3_system_strategic.rs      # 10 system tests
```

## Component Manifest

### Core Components (126 total)

**Agent Module (IM-2000-2019):** 20 components
- Agent, AgentState, PhaseStatus
- Workflow execution, phase management, context handling
- Error recovery and state transitions

**Manifest Module (IM-2020-2029):** 10 components
- Manifest, Phase, DataSchema, QualityGate
- YAML parsing, validation, phase configuration

**LLM Module (IM-3000-3037):** 38 components
- LLMClient, LLMRequest, LLMResponse
- RateLimiter (token bucket algorithm)
- CircuitBreaker (failure protection)
- Provider management (Anthropic, Google, DeepSeek)

**Integration Points (IP-1000-1009):** 10 documented
**Data Transformations (DT-1000-1009):** 10 documented
**Error Handling Patterns (EH-1000-1009):** 10 documented
**Test Infrastructure:** 28 test utilities

## Testing Strategy

### 3-Battery Test Suite (60 tests total)

**Battery 1: Unit Tests (30 tests)**
- Component isolation and validation
- Pure function testing
- State management verification

**Battery 2: Integration Tests (20 tests)**
- Component interaction validation
- Data flow between modules
- Error propagation and recovery

**Battery 3: System Tests (10 tests)**
- End-to-end workflow execution
- Performance under stress
- Multi-provider failover
- Rate limiting and circuit breaking

### Running Tests

```bash
# All tests
cargo test --all

# Individual batteries
cargo test --test battery1_unit_strategic
cargo test --test battery2_integration_strategic
cargo test --test battery3_system_strategic

# Specific test
cargo test --test battery1_unit_strategic test_agent_initialization

# With output
cargo test -- --nocapture
```

## Build & Development

```bash
# Build
cargo build --release

# Run
cargo run

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt
```

## CI/CD

GitHub Actions automatically runs:
- All 3 test batteries
- Clippy linting
- Rustfmt formatting

On push/PR to main, master, or develop branches.

## Dependencies

- **Tauri 2.9**: Desktop application framework
- **Tokio 1.48**: Async runtime
- **Reqwest 0.12**: HTTP client
- **Serde 1.0**: Serialization
- **Serde_yaml 0.9**: YAML parsing

## License

MIT (or your chosen license)

## Documentation

See `docs/se-cpm/test-plans/` for:
- Test specifications
- Implementation details
- Session handoff notes
- Windows runtime analysis

## Quality Metrics

- **Test Coverage:** 60/60 tests (100%)
- **Components Validated:** 126 components
- **Integration Points:** 10 documented
- **Error Handling:** Comprehensive across all layers
- **Performance:** Tested under sustained load (120 req/min)
