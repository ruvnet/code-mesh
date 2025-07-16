# Contributing to Code Mesh

Thank you for your interest in contributing to Code Mesh! This document provides guidelines and instructions for contributing to the project.

## 🚀 Getting Started

### Prerequisites
- Rust 1.75+ (install via [rustup](https://rustup.rs))
- Node.js 18+ (for NPM package development)
- Git

### Setting Up Development Environment

1. Fork and clone the repository:
```bash
git clone https://github.com/yourusername/code-mesh.git
cd code-mesh
```

2. Install Rust dependencies:
```bash
cargo build --workspace
```

3. Run tests:
```bash
cargo test --workspace
```

## 🏗️ Project Structure

- `crates/code-mesh-core/` - Core functionality and abstractions
- `crates/code-mesh-cli/` - Command-line interface
- `crates/code-mesh-wasm/` - WebAssembly bindings
- `npm/` - NPM package for distribution
- `docs/` - Documentation

## 📝 Development Guidelines

### Code Style
- Follow Rust's official style guide
- Use `cargo fmt` before committing
- Run `cargo clippy` and fix any warnings
- Write descriptive commit messages

### Testing
- Write unit tests for new functionality
- Ensure all tests pass before submitting PR
- Add integration tests for new features
- Test both native and WASM targets when applicable

### Documentation
- Add rustdoc comments for public APIs
- Update README.md for user-facing changes
- Document breaking changes in CHANGELOG.md

## 🔄 Pull Request Process

1. Create a feature branch:
```bash
git checkout -b feature/your-feature-name
```

2. Make your changes and commit:
```bash
git add .
git commit -m "feat: add new feature"
```

3. Push to your fork:
```bash
git push origin feature/your-feature-name
```

4. Open a Pull Request with:
   - Clear description of changes
   - Link to related issues
   - Test results
   - Screenshots (if UI changes)

## 🐛 Reporting Issues

When reporting issues, please include:
- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected vs actual behavior
- Error messages or logs

## 💡 Feature Requests

We welcome feature requests! Please:
- Check existing issues first
- Provide detailed use cases
- Explain why the feature would be valuable
- Consider implementation complexity

## 📜 Code of Conduct

Please note that this project is released with a [Contributor Code of Conduct](CODE_OF_CONDUCT.md). By participating in this project you agree to abide by its terms.

## 📄 License

By contributing to Code Mesh, you agree that your contributions will be licensed under the MIT License.

## 🙏 Thank You!

Your contributions make Code Mesh better for everyone. We appreciate your time and effort!