# EPIC: Port OpenCode to Code Mesh - Modular Rust + WASM System

## 🎯 Overview
Port the OpenCode TypeScript codebase to a modular Rust crate system called "Code Mesh" with WebAssembly support and NPX distribution. This will create a high-performance, portable AI coding assistant that can run natively or in the browser.

## 📋 Acceptance Criteria
- [ ] Complete migration of core OpenCode functionality to Rust
- [ ] Three modular crates: `code-mesh-core`, `code-mesh-cli`, `code-mesh-wasm`
- [ ] WASM build with wasm-pack for browser/NPX usage
- [ ] Feature parity with original OpenCode TypeScript implementation
- [ ] NPM package published as `code-mesh` for `npx code-mesh` usage
- [ ] Comprehensive test suite with >80% coverage
- [ ] Documentation and migration guide

## 🏗️ Architecture Design

### Crate Structure
```
code-mesh/
├── crates/
│   ├── code-mesh-core/     # Core functionality
│   │   ├── src/
│   │   │   ├── agent/      # Agent orchestration
│   │   │   ├── llm/        # LLM trait & implementations
│   │   │   ├── planner/    # Task planning
│   │   │   ├── session/    # Session management
│   │   │   ├── memory/     # Memory & storage
│   │   │   └── tool/       # Tool trait & implementations
│   │   └── Cargo.toml
│   ├── code-mesh-cli/      # Native CLI
│   │   ├── src/
│   │   │   ├── cmd/        # Commands (init, run, auth, status)
│   │   │   ├── tui/        # Terminal UI
│   │   │   └── main.rs
│   │   └── Cargo.toml
│   └── code-mesh-wasm/     # WASM bindings
│       ├── src/
│       │   ├── bindings/   # wasm-bindgen interfaces
│       │   └── lib.rs
│       └── Cargo.toml
├── npm/                    # NPM package
│   ├── package.json
│   └── bin/
└── Cargo.toml             # Workspace
```

## 📊 Sub-Issues Breakdown

### Phase 1: Foundation & Core Architecture

#### 1.1 Project Setup & Infrastructure
- [ ] Initialize Rust workspace with three crates
- [ ] Set up CI/CD pipeline (GitHub Actions)
- [ ] Configure wasm-pack build system
- [ ] Set up feature flags for wasm32/native targets
- [ ] Create NPM package structure

#### 1.2 Core Trait Definitions
- [ ] Define `Provider` trait for LLM providers
- [ ] Define `Model` trait with capabilities
- [ ] Define `Tool` trait for tool system
- [ ] Define `Auth` trait for authentication
- [ ] Define `Storage` trait for persistence

#### 1.3 LLM Provider System
- [ ] Implement provider registry
- [ ] Port Anthropic provider (OAuth + API key)
- [ ] Port OpenAI provider
- [ ] Port GitHub Copilot provider
- [ ] Port Mistral provider
- [ ] Implement provider configuration loading

### Phase 2: Core Module Migration

#### 2.1 Session Management
- [ ] Port `Session` module to Rust
- [ ] Implement message handling (V2 format)
- [ ] Port system prompt generation
- [ ] Implement mode system (chat, plan, etc.)
- [ ] Add session persistence

#### 2.2 Tool System
- [ ] Port `BashTool` with process execution
- [ ] Port `ReadTool` and `WriteTool`
- [ ] Port `EditTool` and `MultiEditTool`
- [ ] Port `GrepTool` using ripgrep
- [ ] Port `GlobTool` for file searching
- [ ] Port `TodoTool` for task management
- [ ] Port `WebFetchTool` and `WebSearchTool`
- [ ] Implement tool permission system

#### 2.3 Authentication System
- [ ] Port auth persistence (`auth.json`)
- [ ] Implement OAuth flow (PKCE)
- [ ] Port Anthropic auth module
- [ ] Port GitHub Copilot device flow
- [ ] Implement token refresh logic
- [ ] Add secure credential storage

### Phase 3: CLI Development

#### 3.1 Command Implementation
- [ ] Port `run` command with session continuation
- [ ] Port `auth` command (login/logout/list)
- [ ] Port `serve` command for API server
- [ ] Port `models` command
- [ ] Implement `init` command for project setup
- [ ] Add `status` command for health checks

#### 3.2 Terminal UI
- [ ] Implement TUI using ratatui/crossterm
- [ ] Port chat interface
- [ ] Port diff viewer
- [ ] Port file viewer
- [ ] Implement progress indicators
- [ ] Add theme support

### Phase 4: Advanced Features

#### 4.1 Diff Pipeline & Snapshot
- [ ] Port diff generation system
- [ ] Implement snapshot model
- [ ] Add file change tracking
- [ ] Implement rollback capability

#### 4.2 Task Planner
- [ ] Port Claude Code-style task planning
- [ ] Implement Jules-style reasoning
- [ ] Add task decomposition
- [ ] Implement progress tracking

#### 4.3 Memory & Storage
- [ ] Port storage abstraction
- [ ] Implement file-based persistence
- [ ] Add memory namespacing
- [ ] Implement garbage collection

#### 4.4 MCP (Model Context Protocol)
- [ ] Port MCP client implementation
- [ ] Add MCP server discovery
- [ ] Implement tool bridging
- [ ] Add resource management

### Phase 5: WASM & Distribution

#### 5.1 WASM Bindings
- [ ] Create wasm-bindgen interfaces
- [ ] Implement JavaScript API
- [ ] Add browser compatibility layer
- [ ] Optimize WASM bundle size

#### 5.2 NPM Package
- [ ] Create NPX runner script
- [ ] Implement WASM loader
- [ ] Add platform detection
- [ ] Create fallback mechanisms

#### 5.3 Performance Optimization
- [ ] Profile and optimize hot paths
- [ ] Implement caching strategies
- [ ] Add connection pooling
- [ ] Optimize WASM performance

### Phase 6: Testing & Documentation

#### 6.1 Test Suite
- [ ] Unit tests for all modules
- [ ] Integration tests for workflows
- [ ] WASM-specific tests
- [ ] Performance benchmarks
- [ ] Compatibility testing

#### 6.2 Documentation
- [ ] API documentation (rustdoc)
- [ ] User guide
- [ ] Migration guide from OpenCode
- [ ] WASM usage examples
- [ ] Contributing guidelines

## 🚀 Implementation Strategy

### Development Approach
1. **Incremental Migration**: Start with core modules, gradually port features
2. **Parallel Development**: 10-agent swarm for concurrent development
3. **Test-Driven**: Write tests alongside implementation
4. **Performance-First**: Profile and optimize throughout

### Technology Stack
- **Language**: Rust (stable)
- **WASM**: wasm-pack, wasm-bindgen
- **Async Runtime**: tokio (native), wasm-bindgen-futures (WASM)
- **HTTP**: reqwest (native), web-sys (WASM)
- **TUI**: ratatui, crossterm
- **Serialization**: serde, serde_json
- **Error Handling**: thiserror, anyhow

### Feature Flags
```toml
[features]
default = ["native"]
native = ["tokio", "reqwest", "crossterm"]
wasm = ["wasm-bindgen", "web-sys", "js-sys"]
```

## 📈 Success Metrics
- Performance: 2x faster than TypeScript version
- Bundle Size: <5MB WASM bundle
- Memory Usage: 50% reduction
- Test Coverage: >80%
- Documentation: 100% public API documented

## 🔗 References
- Original Repository: https://github.com/sst/opencode
- OpenCode Documentation: [internal docs]
- Rust WASM Guide: https://rustwasm.github.io/book/
- wasm-pack: https://rustwasm.github.io/wasm-pack/

## 👥 Team Allocation (Hive Mind Swarm)
1. **Core Architect**: Design patterns, trait definitions
2. **LLM Specialist**: Provider implementations
3. **Tool Developer 1**: File/process tools
4. **Tool Developer 2**: Web/search tools
5. **CLI Developer**: Command implementation
6. **TUI Developer**: Terminal interface
7. **WASM Engineer**: Bindings and optimization
8. **Test Engineer**: Test suite development
9. **Performance Engineer**: Optimization and profiling
10. **Documentation Lead**: Guides and API docs

---

**Note**: This EPIC will be broken down into individual GitHub issues for each sub-task. Each issue will include detailed requirements, implementation notes, and acceptance criteria.