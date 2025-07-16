# CLI Reference

Complete reference for all Code Mesh command-line interface commands and options.

## Global Options

These options are available for all commands:

```
--verbose, -v        Increase verbosity (can be used multiple times)
--help, -h          Show help information
--version           Show version information
```

### Verbosity Levels

- No flag: Info level logging
- `-v`: Debug level logging  
- `-vv`: Trace level logging

## Commands

### `code-mesh` (Interactive Mode)

Start Code Mesh in interactive mode.

```bash
code-mesh [OPTIONS]
```

**Options:**
- `--model MODEL` - Override default model (e.g., `anthropic/claude-3-opus`)
- `--session SESSION_ID` - Load specific session
- `--no-save` - Don't auto-save session
- `--theme THEME` - UI theme (`dark`, `light`, `auto`)

**Interactive Commands:**

Once in interactive mode, you can use these commands:

| Command | Description | Example |
|---------|-------------|---------|
| `/help` | Show available commands | `/help` |
| `/exit` | Exit Code Mesh | `/exit` |
| `/clear` | Clear current session | `/clear` |
| `/history` | Show conversation history | `/history` |
| `/save [name]` | Save session | `/save my-feature` |
| `/load <name>` | Load saved session | `/load my-feature` |
| `/status` | Show current status | `/status` |
| `/models` | List available models | `/models` |
| `/switch <model>` | Switch to different model | `/switch openai/gpt-4` |
| `/provider <name>` | Switch provider | `/provider anthropic` |
| `/files` | List project files | `/files` |
| `/open <path>` | Show file contents | `/open src/app.js` |
| `/edit <path>` | Open file for editing | `/edit src/main.rs` |
| `/diff` | Show pending changes | `/diff` |
| `/apply` | Apply pending changes | `/apply` |
| `/reject` | Reject pending changes | `/reject` |
| `/plan` | Show current plan | `/plan` |
| `/agents` | Show active agents | `/agents` |

### `code-mesh run`

Execute a single prompt and exit.

```bash
code-mesh run [OPTIONS] <MESSAGE>...
```

**Arguments:**
- `<MESSAGE>...` - The message to send to Code Mesh

**Options:**
- `--continue, -c` - Continue the last session
- `--session SESSION_ID, -s` - Continue specific session
- `--model MODEL, -m` - Use specific model
- `--mode MODE` - Execution mode (`chat`, `plan`, `code`, `test`)
- `--no-plan` - Skip planning phase
- `--auto-apply` - Automatically apply changes
- `--dry-run` - Show what would be done without executing

**Examples:**

```bash
# Basic usage
code-mesh run "Add error handling to the login function"

# Continue previous session
code-mesh run --continue "Add unit tests for that function"

# Use specific model
code-mesh run --model openai/gpt-4 "Optimize this algorithm"

# Use planning mode
code-mesh run --mode plan "Implement user authentication system"

# Auto-apply changes (be careful!)
code-mesh run --auto-apply "Fix all linting errors"

# Dry run to see planned changes
code-mesh run --dry-run "Refactor the database layer"
```

### `code-mesh auth`

Manage authentication with AI providers.

```bash
code-mesh auth <COMMAND>
```

#### `code-mesh auth login`

Log in to an AI provider.

```bash
code-mesh auth login [OPTIONS]
```

**Options:**
- `--provider PROVIDER` - Specific provider to configure
- `--key KEY` - API key (if not provided, will prompt)
- `--oauth` - Use OAuth flow (if supported)

**Interactive Login:**

```bash
code-mesh auth login
```

You'll be prompted to:
1. Select a provider
2. Enter your API key
3. Test the connection

**Direct Login:**

```bash
code-mesh auth login --provider anthropic --key sk-ant-your-key-here
```

#### `code-mesh auth logout`

Log out from a provider.

```bash
code-mesh auth logout <PROVIDER>
```

**Arguments:**
- `<PROVIDER>` - Provider to log out from

**Example:**
```bash
code-mesh auth logout anthropic
```

#### `code-mesh auth list`

List authenticated providers.

```bash
code-mesh auth list [OPTIONS]
```

**Options:**
- `--verbose, -v` - Show additional details
- `--test` - Test all connections

**Example Output:**
```
✅ anthropic - claude-3-opus (default)
✅ openai - gpt-4  
❌ google - not configured
⚠️  mistral - connection error
```

### `code-mesh init`

Initialize Code Mesh in a project.

```bash
code-mesh init [OPTIONS] [PATH]
```

**Arguments:**
- `[PATH]` - Project path (default: current directory)

**Options:**
- `--force, -f` - Overwrite existing configuration
- `--template TEMPLATE` - Use configuration template
- `--language LANG` - Set primary language
- `--framework FRAMEWORK` - Set framework type

**Examples:**

```bash
# Initialize in current directory
code-mesh init

# Initialize in specific directory
code-mesh init ~/my-project

# Initialize with template
code-mesh init --template typescript-react

# Force overwrite existing config
code-mesh init --force
```

**Templates:**

- `rust` - Rust project configuration
- `typescript` - TypeScript/JavaScript project
- `typescript-react` - React TypeScript project
- `python` - Python project configuration
- `go` - Go project configuration
- `minimal` - Minimal configuration

### `code-mesh status`

Show status and health information.

```bash
code-mesh status [OPTIONS]
```

**Options:**
- `--detailed, -d` - Show detailed status
- `--json` - Output in JSON format
- `--check` - Run health checks

**Example Output:**

```
Code Mesh Status
================

Project: my-web-app (TypeScript/React)
Files: 42 tracked, 156 total
Config: .code-mesh/config.toml

Authentication:
✅ anthropic - claude-3-opus (default)
✅ openai - gpt-4

Current Session: session_2024-01-15_14-30-25
Messages: 8 (4 user, 4 assistant)
Agents: 0 active, 3 available

Tools: file, bash, web, git (4 enabled)
Storage: 15.2 MB used, 47 sessions
Memory: 256 MB used, 1 GB limit

Health: ✅ All systems operational
```

### `code-mesh serve`

Start Code Mesh API server.

```bash
code-mesh serve [OPTIONS]
```

**Options:**
- `--port PORT, -p` - Port to listen on (default: 3000)
- `--host HOST` - Host to bind to (default: 127.0.0.1)
- `--cors` - Enable CORS headers
- `--auth` - Require authentication
- `--config CONFIG` - Configuration file

**Examples:**

```bash
# Start on default port
code-mesh serve

# Start on specific port
code-mesh serve --port 8080

# Start with CORS enabled
code-mesh serve --cors --host 0.0.0.0
```

**API Endpoints:**

- `GET /health` - Health check
- `POST /sessions` - Create new session
- `GET /sessions/:id` - Get session
- `POST /sessions/:id/messages` - Send message
- `GET /models` - List available models
- `POST /auth/test` - Test authentication

### `code-mesh models`

List available models and providers.

```bash
code-mesh models [OPTIONS]
```

**Options:**
- `--provider PROVIDER, -p` - Filter by provider
- `--available` - Show only available models
- `--json` - Output in JSON format

**Examples:**

```bash
# List all models
code-mesh models

# List Anthropic models only
code-mesh models --provider anthropic

# Show only available models
code-mesh models --available
```

**Example Output:**

```
Available Models
================

anthropic (✅ authenticated):
  claude-3-opus      - Most capable model
  claude-3-sonnet    - Balanced performance/speed  
  claude-3-haiku     - Fastest model

openai (✅ authenticated):
  gpt-4             - Most capable GPT model
  gpt-4-turbo       - Faster GPT-4 variant
  gpt-3.5-turbo     - Fast and efficient

mistral (❌ not authenticated):
  mistral-large     - Largest Mistral model
  mistral-medium    - Balanced Mistral model
```

### `code-mesh sessions`

Manage sessions.

```bash
code-mesh sessions <COMMAND>
```

#### `code-mesh sessions list`

List all sessions.

```bash
code-mesh sessions list [OPTIONS]
```

**Options:**
- `--limit LIMIT, -l` - Limit number of sessions
- `--project PROJECT` - Filter by project
- `--since DATE` - Show sessions since date
- `--format FORMAT` - Output format (`table`, `json`, `csv`)

#### `code-mesh sessions show`

Show session details.

```bash
code-mesh sessions show <SESSION_ID>
```

#### `code-mesh sessions delete`

Delete sessions.

```bash
code-mesh sessions delete [OPTIONS] [SESSION_ID]...
```

**Options:**
- `--older-than DURATION` - Delete sessions older than duration
- `--project PROJECT` - Delete sessions for specific project
- `--all` - Delete all sessions (requires confirmation)

### `code-mesh config`

Manage configuration.

```bash
code-mesh config <COMMAND>
```

#### `code-mesh config show`

Show current configuration.

```bash
code-mesh config show [OPTIONS] [KEY]
```

**Options:**
- `--resolved` - Show resolved configuration (after merging all sources)
- `--source` - Show configuration source for each value
- `--format FORMAT` - Output format (`toml`, `json`, `yaml`)

#### `code-mesh config set`

Set configuration value.

```bash
code-mesh config set <KEY> <VALUE>
```

**Examples:**

```bash
# Set default model
code-mesh config set core.default_model anthropic/claude-3-sonnet

# Set UI theme
code-mesh config set ui.theme dark

# Enable auto-apply
code-mesh config set ui.auto_apply true
```

#### `code-mesh config validate`

Validate configuration files.

```bash
code-mesh config validate [OPTIONS]
```

**Options:**
- `--file FILE` - Validate specific file
- `--fix` - Attempt to fix validation errors

## Environment Variables

Code Mesh respects these environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `CODE_MESH_CONFIG` | Configuration file path | `~/.config/code-mesh/config.toml` |
| `CODE_MESH_LOG_LEVEL` | Log level | `debug` |
| `CODE_MESH_DEFAULT_MODEL` | Default model | `anthropic/claude-3-opus` |
| `ANTHROPIC_API_KEY` | Anthropic API key | `sk-ant-...` |
| `OPENAI_API_KEY` | OpenAI API key | `sk-...` |
| `MISTRAL_API_KEY` | Mistral API key | `...` |
| `CODE_MESH_SESSION_DIR` | Session storage directory | `~/.local/share/code-mesh/sessions` |

## Exit Codes

Code Mesh uses these exit codes:

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Authentication error |
| 4 | Network error |
| 5 | Tool execution error |
| 130 | Interrupted by user (Ctrl+C) |

## Shell Completion

Generate shell completion scripts:

```bash
# Bash
code-mesh completion bash > ~/.bash_completion.d/code-mesh

# Zsh  
code-mesh completion zsh > ~/.zsh/completions/_code-mesh

# Fish
code-mesh completion fish > ~/.config/fish/completions/code-mesh.fish

# PowerShell
code-mesh completion powershell > code-mesh.ps1
```

## Keyboard Shortcuts

In interactive mode:

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Cancel current operation |
| `Ctrl+D` | Exit Code Mesh |
| `Ctrl+L` | Clear screen |
| `↑`/`↓` | Navigate command history |
| `Tab` | Auto-complete |
| `Ctrl+R` | Search command history |

## Configuration Files

Code Mesh looks for configuration files in this order:

1. `--config` flag value
2. `CODE_MESH_CONFIG` environment variable
3. `.code-mesh/config.toml` (project)
4. `~/.config/code-mesh/config.toml` (user)
5. `/etc/code-mesh/config.toml` (system)

## Examples

### Common Workflows

```bash
# Start a new feature
code-mesh run "Plan implementation of user profile management"

# Continue working on feature
code-mesh run --continue "Add validation to the profile update form"

# Run tests after changes
code-mesh run "Run the test suite and fix any failures"

# Code review
code-mesh run "Review the recent changes and suggest improvements"

# Documentation
code-mesh run "Add documentation for the new API endpoints"
```

### Debugging

```bash
# Enable debug logging
code-mesh -vv run "Debug the authentication issue"

# Check status
code-mesh status --detailed

# Test authentication
code-mesh auth list --test

# Validate configuration
code-mesh config validate --fix
```

For more examples, see the [Examples](../examples/workflows.md) section.