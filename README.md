# Code Mesh

A high-performance AI coding assistant built with Rust and WebAssembly, inspired by [OpenCode](https://github.com/sst/opencode).

## 🚀 Features

- **🦀 Native Performance**: Built in Rust for maximum speed and efficiency
- **🌐 WebAssembly Support**: Run in browsers or Node.js via NPX
- **🧩 Modular Architecture**: Three specialized crates with clear separation of concerns
- **🤖 Multi-LLM Support**: Unified interface for Anthropic, OpenAI, GitHub Copilot, Mistral, and more
- **🔧 Comprehensive Tool System**: File operations, code search, web tools, and custom extensions
- **🔐 Secure Authentication**: OAuth and API key support with encrypted storage
- **📦 Easy Distribution**: Install via `npx code-mesh` - no manual setup required

## 📦 Quick Start

### Via NPX (Recommended)
```bash
npx code-mesh run "Help me implement a REST API"
```

### Via Cargo
```bash
cargo install code-mesh-cli
code-mesh run "Optimize this function"
```

### From Source
```bash
git clone https://github.com/yourusername/code-mesh.git
cd code-mesh
cargo build --release
./target/release/code-mesh --help
```

## 🎯 Usage

### Basic Commands

```bash
# Interactive mode (default)
code-mesh

# Run with a specific prompt
code-mesh run "Implement binary search in Rust"

# Continue previous session
code-mesh run --continue "Add error handling"

# Use a specific model
code-mesh run --model anthropic/claude-3-opus "Review this code"
```

### Authentication

```bash
# Interactive login
code-mesh auth login

# List configured providers
code-mesh auth list

# Logout from a provider
code-mesh auth logout anthropic
```

### Other Commands

```bash
# Initialize a new project
code-mesh init

# Check system status
code-mesh status --detailed

# Start API server
code-mesh serve --port 3000

# List available models
code-mesh models --provider anthropic
```

## 🏗️ Architecture

Code Mesh uses a modular architecture with three core crates:

### `code-mesh-core`
The heart of Code Mesh, containing:
- **LLM Abstractions**: Provider and Model traits for unified LLM access
- **Tool System**: Extensible framework for AI tools
- **Session Management**: Conversation state and history
- **Authentication**: Secure credential management
- **Agent Orchestration**: Multi-agent coordination (coming soon)

### `code-mesh-cli`
Native command-line interface featuring:
- **Command Parser**: Clap-based CLI with subcommands
- **Terminal UI**: Rich interactive interface with ratatui
- **Server Mode**: HTTP API for IDE integrations

### `code-mesh-wasm`
WebAssembly bindings enabling:
- **Browser Support**: Run Code Mesh in web applications
- **Node.js Integration**: Use via NPX without native binaries
- **JavaScript API**: Simple interface for web developers

## 🌐 Web/WASM Usage

```javascript
import { CodeMesh } from 'code-mesh';

// Create a new instance
const mesh = new CodeMesh();

// Add a user message
await mesh.addUserMessage("Write a fibonacci function");

// Generate response
const response = await mesh.generateResponse("openai/gpt-4");
console.log(response);

// Get conversation history
const messages = mesh.getMessages();
```

## 🛠️ Development

### Prerequisites
- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Node.js 18+ (for NPM package)
- wasm-pack (for WASM builds)

### Building

```bash
# Build all crates
cargo build --workspace

# Build release version
cargo build --release

# Build WASM module
wasm-pack build crates/code-mesh-wasm --target web

# Build NPM package
cd npm && npm run build
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run with verbose output
cargo test -- --nocapture

# Test WASM build
wasm-pack test crates/code-mesh-wasm --node
```

### Project Structure

```
code-mesh/
├── crates/
│   ├── code-mesh-core/     # Core library
│   ├── code-mesh-cli/      # CLI application
│   └── code-mesh-wasm/     # WASM bindings
├── npm/                    # NPM package
├── docs/                   # Documentation
└── examples/               # Usage examples
```

## 📚 Documentation

- [Architecture Overview](docs/architecture.md)
- [Provider Integration Guide](docs/providers.md)
- [Tool Development](docs/tools.md)
- [WASM Integration](docs/wasm.md)

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- Inspired by [OpenCode](https://github.com/sst/opencode) by SST
- Built with the Rust ecosystem
- Powered by state-of-the-art language models

## 🚧 Roadmap

- [x] Core trait system and abstractions
- [x] Basic CLI structure
- [x] WASM compilation support
- [ ] Provider implementations (Anthropic, OpenAI, etc.)
- [ ] Tool implementations (file ops, search, etc.)
- [ ] Terminal UI with ratatui
- [ ] Multi-agent orchestration
- [ ] Cloud deployment options
- [ ] IDE plugins (VS Code, IntelliJ)

See [EPIC-code-mesh-port.md](EPIC-code-mesh-port.md) for the detailed implementation plan.

---

**Note**: This project is under active development. APIs and features may change.