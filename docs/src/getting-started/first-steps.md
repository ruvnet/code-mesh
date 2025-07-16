# First Steps

After installing Code Mesh, this guide will walk you through your first interactions and help you understand the basic concepts.

## 1. Initial Setup

### Check Your Installation

First, verify that Code Mesh is installed correctly:

```bash
code-mesh --version
# or if using NPX:
npx code-mesh --version
```

You should see output like:
```
code-mesh 0.1.0
```

### Initialize Your First Project

Navigate to a code project and initialize Code Mesh:

```bash
cd your-project-directory
code-mesh init
```

This creates a `.code-mesh/` directory with:
- `config.json` - Project configuration
- `sessions/` - Directory for session history
- `memory/` - Directory for agent memory

### Understand the Project Structure

Code Mesh automatically detects your project type and relevant files:

```bash
code-mesh status
```

Output example:
```
Project: my-web-app
Language: TypeScript/JavaScript  
Files: 42 tracked, 156 total
Default Model: anthropic/claude-3-opus
Session: None active
Agents: Ready (0 active)
```

## 2. Authentication Setup

Code Mesh supports multiple AI providers. Let's set up authentication:

```bash
code-mesh auth login
```

You'll be prompted to select a provider:

```
? Select an AI provider:
❯ Anthropic (Claude)
  OpenAI (GPT)
  Google (Gemini)
  Mistral
  Local (Ollama)
  Other
```

Select your preferred provider and enter your API key when prompted.

### Managing Multiple Providers

You can authenticate with multiple providers:

```bash
# Check which providers are configured
code-mesh auth list

# Output:
# ✅ anthropic - claude-3-opus (default)
# ✅ openai - gpt-4
# ❌ google - not configured
```

## 3. Your First Interaction

### Interactive Mode

Start an interactive session:

```bash
code-mesh
```

You'll see the Code Mesh prompt:

```
[CodeMesh] Loaded project "my-web-app" (42 files)
[CodeMesh] Default model: anthropic/claude-3-opus
[CodeMesh] Type '/help' for commands or enter your request.

>>> 
```

Try your first query:

```
>>> Explain the structure of this project
```

Code Mesh will analyze your project and provide an overview.

### One-Shot Mode

For quick tasks, use one-shot mode:

```bash
code-mesh run "Add error handling to the login function"
```

## 4. Understanding Code Mesh Responses

### Multi-Agent Planning

For complex requests, Code Mesh uses multiple agents:

```bash
code-mesh run "Add user authentication with JWT tokens"
```

You might see:

```
[AI:Planner] Analyzing request...
[AI:Planner] Plan created:
  1. 📄 Create user model and database schema
  2. 🔧 Implement JWT token generation/validation
  3. 🛡️ Add authentication middleware
  4. 🚪 Create login/logout endpoints
  5. 🧪 Add comprehensive tests

[CodeMesh] Review plan? (y/n/edit): 
```

### Plan Interaction

You can:
- **Accept**: Type `y` to proceed with the plan
- **Reject**: Type `n` to cancel
- **Edit**: Type `edit` to modify the plan

### Execution Phase

Once approved, agents execute the plan:

```
[AI:Coder] (1/5) Creating user model... ✅
[AI:Coder] (2/5) Implementing JWT utilities... ✅
[AI:Coder] (3/5) Adding auth middleware... ✅
[AI:Coder] (4/5) Creating auth routes... ✅
[AI:Tester] (5/5) Running tests... ✅

[AI:Reviewer] All tasks completed successfully!
```

### Review Changes

Before applying changes, Code Mesh shows a diff:

```
[AI:Reviewer] Proposed changes:

📄 src/models/user.js (new file)
+ export class User {
+   constructor(email, passwordHash) {
+     this.email = email;
+     this.passwordHash = passwordHash;
+   }
+ }

📄 src/middleware/auth.js (new file)
+ import jwt from 'jsonwebtoken';
+ 
+ export const requireAuth = (req, res, next) => {
+   // JWT validation logic
+ }

📄 src/routes/auth.js (new file)
+ // Login and logout endpoints

📄 tests/auth.test.js (new file)
+ // Comprehensive authentication tests

[CodeMesh] Apply these changes? (y/n/view):
```

Choose:
- `y`: Apply all changes
- `n`: Reject changes
- `view`: See detailed diff for each file

## 5. Interactive Commands

In interactive mode, you can use special commands:

### System Commands

```
>>> /help                    # Show available commands
>>> /status                  # Show current status
>>> /history                 # Show conversation history
>>> /clear                   # Clear current session
>>> /save my-session         # Save session with name
>>> /load my-session         # Load saved session
>>> /exit                    # Exit Code Mesh
```

### Model Management

```
>>> /models                  # List available models
>>> /switch openai/gpt-4     # Switch to different model
>>> /provider anthropic      # Switch provider
```

### File Operations

```
>>> /files                   # List project files
>>> /open src/app.js         # Show file contents
>>> /edit src/app.js         # Open file for editing
>>> /diff                    # Show pending changes
```

## 6. Understanding Sessions

### Session Persistence

Code Mesh automatically saves your session:

```bash
# Continue your last session
code-mesh run --continue "Add input validation to the auth endpoints"

# Continue a specific session
code-mesh run --session my-session "Refactor the user model"
```

### Session Management

```bash
# List all sessions
code-mesh sessions list

# Show session details
code-mesh sessions show my-session

# Delete old sessions
code-mesh sessions clean --older-than 30d
```

## 7. Working with Tools

Code Mesh has built-in tools that agents can use:

### File Operations
- Reading and writing files
- Creating directories
- Searching code

### Code Analysis
- Running tests
- Linting code
- Building projects

### Web Tools
- Searching documentation
- Fetching API references

You can see tool usage in action:

```
[AI:Coder] Using tool: read_file("src/app.js")
[AI:Coder] Using tool: run_tests("npm test")
[AI:Tester] Test results: ✅ 15 passed, ❌ 2 failed
[AI:Coder] Using tool: write_file("src/auth.js", content)
```

## 8. Best Practices for Beginners

### Start Small
Begin with simple requests:
- "Explain this function"
- "Add comments to this code"
- "Fix this syntax error"

### Be Specific
Instead of: "Make this better"
Try: "Add error handling and input validation to the login function"

### Review Everything
Always review generated code before applying changes.

### Use Interactive Mode
For learning and exploration, interactive mode is more helpful than one-shot commands.

### Leverage Planning
For complex tasks, let the planner break things down:
```bash
code-mesh run "Implement a complete REST API for user management" --mode plan
```

## 9. Common First-Time Issues

### Authentication Errors
```bash
# Check auth status
code-mesh auth list

# Re-authenticate if needed
code-mesh auth login
```

### Project Not Recognized
```bash
# Ensure you're in a code project
ls -la

# Manually initialize if needed
code-mesh init --force
```

### Slow Performance
```bash
# Use native binary instead of NPX
cargo install code-mesh-cli

# Or adjust model for faster responses
code-mesh run "your query" --model anthropic/claude-3-haiku
```

## 10. Next Steps

Now that you're familiar with the basics:

1. 📖 Read the [CLI Reference](../user-guide/cli-reference.md) for complete command documentation
2. 🔧 Learn about [Configuration](configuration.md) options
3. 🤖 Explore [Multi-Agent Workflows](../user-guide/multi-agent.md)
4. 🛠️ Check out [Tool System](../user-guide/tools.md) capabilities
5. 💡 Browse [Examples](../examples/workflows.md) for inspiration

## Getting Help

- Use `/help` in interactive mode
- Check the [FAQ](../reference/faq.md)
- Visit the [Troubleshooting Guide](../reference/troubleshooting.md)
- Join our community discussions

Happy coding with Code Mesh! 🚀