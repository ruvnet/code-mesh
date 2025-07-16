# Code Mesh Documentation

Welcome to the comprehensive documentation for **Code Mesh**, a high-performance AI coding assistant built with Rust and WebAssembly.

## What is Code Mesh?

Code Mesh is a next-generation AI coding assistant that combines the power of multiple language models with a sophisticated multi-agent orchestration system. Inspired by [OpenCode](https://github.com/sst/opencode), Code Mesh is built from the ground up in Rust to provide:

- **🦀 Native Performance**: Built in Rust for maximum speed and efficiency
- **🌐 WebAssembly Support**: Run in browsers or Node.js via NPX
- **🧩 Modular Architecture**: Three specialized crates with clear separation of concerns
- **🤖 Multi-LLM Support**: Unified interface for Anthropic, OpenAI, GitHub Copilot, Mistral, and more
- **🔧 Comprehensive Tool System**: File operations, code search, web tools, and custom extensions
- **🔐 Secure Authentication**: OAuth and API key support with encrypted storage
- **📦 Easy Distribution**: Install via `npx code-mesh` - no manual setup required

## Quick Links

- **[Quick Start Guide](getting-started/quick-start.md)** - Get up and running in minutes
- **[CLI Reference](user-guide/cli-reference.md)** - Complete command reference
- **[API Documentation](development/api.md)** - Rust API documentation
- **[Examples](examples/workflows.md)** - Practical usage examples
- **[Contributing](development/contributing.md)** - How to contribute to the project

## Key Features

### Multi-Agent Orchestration

Code Mesh employs multiple specialized AI agents that work together to solve complex coding tasks:

- **Planner Agent**: Breaks down complex requests into manageable tasks
- **Coder Agent**: Implements code changes with context awareness
- **Tester Agent**: Runs tests and validates implementations
- **Reviewer Agent**: Reviews code quality and suggests improvements

### Cross-Platform Compatibility

Thanks to Rust and WebAssembly, Code Mesh runs everywhere:

- **Native CLI**: Full-featured command-line interface
- **Browser**: Run directly in web applications
- **Node.js**: Use via NPX without installation
- **IDE Integration**: Extensible for VS Code, IntelliJ, and more

### Intelligent Context Management

Code Mesh maintains intelligent context across sessions:

- **Project Awareness**: Understands your entire codebase
- **Session Memory**: Remembers previous conversations
- **File Context**: Tracks changes and dependencies
- **Agent Memory**: Agents learn from past interactions

## Getting Started

Ready to dive in? Start with our [Quick Start Guide](getting-started/quick-start.md) to get Code Mesh running in your project within minutes.

For a deeper understanding of the architecture and design decisions, explore our [Architecture Overview](development/architecture.md) and [Architecture Decision Records](reference/adrs/README.md).

## Community & Support

- **[GitHub Repository](https://github.com/yourusername/code-mesh)** - Source code and issues
- **[Discussions](https://github.com/yourusername/code-mesh/discussions)** - Community discussions
- **[Contributing Guide](development/contributing.md)** - How to contribute

## License

Code Mesh is open source software released under the [MIT License](appendices/license.md).