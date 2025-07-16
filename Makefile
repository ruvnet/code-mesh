# Makefile for code-mesh project

.PHONY: all build test test-unit test-integration test-wasm coverage benchmark clean format lint check help

# Default target
all: format lint test build

# Build the project
build:
	@echo "Building all targets..."
	cargo build --all-targets --all-features
	@echo "Build completed successfully"

# Build release version
build-release:
	@echo "Building release version..."
	cargo build --release --all-targets --all-features
	@echo "Release build completed successfully"

# Run all tests
test: test-unit test-integration test-wasm
	@echo "All tests completed"

# Run unit tests
test-unit:
	@echo "Running unit tests..."
	cargo test --all-features --lib
	@echo "Unit tests completed"

# Run integration tests
test-integration:
	@echo "Running integration tests..."
	cargo test --all-features --test '*'
	@echo "Integration tests completed"

# Run WASM tests
test-wasm:
	@echo "Running WASM tests..."
	cd crates/code-mesh-wasm && wasm-pack test --node
	@echo "WASM tests completed"

# Run property-based tests with high iteration count
test-property:
	@echo "Running property-based tests..."
	PROPTEST_CASES=10000 cargo test --all-features prop_
	@echo "Property-based tests completed"

# Run tests with coverage
coverage:
	@echo "Generating coverage report..."
	cargo tarpaulin --all-features --workspace --timeout 120 \
		--exclude-files "tests/*" "benches/*" "examples/*" \
		--out html --out xml --output-dir coverage/
	@echo "Coverage report generated in coverage/ directory"

# Run performance benchmarks
benchmark:
	@echo "Running performance benchmarks..."
	cargo bench --all-features
	@echo "Benchmarks completed"

# Run mutation tests
mutation-test:
	@echo "Running mutation tests..."
	cargo install cargo-mutants --locked || true
	cargo mutants --timeout 60 --jobs $$(nproc)
	@echo "Mutation tests completed"

# Format code
format:
	@echo "Formatting code..."
	cargo fmt --all
	@echo "Code formatting completed"

# Check formatting
format-check:
	@echo "Checking code formatting..."
	cargo fmt --all -- --check

# Run lints
lint:
	@echo "Running lints..."
	cargo clippy --all-targets --all-features -- -D warnings
	@echo "Lints completed"

# Run security audit
audit:
	@echo "Running security audit..."
	cargo audit
	@echo "Security audit completed"

# Check for unused dependencies
unused-deps:
	@echo "Checking for unused dependencies..."
	cargo install cargo-udeps --locked || true
	cargo +nightly udeps --all-targets
	@echo "Unused dependencies check completed"

# Check license compliance
license-check:
	@echo "Checking license compliance..."
	cargo install cargo-deny --locked || true
	cargo deny check
	@echo "License check completed"

# Run all checks (format, lint, test)
check: format-check lint test
	@echo "All checks passed"

# Build WASM package
build-wasm:
	@echo "Building WASM package..."
	cd crates/code-mesh-wasm && wasm-pack build --target web
	@echo "WASM package built successfully"

# Build CLI binary
build-cli:
	@echo "Building CLI binary..."
	cargo build --release --bin code-mesh
	@echo "CLI binary built at target/release/code-mesh"

# Install CLI locally
install-cli:
	@echo "Installing CLI locally..."
	cargo install --path crates/code-mesh-cli
	@echo "CLI installed successfully"

# Run CLI tests
test-cli:
	@echo "Running CLI tests..."
	cd crates/code-mesh-cli && cargo test --all-features
	@echo "CLI tests completed"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean
	rm -rf coverage/
	rm -rf target/criterion/
	rm -rf mutants.out/
	@echo "Clean completed"

# Update dependencies
update:
	@echo "Updating dependencies..."
	cargo update
	@echo "Dependencies updated"

# Generate documentation
docs:
	@echo "Generating documentation..."
	cargo doc --all-features --workspace --no-deps
	@echo "Documentation generated"

# Open documentation in browser
docs-open: docs
	@echo "Opening documentation..."
	cargo doc --all-features --workspace --no-deps --open

# Run stress tests
stress-test:
	@echo "Running stress tests..."
	cargo test --all-features stress_ --release
	@echo "Stress tests completed"

# Run performance regression tests
regression-test:
	@echo "Running regression tests..."
	cargo bench --all-features -- --save-baseline main
	@echo "Regression tests completed"

# Setup development environment
setup:
	@echo "Setting up development environment..."
	rustup component add rustfmt clippy
	rustup target add wasm32-unknown-unknown
	cargo install wasm-pack cargo-tarpaulin cargo-audit cargo-deny
	@echo "Development environment setup completed"

# Prepare for release
pre-release: format lint test coverage benchmark
	@echo "Pre-release checks completed successfully"

# Run CI pipeline locally
ci: format-check lint test coverage
	@echo "CI pipeline completed successfully"

# Generate test report
test-report:
	@echo "Generating test report..."
	cargo test --all-features -- --format json > test-results.json
	@echo "Test report generated: test-results.json"

# Profile memory usage
profile-memory:
	@echo "Profiling memory usage..."
	cargo build --release --all-features
	valgrind --tool=massif --stacks=yes target/release/code-mesh --help
	@echo "Memory profiling completed"

# Profile CPU usage
profile-cpu:
	@echo "Profiling CPU usage..."
	cargo build --release --all-features
	perf record -g target/release/code-mesh --help
	perf report
	@echo "CPU profiling completed"

# Check binary size
check-size:
	@echo "Checking binary sizes..."
	cargo build --release
	ls -lh target/release/code-mesh
	strip target/release/code-mesh
	ls -lh target/release/code-mesh
	@echo "Binary size check completed"

# Validate Cargo.toml files
validate-cargo:
	@echo "Validating Cargo.toml files..."
	find . -name "Cargo.toml" -exec cargo verify-project --manifest-path {} \;
	@echo "Cargo.toml validation completed"

# Help target
help:
	@echo "Available targets:"
	@echo "  all              - Run format, lint, test, and build"
	@echo "  build            - Build all targets"
	@echo "  build-release    - Build release version"
	@echo "  test             - Run all tests"
	@echo "  test-unit        - Run unit tests"
	@echo "  test-integration - Run integration tests"
	@echo "  test-wasm        - Run WASM tests"
	@echo "  test-property    - Run property-based tests"
	@echo "  coverage         - Generate test coverage report"
	@echo "  benchmark        - Run performance benchmarks"
	@echo "  mutation-test    - Run mutation tests"
	@echo "  format           - Format all code"
	@echo "  format-check     - Check code formatting"
	@echo "  lint             - Run clippy lints"
	@echo "  audit            - Run security audit"
	@echo "  check            - Run all checks"
	@echo "  clean            - Clean build artifacts"
	@echo "  docs             - Generate documentation"
	@echo "  setup            - Setup development environment"
	@echo "  ci               - Run CI pipeline locally"
	@echo "  help             - Show this help message"