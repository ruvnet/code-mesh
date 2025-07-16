# Quick Start

Get up and running with Code Mesh in under 5 minutes!

## Installation

### Via NPX (Recommended)

The fastest way to try Code Mesh is using NPX:

```bash
npx code-mesh run "Help me implement a REST API"
```

No installation required! Code Mesh will download and run automatically.

### Via Cargo (Native)

For the best performance, install the native binary:

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

## First Run

Let's create your first Code Mesh session:

### 1. Initialize a Project

```bash
# In your project directory
npx code-mesh init
```

This creates a `.code-mesh/config.json` file with default settings.

### 2. Set Up Authentication

Code Mesh supports multiple AI providers:

```bash
npx code-mesh auth login
```

Select your preferred provider (Anthropic, OpenAI, etc.) and enter your API key.

### 3. Run Your First Query

```bash
# Interactive mode
npx code-mesh

# Direct mode
npx code-mesh run "Add a function to calculate fibonacci numbers"
```

## Basic Usage

### Interactive Mode

Start Code Mesh without arguments for interactive mode:

```bash
npx code-mesh
```

You'll see:

```
[CodeMesh] Loaded project "my-app" (15 files, default model: Claude-3-Opus)
[CodeMesh] Type '/help' for commands. Enter your request.
>>> 
```

### One-Shot Mode

Execute a single command:

```bash
npx code-mesh run "Implement error handling for the user service"
```

### Continue Previous Session

```bash
npx code-mesh run --continue "Add unit tests for the new functionality"
```

## Example Workflow

Here's a typical Code Mesh workflow:

```bash
# 1. Start with a high-level request
npx code-mesh run "Add user authentication to my Express.js app"

# 2. Code Mesh creates a plan:
#    1. Create user model and database schema
#    2. Implement authentication middleware
#    3. Add login/logout routes
#    4. Protect existing routes
#    5. Add tests

# 3. Review and approve the plan
# 4. Code Mesh implements each step
# 5. Review the final diff and apply changes
```

## Key Features to Try

### Multi-Agent Collaboration

```bash
npx code-mesh run "Refactor this component and add comprehensive tests" --mode plan
```

Watch as different agents handle planning, coding, and testing.

### Session Memory

Code Mesh remembers your conversation:

```bash
npx code-mesh run "Add a user registration endpoint"
# Later...
npx code-mesh run --continue "Now add email verification to that endpoint"
```

### Tool Integration

Code Mesh can run tests, check code quality, and more:

```bash
npx code-mesh run "Fix the failing tests and improve code coverage"
```

## Next Steps

- 📖 Read the [CLI Reference](../user-guide/cli-reference.md) for all available commands
- 🔧 Learn about [Configuration](configuration.md) options
- 🤖 Explore [Multi-Agent Workflows](../user-guide/multi-agent.md)
- 🌐 Try [WASM/Browser Usage](../user-guide/wasm.md)

## Common Issues

### Authentication Problems

```bash
# Check your authentication status
npx code-mesh auth list

# Re-authenticate if needed
npx code-mesh auth login
```

### Model Not Available

```bash
# List available models
npx code-mesh models

# Use a specific model
npx code-mesh run "your query" --model openai/gpt-4
```

### Performance Issues

Use the native binary for better performance:

```bash
cargo install code-mesh-cli
```

## Getting Help

- Use `/help` in interactive mode
- Check the [Troubleshooting Guide](../reference/troubleshooting.md)
- Visit our [FAQ](../reference/faq.md)
- Open an issue on [GitHub](https://github.com/yourusername/code-mesh/issues)