# Configuration

Code Mesh uses a hierarchical configuration system that allows for global defaults, project-specific settings, and runtime overrides.

## Configuration Hierarchy

Settings are loaded in this order (later overrides earlier):

1. **Built-in defaults**
2. **Global config** (`~/.config/code-mesh/config.toml`)
3. **Project config** (`.code-mesh/config.toml`)
4. **Environment variables**
5. **Command-line flags**

## Configuration File Format

Code Mesh uses TOML format for configuration files:

```toml
# Global settings
[core]
default_provider = "anthropic"
default_model = "claude-3-opus"
max_context_length = 100000
session_auto_save = true
log_level = "info"

# Provider configurations
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
models = ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"]
default_model = "claude-3-opus"
max_tokens = 4096
temperature = 0.7

[providers.openai]
api_key_env = "OPENAI_API_KEY"
models = ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]
default_model = "gpt-4"
max_tokens = 4096
temperature = 0.7

[providers.mistral]
api_key_env = "MISTRAL_API_KEY"
endpoint = "https://api.mistral.ai/v1"
models = ["mistral-large", "mistral-medium"]

# Agent configurations
[agents]
max_concurrent = 3
planning_enabled = true
tool_timeout = 30
memory_limit = "1GB"

[agents.planner]
model_override = "anthropic/claude-3-opus"
max_tasks = 10
auto_approve = false

[agents.coder]
model_override = "anthropic/claude-3-sonnet"
max_file_size = "1MB"
backup_enabled = true

[agents.tester]
model_override = "openai/gpt-4"
auto_run_tests = true
test_timeout = 300

# Tool configurations
[tools]
enabled = ["file", "bash", "web", "git"]
timeout = 30
sandbox_mode = false

[tools.bash]
allowed_commands = ["npm", "cargo", "git", "ls", "cat"]
blocked_commands = ["rm", "sudo", "chmod", "dd"]
timeout = 60

[tools.web]
max_requests_per_minute = 10
user_agent = "CodeMesh/0.1.0"
timeout = 10

# UI and display settings
[ui]
theme = "dark"
show_progress = true
show_diffs = true
auto_apply = false
editor = "code"

[ui.colors]
success = "green"
error = "red"
warning = "yellow"
info = "blue"

# Storage settings
[storage]
backend = "file"  # "file" or "sqlite"
session_retention = "30d"
max_sessions = 100
compression = true

[storage.file]
base_path = "~/.local/share/code-mesh"
backup_enabled = true

[storage.sqlite]
database_url = "~/.local/share/code-mesh/sessions.db"
pool_size = 5
```

## Core Settings

### Global Options

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `default_provider` | string | `"anthropic"` | Default AI provider |
| `default_model` | string | `"claude-3-opus"` | Default model within provider |
| `max_context_length` | integer | `100000` | Maximum context window size |
| `session_auto_save` | boolean | `true` | Auto-save sessions |
| `log_level` | string | `"info"` | Logging level (trace, debug, info, warn, error) |

### Provider Configuration

Each provider can be configured with:

```toml
[providers.{provider_name}]
api_key_env = "ENV_VAR_NAME"     # Environment variable for API key
endpoint = "https://api.url"      # Custom API endpoint
models = ["model1", "model2"]     # Available models
default_model = "model1"          # Default model for this provider
max_tokens = 4096                 # Maximum tokens per request
temperature = 0.7                 # Model temperature
timeout = 30                      # Request timeout in seconds
retry_count = 3                   # Number of retries
```

### Agent Configuration

```toml
[agents]
max_concurrent = 3                # Maximum concurrent agents
planning_enabled = true           # Enable multi-agent planning
tool_timeout = 30                 # Tool execution timeout
memory_limit = "1GB"              # Memory limit per agent

# Per-agent overrides
[agents.planner]
model_override = "provider/model" # Use specific model for planning
max_tasks = 10                    # Maximum tasks in a plan
auto_approve = false              # Auto-approve plans

[agents.coder]
model_override = "provider/model" # Use specific model for coding
max_file_size = "1MB"             # Maximum file size to process
backup_enabled = true             # Create backups before editing

[agents.tester]
model_override = "provider/model" # Use specific model for testing
auto_run_tests = true             # Automatically run tests
test_timeout = 300                # Test execution timeout
```

## Environment Variables

All configuration can be overridden with environment variables using the prefix `CODE_MESH_`:

```bash
# Core settings
export CODE_MESH_DEFAULT_PROVIDER=openai
export CODE_MESH_DEFAULT_MODEL=gpt-4
export CODE_MESH_LOG_LEVEL=debug

# Provider API keys
export ANTHROPIC_API_KEY=your_key_here
export OPENAI_API_KEY=your_key_here
export MISTRAL_API_KEY=your_key_here

# Agent settings
export CODE_MESH_AGENTS_MAX_CONCURRENT=5
export CODE_MESH_AGENTS_PLANNING_ENABLED=false

# Tool settings  
export CODE_MESH_TOOLS_TIMEOUT=60
export CODE_MESH_TOOLS_SANDBOX_MODE=true
```

## Project-Specific Configuration

Create `.code-mesh/config.toml` in your project root:

```toml
# Project-specific settings
[core]
default_model = "anthropic/claude-3-sonnet"  # Faster model for this project

[project]
name = "my-web-app"
language = "typescript"
framework = "react"
test_command = "npm test"
build_command = "npm run build"
lint_command = "npm run lint"

# Project-specific agent behavior
[agents.coder]
auto_format = true
style_guide = "prettier"

[agents.tester]
test_framework = "jest"
coverage_threshold = 80

# Ignore patterns
[project.ignore]
files = ["node_modules", "dist", ".git", "*.log"]
extensions = [".min.js", ".map"]
```

## Command-Line Overrides

Most settings can be overridden via command-line flags:

```bash
# Override default model
code-mesh run "your query" --model openai/gpt-4

# Override provider
code-mesh run "your query" --provider anthropic

# Override agent settings
code-mesh run "your query" --max-agents 5

# Override tool settings
code-mesh run "your query" --tool-timeout 60

# Override UI settings
code-mesh run "your query" --no-progress --no-color
```

## Configuration Validation

Validate your configuration:

```bash
# Check current configuration
code-mesh config show

# Validate configuration files
code-mesh config validate

# Show configuration sources
code-mesh config sources
```

## Common Configuration Patterns

### Development Setup

```toml
[core]
default_model = "anthropic/claude-3-haiku"  # Faster for development
log_level = "debug"

[agents]
max_concurrent = 1                          # Simpler debugging
planning_enabled = false                    # Direct execution

[tools.bash]
sandbox_mode = true                         # Safety in development
```

### Production Setup

```toml
[core]
default_model = "anthropic/claude-3-opus"   # Best quality
log_level = "warn"

[agents]
max_concurrent = 5                          # Full parallelism
planning_enabled = true                     # Complex workflows

[storage]
compression = true                          # Save space
backup_enabled = true                       # Safety
```

### Team Setup

```toml
[core]
session_auto_save = true                    # Share sessions
default_provider = "anthropic"              # Consistent provider

[project]
shared_sessions = true                      # Team collaboration
session_prefix = "team"                     # Namespace sessions

[storage]
backend = "sqlite"                          # Better for sharing
database_url = "./shared/sessions.db"       # Team database
```

## Security Considerations

### API Key Management

- **Never commit API keys** to version control
- Use environment variables or secure key management
- Rotate keys regularly
- Use least-privilege API keys when possible

```toml
# Good: Use environment variables
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"

# Bad: Never do this
[providers.anthropic]
api_key = "sk-ant-actual-key-here"  # ❌ NEVER!
```

### Tool Security

```toml
[tools.bash]
sandbox_mode = true                         # Limit command execution
allowed_commands = ["npm", "cargo", "git"]  # Whitelist safe commands
blocked_commands = ["rm", "sudo", "curl"]   # Blacklist dangerous commands
timeout = 30                                # Prevent hanging commands
```

### File Access

```toml
[tools.file]
max_file_size = "10MB"                      # Limit file sizes
allowed_paths = ["./src", "./tests"]        # Restrict file access
blocked_paths = ["/etc", "/home"]           # Block system directories
```

## Migration from OpenCode

If migrating from OpenCode, convert your configuration:

```bash
# Convert OpenCode config to Code Mesh format
code-mesh migrate --from opencode --config ~/.config/opencode/config.json
```

This creates a compatible `.code-mesh/config.toml` with your existing settings.

## Troubleshooting Configuration

### Debug Configuration Loading

```bash
# Show effective configuration
code-mesh config show --resolved

# Show configuration file locations
code-mesh config paths

# Validate configuration
code-mesh config validate --verbose
```

### Common Issues

#### Provider Not Found
```toml
# Ensure provider is properly configured
[providers.anthropic]
api_key_env = "ANTHROPIC_API_KEY"
models = ["claude-3-opus"]
```

#### Model Not Available
```bash
# List available models for provider
code-mesh models --provider anthropic

# Check provider configuration
code-mesh config show providers.anthropic
```

#### Tool Permissions
```toml
# Ensure tools are enabled
[tools]
enabled = ["file", "bash", "web"]

# Check tool-specific settings
[tools.bash]
allowed_commands = ["npm", "git"]
```

## Configuration Schema

The complete configuration schema is available at: [`schema/config.json`](https://github.com/yourusername/code-mesh/blob/main/schema/config.json)

Use it for IDE autocompletion and validation in VS Code and other editors that support JSON Schema.