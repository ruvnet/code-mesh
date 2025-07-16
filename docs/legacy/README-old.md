# Code Mesh

A high-performance AI coding assistant built with Rust and WebAssembly, inspired by OpenCode.

## 🚀 Overview

Code Mesh is a complete port of [OpenCode](https://github.com/sst/opencode) to Rust, providing:
- Native performance with Rust
- WebAssembly support for browser/Node.js usage
- Modular architecture with three core crates
- Multi-LLM provider support
- Comprehensive tool system
- NPX distribution for easy access

## 📋 Project Status

This project is currently in active development. See [EPIC-code-mesh-port.md](EPIC-code-mesh-port.md) for the complete implementation roadmap.

## 🏗️ Architecture

### Crate Structure
```
code-mesh/
├── crates/
│   ├── code-mesh-core/     # Core functionality and abstractions
│   │   ├── src/
│   │   │   ├── agent/      # Agent orchestration
│   │   │   ├── llm/        # LLM trait & provider implementations
│   │   │   ├── planner/    # Task planning and decomposition
│   │   │   ├── session/    # Session and message management
│   │   │   ├── memory/     # Memory and context storage
│   │   │   ├── tool/       # Tool trait & implementations
│   │   │   ├── auth/       # Authentication system
│   │   │   └── storage/    # Persistent storage abstractions
│   │   └── Cargo.toml
│   ├── code-mesh-cli/      # Native CLI application
│   │   ├── src/
│   │   │   ├── cmd/        # Command implementations
│   │   │   ├── tui/        # Terminal UI components
│   │   │   └── main.rs     # CLI entry point
│   │   └── Cargo.toml
│   └── code-mesh-wasm/     # WebAssembly bindings
│       ├── src/
│       │   ├── bindings/   # wasm-bindgen interfaces
│       │   └── lib.rs      # WASM entry point
│       └── Cargo.toml
├── npm/                    # NPM package for distribution
│   ├── package.json
│   └── bin/
│       └── code-mesh.js    # NPX runner script
└── Cargo.toml             # Workspace root
```

### Key Design Principles

1. **Modular Architecture**: Three separate crates with clear boundaries
2. **Platform Agnostic Core**: Core logic works on both native and WASM
3. **Trait-Based Design**: Extensible providers, tools, and storage
4. **Async-First**: Built on Tokio for native, wasm-bindgen-futures for WASM
5. **Type Safety**: Leveraging Rust's type system for reliability
  * `embed-models` – possibly for bundling local models or enabling heavy features (e.g. if we integrate a local inference library for offline mode). This could be off by default to keep WASM small.
  * `wasm-bindgen` – if the core crate uses `wasm-bindgen`, we may gate those extern definitions behind a feature to avoid conflicts on native.

Features make the build configurable. For example, we might compile `code-mesh-core` for browser with only `openai` and `anthropic` enabled (to keep package small) or compile a full native binary with all providers included.

**1.5 Wasm-Bindgen Interface:** To expose functionality to JavaScript (for NPX and browser use), we will use **`wasm-bindgen`** in either the core or wrapper crate. Key functions to export could include:

* `init_project(path: &str)` – initialize a project (generating config files). In WASM, `path` might be ignored or virtual.
* `run_prompt(prompt: &str) -> JsValue` – execute a one-shot prompt (like `code-mesh run`), returning the result (could be a string or structured JSON).
* `start_session()` / `send_message(msg: &str)` – for interactive usage: start a new session and send user messages, with the function returning the AI’s response (allowing a web UI to build a chat).
* `get_status() -> JsValue` – fetch current status (for the `status` command or UI panels, e.g. what tasks are in progress, which model is used, etc.).
* `load_session(id: &str)` – load a past session transcript.

Each exported function will use `#[wasm_bindgen]` annotation. We’ll also mark the crate type as `cdylib` for WASM. In web builds, the `ProjectFS` implementation will internally use browser APIs (possibly via `web_sys` for IndexedDB, see Phase 5) instead of real FS. The `llm` module will use `fetch` (via `wasm-bindgen` or `reqwest` with feature flags) for network calls since direct sockets aren’t available in browser.

**1.6 Concurrency Model:** Rust’s async runtime (e.g. `tokio` or `async-std`) will allow concurrent tasks without OS threads (important for WASM, which doesn’t support threads by default). The core will be largely asynchronous:

* LLM API calls will be `async` so that we can issue multiple queries in parallel (for multi-agent or concurrent tool use).
* Agents coordination (Phase 3) can use async tasks or a lightweight task scheduler.
* For native builds, we can also enable multi-threading for parallel tasks if needed (ensuring to use `wasm-bindgen-rayon` or similar if we ever attempt parallelism in WASM).

**1.7 Continuous Integration (CI) Setup:** We will set up CI to enforce this architecture:

* **Build matrix:** Test compiling the project for both `x86_64-unknown-linux-gnu` (native) and `wasm32-unknown-unknown` (WASM) to catch any incompatibilities early.
* **Automated tests:** Run unit tests for core logic on native. We can use `wasm-pack test --node` to run tests in a Node environment for the WASM build as well, ensuring core logic behaves the same. (Phase 5 covers testing in detail.)
* **Linting/format:** Use `cargo fmt` and `cargo clippy` with appropriate target flags in CI to keep code quality consistent.

By the end of Phase 1, we will have a skeleton repository with proper crate structure, configuration, and stubs for each module. This enables parallel development in subsequent phases (e.g. one group can work on CLI parsing while another works on LLM integration, since the interfaces will be defined).

## Phase 2: Module Implementation Breakdown

In this phase we implement the core modules identified above. Each module corresponds to a cohesive piece of functionality, making it easier for a “hive-mind” of contributors to work concurrently. Below we break down each major module, along with responsibilities and interactions:

**2.1 CLI Command Module (Argument Parsing & Routing):**
**Scope:** Provide a user interface for all commands (`init`, `run`, `auth`, `status`, and default interactive mode).
**Implementation:** Use the **`clap`** crate (or an equivalent like `structopt` or `clap_derive`) to define subcommands and flags. For example:

* `code-mesh init [path]`: Initialize a project at the given path (or current directory if none). This might create a default config file (e.g. `.code-mesh/config.json`) and possibly a sample “.gitignore” or template. It could also set up a new session. We will implement this by calling a core function `Config::init_project(path)` in the `config.rs` module of core.
* `code-mesh run "<prompt>"`: Run in one-shot mode, taking a prompt from the CLI and returning the result directly to stdout. This will call an API like `core::execute_prompt(prompt, options)` where `options` might include model selection (`-m` flag) or session reuse (`-c` to continue last session). The core will handle loading context and producing an answer (possibly by spinning up the agent orchestration for a single query).
* `code-mesh auth [login|list|logout]`: Manage API keys/credentials. We will mirror OpenCode’s approach: `auth login` should prompt the user to select a provider and enter API key, then save it in a credentials store (e.g. `~/.local/share/code-mesh/auth.json`). We’ll implement this in the `config.rs` or `llm.rs` module by maintaining a mapping of provider -> API key/token. `auth list` displays which providers have keys stored, and `auth logout [provider]` removes a key. For interactive selection (like choosing provider from a list), we can use a simple text prompt (or in future, a nicer TUI dialog).
* `code-mesh status`: Show the current status of Code Mesh. This is an extension beyond OpenCode. For example, it could display which model is currently active, how many agents are running or idle, and perhaps the last operation or an overview of session memory. If Code Mesh has background tasks (as in Jules’ asynchronous mode), `status` will enumerate them. Implementation: call a core method that returns a struct or string of status info (including active session ID, list of agents or threads in use, and any queued tasks).
* (Optional) `code-mesh upgrade`: Similar to OpenCode’s self-update. This can be deferred if we rely on cargo or npm updates, but including it improves UX. It might simply print a message advising to update via npm or cargo. A more advanced approach is to have the CLI check the npm registry for a newer version.

All these subcommands route to functions in `code-mesh-core`. The `main.rs` will parse args and match on subcommand. For example, a pseudocode snippet:

```rust
match cli_args.subcommand() {
    "init" => core::Config::init_project(path),
    "run" => {
        let prompt = cli_args.get_one::<String>("message").unwrap();
        let opts = make_options_from_flags(cli_args);
        let output = core::Session::run_prompt(prompt, opts);
        println!("{}", output);
    },
    "auth" => handle_auth_subcommand(cli_args),
    "status" => println!("{}", core::Session::current_status()),
    _ => interactive_repl(core::Session::new()),
}
```

This module is relatively straightforward and can be developed in parallel with core logic by stubbing calls initially. For example, `core::Session::run_prompt` can be a placeholder that returns “(stubbed answer)”. This way, CLI development (argument parsing, user prompts) can proceed concurrently with the deeper implementation of agents and LLMs.

**2.2 Agent Orchestration Module:**
**Scope:** Implement the **multi-agent orchestration engine** inspired by Codex, Claude Code, and Jules. This is the heart of Code Mesh’s AI capabilities – coordinating one or more LLM “agents” to perform complex coding tasks cooperatively.
**Design:** We introduce an `Agent` struct and an `Orchestrator` (or “Swarm”) struct:

* `struct Agent { id: String, role: AgentRole, model: ModelBackend, memory: AgentMemory, ... }`
* `struct Orchestrator { agents: Vec<Agent>, strategy: OrchestrationStrategy, shared_memory: SharedMemory, task_queue: Vec<Task>, ... }`

Each `Agent` has a **role** or specialization. For example, roles might include **Planner**, **Coder**, **Tester**, or even roles like “Explainer” or “Reviewer.” The **ModelBackend** encapsulates the LLM used by that agent (e.g. GPT-4, Claude 2, etc.), and could also carry provider-specific settings. **AgentMemory** could store recent dialogue or facts known by the agent (like its viewpoint or partial code context relevant to its role).

The `Orchestrator` manages these agents. It can spawn agents as needed, assign tasks, and aggregate results. Initially, we might start with a simpler approach (one agent at a time) but design the system to allow concurrency and collaboration:

* **Single-agent baseline:** Initially run a single agent (using the chosen model) to handle user queries in a loop (basically replicating OpenCode’s behavior where one AI assistant interacts with you).
* **Multi-agent extension:** Allow orchestrator to create multiple agents for a query. For example, when the user asks for a complex feature, the orchestrator could use a **Planner agent** (perhaps using a model known for planning) to break the request into steps. This yields a list of tasks (like “1. Modify file X for backend, 2. Update UI in file Y, 3. Write unit tests…”). Then, separate **Coder agents** (possibly using code-specialized models like Codex or Claude’s coding model) can tackle each code change task. They might run in sequence or in parallel if tasks are independent. After code generation, a **Tester agent** could run the test suite (or specific tests) using tool execution, and a **Reviewer agent** might verify the diff or look for errors.
* **Coordination and communication:** Agents need to share state. We will implement a **shared memory** or message-passing system. For simplicity, a shared memory can be a structure containing:

  * The **global context** (project files, config),
  * Current **plan/task list** (if a Planner is used),
  * A transcript of the conversation or actions (so agents can see what others have done, akin to a chat history among agents).
    Agents can read/write to this memory or send messages via the Orchestrator. This is analogous to a blackboard in blackboard-system AI, or a “hive mind” state. We could model it by channels or just by orchestrator-mediated function calls (since all agents run in one process here).
* **Swarm strategies:** We will support different topologies or strategies for agent collaboration, inspired by Ruv-Swarm’s design. The default will be a **mesh network** – all agents can communicate and any agent can take the lead if needed (decentralized). This is flexible for creative problem solving. In the future, we could allow hierarchical (one master agent directing sub-agents) or other patterns if beneficial, but initial focus is a simple coordinator that behaves like a central brain orchestrating specialized sub-processes.

**Ruv-Swarm Influence:** Ruv-swarm’s concept of **ephemeral, purpose-built “micro-net” agents** will inform our implementation. Code Mesh will spawn lightweight agents on demand for tasks and dissolve them when done (for example, spin up a specialized agent to handle a single file’s refactor, then retire it). This ensures resources (like context window and memory) stay focused per task. Each agent will be configured with just enough context (and a relevant model) to solve its specific task, analogous to Ruv-Swarm’s “tiny purpose-built brains” approach. All agents run within the local process (or thread pool), and for the browser we rely on async tasks (no threads) to simulate parallelism.

**Claude Code and Jules Influence:** We integrate the successful ideas from Claude Code and Jules:

* **Planning Mode:** Just as Claude Code can break a problem into a checklist and dynamically update it, Code Mesh’s Planner agent will output a step-by-step plan (list of tasks or todo items). We will surface this to the user as needed (e.g. in interactive mode, the AI can present a plan and ask for confirmation before executing). The plan is stored in shared memory and agents tick off tasks as they complete them.
* **Tool usage (Tool Reasoning):** Jules demonstrates executing commands in a sandbox (like running tests, building code) as part of the agent’s workflow. Code Mesh will incorporate a **Tool API** for agents. This means our orchestrator or agents can call predefined “tools” such as:

  * `tool.run_command(cmd: &str)` – run a shell command (restricted to safe ones like running tests or linter).
  * `tool.open_file(path)` – load file content (for reading by the LLM, possibly chunked if large).
  * `tool.write_file(path, new_content)` – write changes to a file (likely staged as a diff or in memory until user approval, depending on mode).
  * `tool.git_diff()` – get a diff of changes made.
    These tool functions will be implemented in the core (with necessary `cfg` guards: in native they execute directly, in WASM they might be stubbed or require user to allow it). The agents won’t call these directly; rather, when an agent “decides” to use a tool (likely by outputting a special command in its text), the orchestrator intercepts it. For example, if an LLM response contains something like `<TOOL> run_tests`, Code Mesh will parse that and execute the corresponding action, then feed the result back into the agent’s input (closing the loop, so the agent can adjust based on actual test results). This design follows the idea of **Jules’s reasoning with real execution** and Anthropic’s “agentic search” where the AI can run code and see outputs. It lets agents verify their code (e.g., run the app or tests to ensure the changes work, as Jules does in a cloud VM). In Code Mesh, everything runs locally (unless future extension to cloud execution), so we must be careful to sandbox dangerous actions (initially, we will allow only read/write within the project and running user-approved commands).
* **Approval Workflow:** By default, Code Mesh will not persist any file modifications until the user approves. This is in line with Claude Code’s philosophy (“never modifies your files without explicit approval”). The orchestrator can accumulate all proposed file edits (diffs) in a staging area. When an operation or plan is complete, the CLI can show a summary or diff to the user. The user can then confirm to apply changes (write to disk) or abort. This adds a safety layer.

**Example Workflow:** To illustrate multi-agent orchestration, consider the user prompt: *“Add a new API endpoint to fetch weather data and display it in the UI.”*

1. **Planner Agent** (using a general model like GPT-4) reads this and produces a plan: e.g.

   1. *Create a new endpoint `/weather` in backend (modify `server/routes.js`)*
   2. *Fetch data from an external API in that route (maybe use an API key)*
   3. *Create a frontend component to display the weather (modify `WeatherWidget.jsx`)*
   4. *Integrate the component in homepage (modify `HomePage.jsx`)*
   5. *Write a test for the backend route (create `weather.test.js`)*.
2. Code Mesh’s orchestrator creates a **Coder Agent** with a code-savvy model (say Claude Instant or OpenAI Codex model) for each of the first three tasks (which can potentially be done in parallel since they touch different files). Alternatively, it could do them sequentially if we prefer ordering (especially if tasks have dependencies). Each coder agent gets the relevant file context (via `tool.open_file`) and the task description. They generate code modifications which we capture as diffs.
3. After coding tasks, a **Tester Agent** could run the test suite (`tool.run_command("npm test")`). Suppose the tests fail due to a bug. The agent reads the test output and either fixes it itself or informs a **Debugger Agent** role to handle it. The orchestrator may route the failure to a new agent or back to one of the coder agents to fix.
4. Once all tasks are done and tests pass, a **Reviewer Agent** (could be the original Planner or a dedicated reviewer) aggregates the diffs from all changes and perhaps comments or refines them. It might run a final `tool.git_diff()` and summarize the changes for the user.
5. The orchestrator now pauses and the CLI presents the plan and diffs to the user: the **“visible workflow”** similar to Jules (Jules shows its plan, reasoning, and diff before changes are finalized). The user can scroll through changes and then approve.
6. Upon approval, Code Mesh writes the changes to disk and possibly commits to git (optionally, we could integrate a `tool.git_commit()` if configured).

Though the above is complex, the architecture allows incremental development. Initially, we may implement a simpler loop: one agent that does planning and coding step-by-step with user prompts. Over time, we expand to concurrent multi-agent operation. Importantly, the **task planner and agent interfaces** are designed from the start, so contributors can work on them incrementally.

**2.3 LLM Integration Module:**
**Scope:** Provide a unified interface to various Large Language Models (LLMs) – OpenAI GPT series, Anthropic Claude, local models like Llama/Mistral, etc. This module will handle API calls or model inference, so that the rest of the system can call “LLM.complete(prompt)” without worrying about which provider is used. It also manages provider-specific settings (API keys, model IDs) and rate limiting or error handling.

**Design:** Define an `LLMProvider` **trait** in `llm.rs`:

```rust
trait LLMProvider {
    fn name(&self) -> &'static str;
    async fn complete(&self, prompt: &str, context: &LLMContext) -> Result<LLMResponse, LLMError>;
    // possibly streaming interface or other methods
}
```

We then implement this trait for each provider or model:

* `OpenAIClient` implements `LLMProvider` by calling OpenAI’s REST API (using `reqwest` or OpenAI’s official Rust SDK if available). It will format the prompt into the API’s JSON payload, include the API key from config, and parse the response. We allow specifying model (like `gpt-4` or `gpt-3.5-turbo`).
* `AnthropicClient` for Claude (calls Claude’s API via Anthropics SDK or direct HTTP).
* `LocalLLMClient` for local models. For example, if supporting **Mistral** or Llama2 via an inference library (like `llama-rs` or `ollama`), this implementation might call a local runtime. OpenCode allows using local models via Ollama and LMStudio, so Code Mesh can integrate similarly: e.g. if user configures an Ollama provider, the `LocalLLMClient` calls an Ollama HTTP server to get completions.
* Additional providers: DeepInfra, Cohere, etc., can be added easily by implementing the trait. We’ll design the auth system to accommodate any number of providers.

**Provider Configuration and Selection:** We will use a config file (like `~/.config/code-mesh/config.json`) to store preferences, including the default provider and model. For example:

```json
{
  "provider": "openai",
  "model": "gpt-4",
  "providers": {
     "openai": { "api_key": "sk-...", "models": ["gpt-4","gpt-3.5-turbo"] },
     "anthropic": { "api_key": "xylophone-...", "models": ["claude-2"] },
     "ollama": { "host": "localhost:11434", "models": ["llama2-7b"] }
  }
}
```

The `auth login` command will populate the `providers` section and credentials (similar to opencode’s `auth.json` and config files). We’ll also support multiple providers in one session: for instance, a user could log in to OpenAI and Anthropic and have both available. Code Mesh might use OpenAI for one task and Claude for another if configured – this could be manually specified per command (via `-m provider/model` flag as in OpenCode) or automatically based on a strategy (for example, use Claude for larger context needs, OpenAI for coding, etc.). Initially, manual selection is fine; extensibility for auto-selection can be added.

**Cooperative LLM Behavior:** With multiple LLMs integrated, we can experiment with **cooperative prompting**. For example, when faced with a tough problem, orchestrator might query both GPT-4 and Claude in parallel and then merge their answers (or have them debate). This is an advanced use-case but something the architecture enables:

* Each agent could be backed by a different model (one reason we allow per-agent model selection in the Agent struct).
* The Orchestrator can implement a **“swarm consensus”** algorithm: e.g. ask multiple models to propose a solution and either vote or have a final agent summarize the best parts. Ruv-Swarm’s results suggest combining multiple “brains” can outperform a single model, and Code Mesh’s design allows such experimentation. However, initial implementation will likely stick to one model at a time to reduce complexity.

**2.4 File System & Project Context Module:**
**Scope:** Provide file-aware capabilities – reading, writing, and keeping track of project files/codebase context. This module is crucial for “codebase aware” AI functionality (reading multiple files, making multi-file edits as Claude Code does).

**Implementation:** We’ll create an abstraction `ProjectIO` or `FileManager`:

* **Native Implementation:** Use `std::fs` to read/write files. We also use `notify` crate (optional) to watch file changes if needed for live updates in interactive mode. The file paths should be normalized and restricted to within the project root (for safety).
* **WASM Implementation:** In a browser, direct disk access is not possible. We have a few strategies:

  * Use the **File System Access API** if available (not widely supported yet, and not via pure WASM without JS help).
  * More portable: require that the web app supplies the file tree (for example, if Code Mesh runs in a web IDE, the IDE extension can send the file content to Code Mesh). We can design functions to accept file content from outside in WASM mode (e.g. an `openProject(files: JsValue)` that loads a list of {path, content} into a virtual filesystem in memory).
  * Use **IndexedDB** to persist changes (detailed in Phase 5). The idea is to maintain an in-memory representation of files (a simple `HashMap<PathBuf, String>` for content, plus perhaps an in-memory diff for unsaved changes).
  * For simplicity, initial browser support might be read-only or require user to paste code. However, since one goal is full in-browser editing, we lean towards having the host environment provide the project files. (In an IDE scenario, the extension can do that easily.)

We will include functionalities:

* `load_project(root_path)` – scans directory (native) or initializes from provided file list (web), and prepares a project context (list of file paths, maybe building an index for search).
* `read_file(path)` – returns file content (from disk or memory).
* `write_file(path, content, options)` – either directly writes (if `options.force` or user-approved) or stores in a pending diff structure. Perhaps `options.dry_run` will indicate whether to just simulate the change. By default, in interactive sessions, we accumulate changes and only flush when user approves.
* `diff(original, new)` – utility to get a diff (we can use a diff library crate to produce unified diff text for user display or for agent reasoning).

Additionally, to give the LLM agent context, we might implement a **context window manager**:

* If a user’s prompt references a specific file or function, we should load that content and include it in the LLM prompt (since LLMs have token limits). Anthropics’ Claude Code uses “agentic search” to automatically include relevant files. We can mimic this by implementing a simple search: the agent (or orchestrator) can scan for keywords or identifiers in the project and fetch those file snippets to prepend to the prompt. Another approach is using embeddings to find relevant files, but that might be a future enhancement. Initially, a heuristic or user guidance (like “user opened this file, so treat it as context”) is sufficient.

This module will ensure that our AI’s actions are **file-aware and project-scoped**. It allows the multi-agent system to perform coordinated multi-file edits (for example, a single session could have different agents editing different files concurrently, since each agent will call into FileManager which will handle locking or merging if needed).

**2.5 Memory and Session Module:**
**Scope:** Manage the conversation history, agent memories, and persistent session data. This ties in closely with LLM prompts (to maintain context) and with the file system (to remember what’s been changed or decided).

Components:

* **Session History:** A session (e.g. one interactive chat with Code Mesh in a project) will have a log of interactions: user messages, AI responses, actions taken (e.g. “test run output” or “file X changed”). We define a `Session` struct to hold this log along with metadata like session ID, timestamp, and associated project path. We will implement serialization for sessions (JSON or Markdown logging) so that they can be saved to disk (native) or browser storage.
* **Agent Memory:** Beyond the raw transcript, an agent might maintain distilled knowledge. For example, if an agent reads multiple files, it could store a summary or important facts to avoid re-reading everything if not needed. We may implement a simple vector-store or memory index in the future (this could be behind a feature flag, using something like `blinkdb` or an embedding library). Initially, agent memory might just be the last N messages or a summary string. The architecture should allow plugging in more complex memory (like RAG – retrieve relevant info on demand).
* **Global vs. Agent-specific memory:** The orchestrator’s shared memory (mentioned in **2.2**) will act as global session memory. Each agent can also have private memory (some tasks it learned, or a reflection). For example, after solving a bug, an agent could note “remember: library X has a quirk in version Y” in a knowledge base. These nuances may be future work, but our design will include placeholders for them (e.g. an interface for an agent to store a note in a `KnowledgeBase`).

**Persistence:** For native, we will store session history to files under `~/.local/share/code-mesh/sessions/`. The filename might incorporate project name or a hash, and timestamps. Possibly we maintain an index file of sessions. When `--continue` flag is used with `run`, the CLI will load the latest session for continuity. In interactive mode, we can periodically checkpoint the session to disk (so progress isn’t lost on crash).

For browser (or any WASM environment), we use **IndexedDB** or **LocalStorage**. We can use `web_sys::window().local_storage()` to get a storage and save small data (but size is limited). IndexedDB (accessible via `wasm-bindgen` as well) is better for larger data; we can store session logs as JSON strings keyed by session ID. Another approach is to send the session data out to the host (e.g. a web app could handle persistence). However, using browser storage makes Code Mesh more self-contained. We will abstract this behind the same interface so that the core code calls `SessionStore.save(session)` and the implementation uses either filesystem or IndexedDB depending on target.

**Memory Management:** We must be mindful of LLM context length. If a session gets very long, we should summarize or truncate. Code Mesh can implement an **auto-summarization** feature: when the token count of history exceeds a threshold, an agent (or a function using an LLM) creates a summary of earlier messages and we drop the raw text of those messages, keeping the summary in memory. This ensures the prompt sent to LLM stays within limits. We can also give users an option to start fresh or refer to past sessions by ID if needed.

By the end of Phase 2, the core functionality will be in place: CLI commands calling into an orchestrator that manages agents which in turn call LLM providers and use file I/O and memory. We will have internal unit tests for each module (e.g. test that `auth login` writes the key file, test that `FileManager.diff` works, test that LLM trait implementations format requests correctly using stub HTTP). Now we proceed to user-facing interface and testing in Phase 3 and beyond.

## Phase 3: Interactive UI, TUI and Command Routing

With the core implemented, we focus on the interactive user experience in the terminal and ensure smooth command routing both in CLI and when invoked via NPX or an IDE.

**3.1 Interactive REPL Mode:** If the user runs `code-mesh` without subcommands, we launch an interactive session (REPL). This behaves similarly to OpenCode’s default mode: Code Mesh loads the project context, greets the user, and waits for input. Implementation details:

* We will likely use a crate like `rustyline` or `crossterm` to handle input editing, history, and perhaps basic text UI features (like multi-line prompts). However, we can start with a simple loop using `std::io::stdin` and print.
* Prompt formatting: It’s helpful to distinguish user vs AI in the terminal. For example, prefix user input with `>>> ` and AI responses with no prefix or a different color. We can incorporate colors via the `colored` crate or similar. We’ll also parse Markdown in the AI’s output for nice formatting if possible (e.g. if the AI responds with markdown code blocks, we can colorize those).
* **Command vs Query:** In interactive mode, if the user types something starting with `/` or `:` (or another prefix), we interpret it as a command to the tool rather than to the AI. For instance:

  * `/exit` to quit the session.
  * `/switch-model openai/gpt-3.5-turbo` to dynamically change the active model.
  * `/history` to display past interactions in this session.
  * `/files` to list loaded project files (maybe even open one to view).
  * `/help` to show available commands.
    These are analogues to how some REPLs work. We’ll route such commands internally (not sending them to LLM). The design will have the main loop check each input; if it matches a known slash-command, handle it directly, otherwise forward to orchestrator as a user message.
* **TUI Enhancements:** In the future (or for advanced contributors concurrently), we can develop a richer text user interface. For example, using the `tui` crate to create a split-screen: the top pane shows the chat, the bottom pane could show the last diff or plan steps. We might hold off on full TUI until core features are stable, but it’s good to design in a way that adding it is straightforward. For now, perhaps implement a simplistic **“plan viewer”**: after the AI outputs a plan (list of tasks), we can number them and as tasks complete we print checkmarks or updates.
* **Example Interaction (textual):**

  ```
  $ code-mesh
  [CodeMesh] Loaded project "weather-app" (3 files, default model: Claude-2)  
  [CodeMesh] Type '/help' for commands. Enter your request.
  >>> Add an endpoint to fetch weather and display it
  [AI:Planner] Plan:
    1. Create '/weather' API route in server.js
    2. Fetch weather from OpenWeatherMap in the route
    3. Add WeatherWidget component in WeatherWidget.jsx
    4. Display WeatherWidget in HomePage.jsx
    5. Write basic tests for the new route
  [CodeMesh] Approve plan? (y/n) 
  ```

  Here CodeMesh presented a plan (via a Planner agent). If user presses `y`, it proceeds with each step:

  ```
  [AI:Coder] (server.js) Added /weather route – DONE  
  [AI:Coder] (WeatherWidget.jsx) Created component – DONE  
  [AI:Coder] (HomePage.jsx) Integrated WeatherWidget – DONE  
  [AI:Tester] Running tests...
  [AI:Tester] Tests passed. All tasks complete.
  [AI:Reviewer] Proposed changes:
    diff --git a/server.js b/server.js
    + app.get('/weather', ... ) {...}
    diff --git a/src/WeatherWidget.jsx b/src/WeatherWidget.jsx
    + function WeatherWidget() { ... }
    ...
  [CodeMesh] Review the diff above. Apply changes? (y/n)
  ```

  If user approves, files are written and maybe committed.

This example demonstrates how **command routing** works: internal commands (`/help`, plan approval prompts) are handled by Code Mesh, whereas normal text is sent to the agent system. We will implement these checks in the main loop.

**3.2 Web/IDE Interface Compatibility:** Because the core is separate, hooking into a web or IDE UI is straightforward:

* **VS Code Extension:** We can create a minimal VSCode extension that launches Code Mesh as a background process or uses the WASM. However, even without writing the extension in this plan, we ensure Code Mesh can operate headlessly with well-defined inputs/outputs. For instance, an IDE could call `code-mesh run` with a prompt and get an output, or use the library to manage a session in memory. The fact that Code Mesh can produce a plan and diff means an IDE could show those in a panel for the user to accept (similar to GitHub Copilot’s experimental “agents” feature).
* **Browser App:** For a pure in-browser scenario, imagine a web app that allows users to load a small project and chat with Code Mesh. Our `wasm-bindgen` interface (from Phase 1) will expose methods like `send_message` and events or callbacks for responses. The web developer can then render the conversation on screen and provide buttons for e.g. applying diff changes by writing them back to the source editor.

We will test a basic browser integration after building the WASM: e.g., create a simple HTML page with an input box and output area, load the `code-mesh.wasm` via a `<script>` and ensure we can call into it. This might be part of our examples/demos rather than core testing, but it’s important to verify the WASM module works outside of Node.

**3.3 NPX Distribution and CLI Routing:** The plan is to distribute Code Mesh via `npx code-mesh`, which means publishing to **npm**. We need to ensure that running `npx code-mesh` just works on user’s machine:

* We will use `npm pkg` **bin** field to provide an executable. For example, in `package.json` of the npm package, we set `"bin": {"code-mesh": "code-mesh.js"}`. The `code-mesh.js` might be a small launcher script.
* **Approach 1: WASM + NodeJS** – We bundle the compiled WASM and a JavaScript runner. The runner script will load the WASM either via `require` (if we target Node) or dynamic import, then call the entry function. We can target Node specifically using `wasm-pack` with `--target nodejs`, which generates a JS file that uses Node’s `fs`/`crypto` to load the .wasm and an initialization function. In this script, we can parse `process.argv` for subcommands just like any Node CLI. However, since we have our CLI logic in Rust (clap), we might instead prefer to compile the entire CLI to a self-contained binary.
* **Approach 2: Precompiled Binaries** – We could compile native binaries for each platform and package them (similar to how some tools distribute via npm, e.g. esbuild). But that requires building and packaging multiple binaries and increases package size. Given our emphasis on WASM, Approach 1 is preferred for simplicity and consistency (one build for all platforms).
* We will likely proceed with a NodeJS-targeted WASM build. That means our `code-mesh-core` compiled with `wasm32-unknown-unknown` + `wasm-bindgen` but expecting to be loaded in Node (so using Node’s APIs for any system access through imported JS functions). There’s also the option of using **WASI** (WebAssembly System Interface) which gives WASM some filesystem and args capabilities in Node. We might consider compiling to `wasm32-wasi` for the Node distribution so it can directly use FS within WASM. The downside is WASI and wasm-bindgen don’t mix well; also WASI would bloat the distribution if we need a runtime. Instead, using wasm-bindgen for Node, we can explicitly import Node’s `fs` module via `wasm-bindgen` if needed (or just handle FS on the Rust side by treating it as external via `cfg` and making syscalls – but easier is to let Rust do nothing special and use Node’s ability to read files by bundling them in memory).
* Our CI/CD will include a step to run `wasm-pack` (or `cargo build --target wasm32-unknown-unknown` followed by `wasm-bindgen`) and then publish to npm. We’ll ensure that the package includes the `.wasm` file and the JS glue, and the `bin` points to a file that invokes it. For example, ruv-swarm advertises `npx ruv-swarm@latest init --claude` as a quick start, meaning their npm is set up similarly. We aim for the same user experience: **no install needed, just one NPX command to use Code Mesh**.
* **Code-mesh Command Routing in NPM:** The wrapper script (code-mesh.js) essentially will do:

  ```js
  #!/usr/bin/env node
  const codeMesh = require('./code_mesh_nodejs.js');  // the wasm-bindgen output
  codeMesh.main();  // assuming we exposed a main() that wraps clap-based CLI
  ```

  If the Rust CLI arg parsing runs inside WASM, it will see `process.argv` automatically. Alternatively, we skip clap in WASM and use JS for argument parsing – but double parsing is unnecessary if Clap works under wasm (with some caveats, but it should for basic args).
  We might need to adjust how output is flushed (WASM writes to stdout might need to be captured and printed by Node).
  These details will be worked out during packaging. We will test `npx code-mesh --help` as a sanity check to see that it prints usage.

**3.4 Concurrent Development Considerations:** The interactive UI and packaging tasks can proceed while core is being built. For example:

* One team can start building the CLI interface with stubbed core logic (Phase 2) and also set up the npm packaging skeleton in parallel.
* Another team focuses on WASM builds and ensuring the example NPX invocation works (possibly using a dummy function).
* This segmentation ensures by the time core logic is ready, the interfaces to users (terminal or NPX) are also ready to integrate.

## Phase 4: Testing, Quality Assurance & CI/CD

Robust testing and continuous integration will ensure Code Mesh’s reliability, especially given the complexity of multi-agent interactions. We outline testing strategies and CI/CD tasks:

**4.1 Unit and Integration Tests:**
Each module will include targeted unit tests:

* **CLI Tests:** Using `assert_cmd` or similar, we can spawn the CLI with various arguments to ensure they parse correctly and call the right core functions. We can stub out actual LLM calls in tests (e.g. by using a dummy LLMProvider implementation that returns predictable outputs). For example, test that `code-mesh run "2+2" -m openai/gpt-3.5-turbo` yields a response (stubbed) and that `auth login` writes a file.
* **LLM Integration Tests:** We will write tests for the LLM trait implementations using mocked HTTP responses. Possibly use the `httpmock` crate or if the API clients have a sandbox mode. Ensure that given a sample prompt, we correctly format the request for OpenAI and parse the response. (Directly hitting real APIs in tests is not ideal due to cost and variability.)
* **Agent Orchestration Tests:** This is trickier to test deterministically, but we can simulate a scenario by injecting a fake LLMProvider that is programmed with expected question->answer mapping. For instance, we feed a “Planner” agent with a prompt and the fake LLM returns a fixed plan string. The test asserts the Orchestrator correctly parsed it into tasks. Similarly, test that if a “Coder” agent is given a task and a fake model returns a code diff text, the orchestrator applies it to the file manager correctly. We will also test the tool interception logic: e.g. if an LLM output contains a `<TOOL>` command, our code should capture it and not send it verbatim to the user.
* **File Operations Tests:** Using a temporary directory (for native), test that `ProjectIO` can scan files, read, write, and diff properly. For WASM, we might use a headless browser test via `wasm-pack test` to simulate adding files and verifying diff logic (or just test the logic as it doesn’t depend on actual OS).
* **Memory Tests:** Simulate a session with multiple messages and test that summarization triggers after N messages, that session saving and loading round-trip correctly, and that the IndexedDB (in a browser test environment) stores and retrieves data. We might need to run a browser environment to fully test IndexedDB, but we can abstract that so that in tests we use an in-memory map to simulate it.

**4.2 End-to-End Testing (Manual & Automated):**
Given the interactive nature, automated E2E tests are challenging. We can do some of the following:

* Write a Python or shell script that uses `npx code-mesh run "Hello"` after building the package, expecting a certain output (maybe using a dummy model or if we allow a special environment variable to use a fake LLM).
* Use snapshot testing for diffs or plans: have known prompts and configure Code Mesh to use a deterministic small model (possibly a local mini model or a recorded fake). Ensure the outputs (like plan and diff) match expected results.
* Beta testing with actual LLMs: At least manually, we will test Code Mesh with OpenAI and Anthropic keys to verify it can complete non-trivial tasks (like a simple “add a function to do X” in a sample project).

**4.3 Performance Testing:**
We should measure memory and speed, especially:

* Cold start overhead (particularly for NPX use, as downloading and initializing WASM should be quick).
* Latency of multi-agent: e.g. ensure that spawning agents doesn’t add too much overhead versus a single agent run. We can instrument timing for each phase (planning, coding, testing) and log it if `--print-logs` flag is set (OpenCode had a `--print-logs` flag to output internal logs which we can adopt).
* We’ll also test behavior with large files or many files to ensure our file context handling scales (perhaps using streaming or chunking if needed for LLM input).

**4.4 Continuous Integration (CI):**
We integrate tests into CI (GitHub Actions or GitLab CI):

* On each push, run `cargo test` for core (on a stable Rust toolchain, and maybe also one older version if we want to support).
* Run `cargo fmt -- --check` and `cargo clippy` to enforce style.
* Build the WASM target and perhaps run `wasm-pack test --headless --firefox` to execute tests in a browser engine for fidelity.
* If all tests pass, we can have CI build artifacts for release:

  * Build the WASM bundle and package it (possibly using `wasm-pack build` which outputs a pkg/ directory with package.json and .wasm).
  * (If we choose binary distribution) cross-compile the binary for Windows, Mac, Linux and attach to GitHub Releases. But likely we stick to WASM to avoid this complexity.

**4.5 Continuous Deployment (CD):**
When ready to publish:

* **Crates.io:** We will publish the `code-mesh-core` crate (and possibly `code-mesh-cli` crate if it’s separate) to crates.io for Rust developers. This allows `cargo install code-mesh-cli` as an alternative to npm. (Note: We should ensure the binary is named `code-mesh` in Cargo so that install yields a `code-mesh` executable).
* **npm:** We will publish the npm package `code-mesh`. We might automate this via CI on git tags. For example, when we push a tag `v1.0.0`, the CI runs `npm publish` with the built package. We must ensure the package version aligns with the crate version for consistency.
* **Versioning and Upgrading:** Document the version in the CLI (`code-mesh --version`). Because we have an `upgrade` command (like OpenCode), we can implement it to simply run `npm install -g code-mesh@latest` or instruct the user to run it. True self-update might not be trivial with WASM, so likely we just point to package managers.

**4.6 Documentation and Examples:**
Testing is also about making sure users know how to use it. We will prepare:

* A comprehensive README documenting installation and usage (with examples of each command).
* Perhaps a quick **“Getting Started”** guide for using Code Mesh on a sample project.
* Ensure to include examples in the repo (like a `examples/` directory with a tiny project and a script demonstrating Code Mesh solving a task on it).

Finally, as part of QA, we consider **safety testing**: Given Code Mesh can run shell commands and modify files, we will implement safeguards (prompt the user before any destructive action, and possibly a config option to disable command execution or internet access by the agent). We will test those safeguards (e.g. if user says “format my disk”, ensure the agent doesn’t actually execute dangerous commands – ideally our tool whitelist prevents it entirely).

By the end of Phase 4, we should have high confidence in Code Mesh’s stability on both native and WASM, and a CI/CD pipeline that can deliver it to users.

## Phase 5: Agent Coordination & Memory Model (Advanced Design Considerations)

This phase can run in parallel to earlier phases (or slightly after core basics) and focuses on the **intelligence layer** – how agents reason, coordinate, and remember, incorporating Jules-style and Claude Code-style behaviors more deeply. This is about refining scope vs. extensibility: some features might be beyond the initial implementation, but we design hooks now to allow adding them.

**5.1 Task Planning and Reasoning (Jules-style):**
We’ve touched on showing plans and using tools; here we solidify how the “task planner” works:

* The Planner agent prompt template might be: *“You are a planning assistant. Your job is to break down the user’s request into a sequence of development tasks, without writing actual code. List the tasks clearly and succinctly.”* This agent would use a robust model (maybe GPT-4 or Gemini) to ensure high-level understanding. If the user declines the plan or modifies it, we allow editing: Code Mesh could accept user input to add/remove tasks from the plan (as Jules allows user to modify the plan before execution).
* The orchestrator then treats this plan as authoritative (unless overridden). Each task description is given to the coding agent(s) in turn.
* If the user skips planning (maybe for small queries, or using a `--no-plan` flag), Code Mesh can either run a default single-agent flow or silently do a quick plan internally.

**5.2 Streaming and Continuous Update (Claude Code-style):**
Claude Code has a mode where it streams a plan and updates it as it discovers new info. We could implement a similar streaming approach:

* When an agent is generating a long answer (like when writing a large block of code), we stream partial output to the terminal. This improves UX so the user isn’t staring at nothing. With `wasm-bindgen`, streaming might require using async streams or a callback since JS expects events – we can adapt by yielding partial output chunks.
* The plan can be dynamic: if mid-way an agent realizes it needs an extra task (e.g. tests failing, so “fix bug X” task added), the orchestrator can insert it into the task list. We need to inform the user of this change (maybe print “\[Plan updated: added task 6: Fix null pointer issue]”). This emergent reprioritization is something Claude Code has been observed to do. By designing the task list as a mutable structure and not assuming it’s static, we allow such updates.

**5.3 Agent Memory & Learning (Ruv-swarm influence):**
We consider how agents can improve over time or share knowledge:

* A simple approach to “learning” is caching results. For example, if an agent had to read a long file and summarize it, we can store that summary in the session cache so subsequent queries don’t call the LLM for it again (saves tokens).
* Another concept: if an agent solved a tricky bug after many steps, we could store that solution in a project knowledge file (e.g. `.code-mesh/knowledge.md`) so that if a similar bug appears, it remembers the fix. This is not fully autonomous learning, but a practical memory.
* Ruv-swarm mentions adaptive agents and evolving neural parameters. While we won’t implement neural network training, we keep extensibility for adding learning modules. For instance, if later we integrate a small online-learning model (perhaps fine-tuning on the fly or a rules engine that picks up patterns), the architecture (with an Orchestrator overseeing agent behavior) can incorporate a feedback loop. We have a placeholder where after each task or session, we can call a `agent.feedback(result)` method – which currently might just log success/failure, but could later update the agent’s strategy.

**5.4 Scope vs Extensibility Clarification:**
At this point, it’s important to delineate what will be implemented in the initial version of Code Mesh, versus what is left as extension:

* **Initial Scope:**

  * Multi-provider LLM support (OpenAI, Anthropic at least) with a single active model at a time (multi-agent sequentially rather than fully parallel).
  * Basic planning and tool usage: The agent can propose a plan and run simple tools (like file operations, tests) with user approval.
  * Interactive CLI with persistent sessions and file modifications (user-approved).
  * WASM module capable of running the above (though possibly with limited tool usage in browser for safety).
* **Not in Initial Scope (Extensibility):**

  * True parallel agent execution (we will sequence tasks for now; concurrency could be experimental behind a flag).
  * Complex topologies or >2 agents simultaneously collaborating in real-time.
  * Cloud VM integration (Jules’s approach of using cloud resources) – Code Mesh will run locally only. However, the design could extend by allowing an agent to offload execution to a remote environment if needed (future concept).
  * Advanced learning/adaptive agents – for now, any improvement is manual (developers updating the logic or model upgrades).
  * Full TUI with multiple windows – initial UI is line-based, we mark the richer TUI as a possible enhancement if time permits or for community contributors to build. The modular design (separating UI) makes this feasible to add later without affecting core logic.

By explicitly listing these, we can focus development effort on core features first, while leaving hooks and documentation for future contributors who might want to implement these advanced features.

**5.5 Example of Extending Code Mesh:**
To illustrate how one might extend Code Mesh beyond the initial scope, consider adding a new LLM provider or a new agent type:

* *Adding a provider:* Suppose a new model **Mistral-7B** is accessible via an API. A developer can add a `MistralClient` implementing `LLMProvider` trait, register it in the provider list. Thanks to our design, the CLI `auth login` will automatically handle it (either through “other” provider option or we update the provider list in config schema). The rest of Code Mesh doesn’t need changes – agents can now call `MistralClient.complete()` if the user selects it. (OpenCode’s flexible config already sets a precedent: it allowed adding custom providers via config JSON, which we emulate).
* *Adding an agent type:* Maybe we want a **DocumentationAgent** that, when code changes are made, generates documentation or comments. We can add a new role and spawn that agent after code is written. Because Orchestrator handles a list of agents generically, adding one more is not disruptive. We might add a hook like `orchestrator.on_all_tasks_done()` where we insert the DocumentationAgent task.
* *Integration with a Knowledge Base:* As an extension, we could integrate with an external doc (like importing MDN or library docs for reference). The design allows an agent to use a “search tool” to fetch documentation (we could implement a `tool.web_search(query)` if internet is accessible or a local docs database). This isn’t in initial scope, but nothing in our architecture precludes it.

## Phase 6: File Persistence and Collaboration

*(Phase 6 is partly an extension of Phase 5 focusing on persistence and multi-user scenarios, which might be relevant if we consider a swarm of devs working with Code Mesh. This phase can run in parallel or after initial release.)*

**6.1 Persistent State & Collaboration:**
We’ve handled local persistence (sessions, config). If multiple developers or a team use Code Mesh on the same project, how do they share state? While not required, we consider:

* Storing session history in the project repo itself (like a `.code-mesh/history.md` file) so others can see what AI did or attempted. This can help avoid repeating questions or understand AI-made changes. We could implement an option to log sessions to a Markdown file in the project.
* Locking mechanism: If Code Mesh is running in a terminal and an IDE plugin simultaneously, or two devs run it, we should avoid clashing edits. Perhaps the config file can include a simple lock or the orchestrator can detect if the working directory is in use by another Code Mesh instance (e.g. by a pid file).

**6.2 CI Integration:**
Another future angle: running Code Mesh in CI (maybe as a GitHub Action) to automatically create PRs from tasks. This would use the non-interactive mode heavily. Our design with `code-mesh run` and possibly a batch mode (maybe `code-mesh plan --execute`) could support this. Though not immediate, it underscores the importance of having a fully functioning CLI without interactive prompts when needed (e.g. providing flags like `--yes` to auto-approve changes for automation).

**6.3 Testing Swarm/Hive Development Approach:**
As a meta step, we could attempt to “dogfood” Code Mesh’s multi-agent capabilities in its own development. For instance, using Code Mesh (with GPT-4, etc.) to generate parts of its code or tests. This isn’t exactly an implementation detail, but a fun outcome of our design enabling multiple contributors (human or AI) to work concurrently. We might document such experiments in the repo.

---

## Conclusion

This implementation plan for Code Mesh is designed to be comprehensive and modular, allowing distributed teams (or even AI agents) to develop it in parallel. We have outlined a clear architecture with separate phases and modules – from core orchestration to CLI and WASM packaging – ensuring that the final product will meet all requirements:

* It **fully replicates OpenCode’s CLI functionality** (project init, interactive chat, run mode, auth management, etc.) and extends it with new commands like `status` and a richer multi-agent workflow.
* It features a **multi-agent “swarm” intelligence**, inspired by state-of-the-art coding assistants: planning tasks before coding (Claude Code’s checklist approach), using tools and executing code (Jules’ tool reasoning), and potentially coordinating multiple specialized agents in a mesh (as envisioned by Ruv-Swarm). These elements combine to create a cooperative hive-mind of coding agents within one tool.
* The solution is **dual-targeted for CLI and browser** – by leveraging Rust’s wasm capabilities and careful abstraction, Code Mesh can run natively or be embedded in web apps/IDEs, with consistent behavior. Distribution via `npx code-mesh` is achieved by publishing a WASM-powered package, making installation frictionless.
* We addressed **persistent memory and context**, storing sessions and project knowledge on disk or IndexedDB as appropriate, so the AI agents have continuity and the user can resume work seamlessly.
* Lastly, the plan emphasizes **extensibility**: new LLMs, new agent types, or UI improvements can be integrated without overhauling the system, thanks to our trait-based design and modular separation. The initial scope focuses on the core experience (to avoid over-complicating the first release), but we have paved the way for future enhancements.

With this plan, Code Mesh development can proceed step-by-step while multiple contributors work concurrently on different components, much like an effective swarm. By following the phases and using the defined interfaces, the end result will be a robust, innovative AI coding assistant that bridges the gap between powerful CLI tools and modern multi-agent intelligence in a portable package.

**Sources:**

* OpenCode CLI usage and commands
* OpenCode multi-provider model support
* Claude Code task planning and checklist approach
* Anthropic Claude Code capabilities (codebase awareness, multi-file editing)
* Google Jules asynchronous agent workflow (planning, parallel tasks, diff review)
* Ruv-Swarm cooperative agent orchestration (swarm topologies, mesh integration)
* Ruv-Swarm NPX distribution and WASM portability
