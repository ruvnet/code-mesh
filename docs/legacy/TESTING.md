# Testing Guide for Code-Mesh

This document provides comprehensive information about the testing infrastructure and best practices for the code-mesh project.

## Overview

The code-mesh project employs a multi-layered testing strategy including:

- **Unit Tests**: Testing individual modules and functions
- **Integration Tests**: Testing component interactions and workflows
- **Property-based Tests**: Testing with generated inputs using proptest
- **Performance Tests**: Benchmarking and regression testing
- **WASM Tests**: Browser and Node.js environment testing
- **Mutation Tests**: Validating test quality
- **End-to-End Tests**: Complete workflow validation

## Test Structure

```
crates/
├── code-mesh-core/
│   ├── tests/
│   │   ├── common/           # Shared test utilities
│   │   │   ├── mod.rs       # Common exports
│   │   │   ├── mocks.rs     # Mock implementations
│   │   │   └── fixtures.rs  # Test data and fixtures
│   │   ├── unit/            # Unit tests
│   │   │   ├── auth_tests.rs
│   │   │   ├── llm_tests.rs
│   │   │   ├── session_tests.rs
│   │   │   └── tool_tests.rs
│   │   └── integration/     # Integration tests
│   │       └── end_to_end_tests.rs
│   └── benches/             # Performance benchmarks
│       └── performance_benchmarks.rs
├── code-mesh-cli/
│   └── tests/
│       ├── common/          # CLI test utilities
│       └── integration/     # CLI integration tests
└── code-mesh-wasm/
    └── tests/               # WASM-specific tests
        └── wasm_tests.rs
```

## Running Tests

### Quick Test Commands

```bash
# Run all tests
make test

# Run specific test types
make test-unit              # Unit tests only
make test-integration       # Integration tests only
make test-wasm             # WASM tests only
make test-property         # Property-based tests

# Generate coverage report
make coverage

# Run performance benchmarks
make benchmark
```

### Detailed Test Commands

```bash
# Run tests with specific features
cargo test --features "anthropic,openai"

# Run tests with verbose output
cargo test -- --nocapture

# Run a specific test
cargo test test_session_creation

# Run tests matching a pattern
cargo test auth_

# Run tests with thread control
cargo test -- --test-threads=1
```

## Test Categories

### 1. Unit Tests

Located in `tests/unit/`, these test individual modules:

- **Authentication Tests** (`auth_tests.rs`):
  - Token storage and retrieval
  - Provider configuration
  - Security validation
  - File permissions

- **LLM Tests** (`llm_tests.rs`):
  - Chat completion generation
  - Streaming responses
  - Model configuration
  - Error handling

- **Session Tests** (`session_tests.rs`):
  - Session lifecycle
  - Message management
  - Conversation context
  - Serialization

- **Tool Tests** (`tool_tests.rs`):
  - Tool execution
  - Parameter validation
  - Result formatting
  - Concurrent execution

### 2. Integration Tests

Located in `tests/integration/`, these test component interactions:

- **End-to-End Workflows**:
  - Complete user journeys
  - Multi-component interactions
  - Error recovery scenarios
  - Performance under load

- **API Integration**:
  - Real LLM provider testing (when API keys available)
  - HTTP client behavior
  - Rate limiting handling
  - Network error scenarios

### 3. Property-Based Tests

Using `proptest` for generating test inputs:

```rust
proptest! {
    #[test]
    fn test_session_properties(
        session_id in "[a-zA-Z0-9-]{1,100}",
        user_id in "[a-zA-Z0-9_]{1,50}"
    ) {
        let session = Session::new(session_id.clone(), user_id.clone());
        prop_assert_eq!(session.id, session_id);
        prop_assert_eq!(session.user_id, user_id);
    }
}
```

### 4. WASM Tests

WASM-specific tests run in both Node.js and browser environments:

```bash
# Test in Node.js
wasm-pack test crates/code-mesh-wasm --node

# Test in browser (headless)
wasm-pack test crates/code-mesh-wasm --headless --chrome
```

### 5. Performance Tests

Benchmarks using `criterion`:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench session_operations

# Compare with baseline
cargo bench -- --save-baseline main
```

## Test Utilities

### Mock Implementations

The `tests/common/mocks.rs` file provides:

- `MockLLMProvider`: Simulates LLM responses
- `MockStorage`: In-memory storage implementation
- `MockAuthStorage`: Authentication storage mock
- `CountingTool`: Tool execution tracking
- `InMemoryStorage`: Fast storage for testing

### Test Fixtures

The `tests/common/fixtures.rs` file includes:

- Sample chat conversations
- File content examples
- Configuration templates
- Error scenarios
- Performance test data

### Test Environment Setup

```rust
use crate::common::*;

#[tokio::test]
async fn test_example() {
    let mut env = TestEnvironment::new();
    env.setup_default_auth();
    env.setup_default_storage();
    env.setup_default_llm();
    
    // Your test code here
}
```

## Code Coverage

### Generating Coverage Reports

```bash
# Generate HTML coverage report
make coverage

# View coverage report
open coverage/tarpaulin-report.html
```

### Coverage Requirements

- **Minimum Coverage**: 80%
- **Critical Modules**: 90% (auth, storage, session)
- **Integration Tests**: 70%
- **WASM Tests**: 75%

### Coverage Configuration

Coverage settings are in `tarpaulin.toml`:

```toml
[report]
out = ["Html", "Xml", "Json"]
output-dir = "coverage"
fail-under = 80
exclude-files = [
    "tests/*",
    "benches/*",
    "examples/*"
]
```

## Continuous Integration

### GitHub Actions Workflow

The `.github/workflows/tests.yml` includes:

- **Multi-platform Testing**: Ubuntu, Windows, macOS
- **Multi-version Testing**: Stable, Beta, Nightly Rust
- **WASM Testing**: Node.js and browser environments
- **Coverage Reporting**: Automated coverage uploads
- **Performance Monitoring**: Benchmark regression detection
- **Security Scanning**: Dependency audit and license checks

### CI Test Jobs

1. **test**: Core test suite across platforms
2. **test-wasm**: WASM-specific testing
3. **coverage**: Code coverage generation
4. **benchmark**: Performance regression testing
5. **property-tests**: Extended property-based testing
6. **mutation-tests**: Test quality validation
7. **security**: Security audit and dependency checks

## Best Practices

### Writing Tests

1. **Test Naming**: Use descriptive names
   ```rust
   #[test]
   fn test_session_adds_message_updates_timestamp() { }
   ```

2. **Test Structure**: Follow AAA pattern (Arrange, Act, Assert)
   ```rust
   #[test]
   fn test_example() {
       // Arrange
       let session = Session::new("test".to_string(), "user".to_string());
       
       // Act
       session.add_message(message);
       
       // Assert
       assert_eq!(session.message_count(), 1);
   }
   ```

3. **Error Testing**: Test both success and failure cases
   ```rust
   #[test]
   fn test_invalid_input_returns_error() {
       let result = function_under_test(invalid_input);
       assert!(result.is_err());
   }
   ```

4. **Async Testing**: Use `#[tokio::test]` for async tests
   ```rust
   #[tokio::test]
   async fn test_async_operation() {
       let result = async_function().await;
       assert!(result.is_ok());
   }
   ```

### Mock Usage

1. **Use Mocks for External Dependencies**:
   ```rust
   let mut mock_llm = MockLLMProvider::new();
   mock_llm.expect_chat_completion()
           .returning(|_| Ok(mock_response()));
   ```

2. **Configure Mock Expectations**:
   ```rust
   mock_storage.expect_save()
               .with(eq("key"), eq(value))
               .times(1)
               .returning(|_, _| Ok(()));
   ```

### Property-Based Testing

1. **Use for Complex Logic**:
   ```rust
   proptest! {
       #[test]
       fn test_serialization_roundtrip(data in any::<SessionData>()) {
           let serialized = serialize(&data).unwrap();
           let deserialized = deserialize(&serialized).unwrap();
           prop_assert_eq!(data, deserialized);
       }
   }
   ```

2. **Generate Realistic Data**:
   ```rust
   prop_compose! {
       fn arb_session_id()(id in "[a-zA-Z0-9-]{1,50}") -> String {
           id
       }
   }
   ```

### Performance Testing

1. **Measure Critical Paths**:
   ```rust
   #[bench]
   fn bench_session_creation(b: &mut Bencher) {
       b.iter(|| {
           Session::new(
               black_box("test-session".to_string()),
               black_box("user".to_string())
           )
       });
   }
   ```

2. **Set Performance Baselines**:
   ```bash
   cargo bench -- --save-baseline main
   ```

## Troubleshooting

### Common Issues

1. **Test Timeout**: Increase timeout in `tarpaulin.toml`
2. **WASM Test Failures**: Ensure wasm-pack is installed
3. **Coverage Low**: Add tests for uncovered code paths
4. **Flaky Tests**: Use proper async handling and timeouts

### Debugging Tests

```bash
# Run with debug output
RUST_LOG=debug cargo test test_name -- --nocapture

# Run single threaded
cargo test -- --test-threads=1

# Show test output
cargo test -- --show-output
```

### Performance Issues

```bash
# Profile test execution
cargo test --release

# Memory profiling
valgrind --tool=memcheck cargo test

# CPU profiling
perf record cargo test
```

## Contributing Tests

When contributing to the project:

1. **Add Tests for New Features**: Every new feature should include comprehensive tests
2. **Maintain Coverage**: Don't reduce overall coverage percentage
3. **Update Documentation**: Update this guide for new testing patterns
4. **Run Full Test Suite**: Ensure all tests pass before submitting PR

### Test Review Checklist

- [ ] Tests cover both success and failure cases
- [ ] Property-based tests for complex logic
- [ ] Integration tests for new workflows
- [ ] Performance tests for critical paths
- [ ] Documentation updated
- [ ] CI pipeline passes

## Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [Proptest Documentation](https://docs.rs/proptest/)
- [Criterion.rs User Guide](https://bheisler.github.io/criterion.rs/book/)
- [wasm-pack Testing](https://rustwasm.github.io/wasm-pack/book/commands/test.html)
- [Tarpaulin Documentation](https://github.com/xd009642/tarpaulin)