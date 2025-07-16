# Installation

Code Mesh can be installed and used in multiple ways, depending on your needs and preferences.

## NPX (Recommended for Trying)

The fastest way to try Code Mesh without installation:

```bash
npx code-mesh run "Help me debug this function"
```

### Pros
- ✅ No installation required
- ✅ Always uses the latest version
- ✅ Works on all platforms
- ✅ Automatic WASM download and caching

### Cons
- ❌ Slower startup time
- ❌ Requires internet connection for first run
- ❌ Limited by Node.js environment

## Native Binary (Recommended for Regular Use)

### Via Cargo

```bash
cargo install code-mesh-cli
```

This installs the `code-mesh` binary to your Cargo bin directory.

### Via Package Managers

#### macOS (Homebrew)
```bash
# Coming soon
brew install code-mesh
```

#### Linux (Snap)
```bash
# Coming soon
snap install code-mesh
```

#### Windows (Chocolatey)
```bash
# Coming soon
choco install code-mesh
```

### From Pre-built Binaries

Download the latest release from [GitHub Releases](https://github.com/yourusername/code-mesh/releases):

```bash
# Linux x86_64
wget https://github.com/yourusername/code-mesh/releases/latest/download/code-mesh-linux-x86_64.tar.gz
tar xzf code-mesh-linux-x86_64.tar.gz
sudo mv code-mesh /usr/local/bin/

# macOS
wget https://github.com/yourusername/code-mesh/releases/latest/download/code-mesh-macos-x86_64.tar.gz
tar xzf code-mesh-macos-x86_64.tar.gz
sudo mv code-mesh /usr/local/bin/

# Windows
# Download code-mesh-windows-x86_64.zip and extract
```

### Pros
- ✅ Fastest startup and execution
- ✅ Full feature set
- ✅ Works offline (after initial setup)
- ✅ Better resource usage

### Cons
- ❌ Platform-specific installation
- ❌ Manual updates

## From Source

### Prerequisites

- Rust 1.75+ ([install via rustup](https://rustup.rs))
- Node.js 18+ (for NPM package building)
- wasm-pack (for WASM builds)

### Install Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Install Node.js (use your preferred method)
# On macOS with Homebrew:
brew install node

# On Ubuntu/Debian:
sudo apt update && sudo apt install nodejs npm
```

### Build Process

```bash
# Clone the repository
git clone https://github.com/yourusername/code-mesh.git
cd code-mesh

# Build all crates
cargo build --workspace --release

# Build WASM module
wasm-pack build crates/code-mesh-wasm --target web

# Build NPM package
cd npm && npm run build
```

### Install Built Binary

```bash
# Install the CLI binary
cargo install --path crates/code-mesh-cli

# Or run directly
./target/release/code-mesh --help
```

### Pros
- ✅ Latest development features
- ✅ Customizable build options
- ✅ Full development environment

### Cons
- ❌ Requires build tools
- ❌ More complex setup
- ❌ Potential build issues

## Browser Integration

### Direct WASM Usage

```html
<!DOCTYPE html>
<html>
<head>
    <script type="module">
        import init, { CodeMesh } from './code-mesh.js';
        
        async function run() {
            await init();
            const mesh = new CodeMesh();
            
            await mesh.add_user_message("Hello, Code Mesh!");
            const response = await mesh.generate_response("anthropic/claude-3-opus");
            console.log(response);
        }
        
        run();
    </script>
</head>
<body>
    <h1>Code Mesh in Browser</h1>
</body>
</html>
```

### Via NPM Package

```bash
npm install code-mesh-wasm
```

```javascript
import { CodeMesh } from 'code-mesh-wasm';

const mesh = new CodeMesh();
await mesh.add_user_message("Implement a todo list component");
const response = await mesh.generate_response("openai/gpt-4");
```

## Verification

After installation, verify Code Mesh is working:

```bash
# Check version
code-mesh --version

# Test basic functionality
code-mesh status

# Run a simple query
code-mesh run "echo 'Hello, Code Mesh!'"
```

## Platform Support

| Platform | Native Binary | NPX | Browser |
|----------|---------------|-----|---------|
| Linux x86_64 | ✅ | ✅ | ✅ |
| Linux ARM64 | ✅ | ✅ | ✅ |
| macOS x86_64 | ✅ | ✅ | ✅ |
| macOS ARM64 (M1/M2) | ✅ | ✅ | ✅ |
| Windows x86_64 | ✅ | ✅ | ✅ |
| Windows ARM64 | 🔄 | ✅ | ✅ |

Legend: ✅ Supported, 🔄 Planned, ❌ Not supported

## System Requirements

### Minimum Requirements
- **RAM**: 512 MB available
- **Storage**: 100 MB for binary, 500 MB for cache
- **Network**: Internet connection for AI providers

### Recommended Requirements
- **RAM**: 2 GB available
- **Storage**: 1 GB for cache and sessions
- **CPU**: Multi-core for better performance

## Troubleshooting Installation

### Common Issues

#### `cargo install` fails
```bash
# Update Rust
rustup update

# Clear cargo cache
cargo clean

# Try again with verbose output
cargo install code-mesh-cli --verbose
```

#### NPX command not found
```bash
# Ensure Node.js is installed
node --version
npm --version

# Clear NPX cache
npx clear-npx-cache
```

#### Permission denied on Unix
```bash
# Ensure binary is executable
chmod +x code-mesh

# Or install to user directory
cargo install --path . --root ~/.local
```

#### WASM doesn't load in browser
- Ensure server supports WASM MIME type
- Check browser console for errors
- Verify CORS headers if loading from different domain

### Getting Help

If you encounter issues:

1. Check our [Troubleshooting Guide](../reference/troubleshooting.md)
2. Search existing [GitHub Issues](https://github.com/yourusername/code-mesh/issues)
3. Join our [Discord Community](https://discord.gg/codemesh)
4. Open a new issue with:
   - Installation method attempted
   - Operating system and version
   - Error messages and logs
   - Output of `code-mesh --version` (if applicable)