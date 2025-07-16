Thanks. I’ll put together a comprehensive plan to port and rebuild the full opencode system into Rust as a reusable crate and a WASM target, distributed via `npx`. This will cover dual compilation, WASM packaging, crate structuring, and NPX bundling. I’ll update you shortly with the detailed breakdown.


# Plan for Porting **opencode** to Rust

**Overview:** We propose a full reimplementation of **opencode** – an AI coding assistant built for the terminal – in Rust. The goal is to deliver both a native Rust CLI application and a WebAssembly (WASM) build that can run in browsers and be distributed via **npx** (Node.js). This plan outlines how the original architecture (CLI, server, agent, UI) will map to Rust modules, how to structure the Rust crates for dual (native + WASM) targets, what tools/crates to use, and how to handle packaging, testing, and CI/CD.

&#x20;*Figure: Screenshot of the original opencode terminal UI. It features an interactive TUI (Terminal User Interface) with code diffs and an AI assistant prompt.*

## 1. Original Architecture vs Rust Mapping

**Original opencode Architecture:** The existing opencode is described as an *“AI coding agent, built for the terminal”*. It uses a **client/server** design where the core logic can run as a server (potentially remotely or in the background) and the terminal UI acts as a client. The system supports multiple AI “agents” working in parallel on a project, and it integrates with developer tools (e.g. Language Servers) to improve accuracy. Key components include:

* **CLI & TUI:** A responsive terminal UI (text-based) for interacting with the AI agent.
* **Server (Core Engine):** The backend that handles AI model requests, tool integration, and state. In the original, parts of this are implemented in Go/TypeScript.
* **Agent:** Represents an AI coding assistant instance working on a task. Opencode allows multiple agents (possibly separate threads or processes) to collaborate.
* **UI Clients:** The terminal UI is one client, but the architecture is open to other clients (e.g. a mobile or web app) connecting to the same server.

**Rust Port Mapping:** In the Rust implementation, we will mirror this structure with clear module boundaries and use Rust’s strengths (safety, concurrency) for each part:

* **Core Engine (Server logic):** Implement as a **Rust library crate** (e.g. `opencode_core`) handling all core functionalities:

  * **LLM Integration:** Making API calls to providers like Anthropic, OpenAI, etc., using async HTTP (via `reqwest`).
  * **Session & Agent Management:** Managing multiple agent sessions, each with its context (conversation history, loaded project files, etc.).
  * **Tooling Integration:** Loading language server data, running commands or tests as requested by the AI (with appropriate sandboxing or user confirmation).
  * This core will be used by both the CLI and the WASM/web UI, ensuring a single source of truth for logic.

* **CLI (Terminal UI) Client:** A **binary crate** (e.g. `opencode_cli`) for the native CLI:

  * Uses a text-based UI library (`ratatui` or similar) to render a terminal interface (windows, panels, text input, etc.).
  * Communicates with the core engine. This could be via direct function calls (if embedded in the same process) or via an IPC/HTTP API if we mimic a client/server separation.
  * In Rust, we can initially run the core in-process for simplicity. The CLI can instantiate the core engine (or connect to it) and provide an interactive TUI. In the future, the core could optionally run as a separate daemon for remote access, but that can be a configurable feature.
  * The terminal UI will support rich text (syntax highlighting, diffs) and interactive prompts, similar to the original. Rust’s `ratatui` crate (an enhanced fork of tui-rs) works well for this, enabling us to create windows, lists, and text boxes in terminal.

* **WASM/Web Client:** A **WASM target** (e.g. `opencode_web` package) for browser usage:

  * This will provide a web-based UI that corresponds to the CLI’s functionality. It could be a richer UI (with buttons, panels, etc.) or even a web-based terminal emulator. We have two sub-approaches:

    1. **Full Web App UI:** Build a front-end using a Rust web framework like **Yew** or **Leptos**. Yew is “a modern Rust framework for creating multi-threaded front-end web apps with WebAssembly”, and Leptos describes itself as *“a cutting-edge Rust framework for the modern web”* focused on building interactive web apps in Rust. Using such a framework, we can create a single-page application that communicates with the core engine compiled to WASM. The UI might include components to display chat history, code diffs, file explorer, etc., providing a more graphical interface in the browser.
    2. **Web Terminal Emulator:** Alternatively, to replicate the terminal feel, we could embed a web-based terminal (using a JS library like xterm.js) and have the WASM code handle input/output. In fact, there are examples of Rust crates that unify a terminal interface for both native and web: *“workflow-terminal”* is one such crate that *“combining termion and xterm.js into a unified module”* so the same code runs on a real TTY or a browser terminal. Similarly, the 2048 game demo runs in a text-mode UI on both CLI and web by sharing logic and using xterm.js for browser display.
  * **Core Integration:** The core engine will be compiled to WebAssembly (via `wasm32-unknown-unknown` target). We will expose an interface for the UI to interact with (e.g. using `wasm-bindgen` to export functions that the JS or Yew app can call). For example, functions like `start_new_agent(...)`, `send_message(agent_id, user_input)`, `get_agent_state(agent_id)` can be exposed for the web UI to call. The web UI (written in Rust or TS) will call these to drive the core.
  * **Networking:** In the browser, direct HTTP calls from Rust code are possible. The `reqwest` crate automatically uses a WASM-compatible implementation when compiled for `wasm32` (using the Fetch API under the hood). This means our core can still use `reqwest` to call AI APIs, and on WASM it will translate to browser fetch calls (though certain features like custom TLS settings or cookie stores are disabled on WASM).
  * **Threads:** WebAssembly in browsers currently runs on a single thread (unless using experimental threading support). Our core engine will use **async Rust** (futures) to manage concurrency rather than OS threads, so it should work in the single-threaded browser environment. We will avoid any blocking calls or use of thread-based parallelism in the WASM build. (If needed, Web Workers could be used for parallelism, but that adds complexity.)

* **Agent Abstraction:** In Rust, we will create an `Agent` struct (or module) within the core representing each AI assistant instance. This will encapsulate:

  * The conversation state (messages history, proposed code changes, etc.).
  * A reference to project context (files, possibly an in-memory representation of the user’s codebase or an interface to the filesystem).
  * Methods for the agent to perform actions: e.g. generating code, modifying files (with user approval), running tests, etc.
  * **Multi-agent Support:** The core can manage multiple `Agent` instances, possibly running them concurrently via async tasks (using Tokio). This maps to opencode’s ability to have *“multiple agents working in parallel on the same project”*. In a Rust CLI, this could mean spawning multiple tasks or threads, each with its own agent (for example, one agent working on documentation, another on a coding task). We’ll use asynchronous programming to interleave their operations and a mechanism in the UI to switch view/focus between agents.
  * **Inter-agent Communication:** If needed (for collaboration between agents), we can use channels or shared state. But initially, agents might work independently unless the user explicitly coordinates them.

* **Remote/Server Mode:** While initially the Rust implementation can run the core and UI in one process, we will design it such that a **server mode** is possible. For instance, we might include a feature to run `opencode_core` as an HTTP/WebSocket server (using **Tokio** and perhaps **axum** or **warp** for an API). Then the CLI or web UI could connect to it remotely. This preserves the original’s client/server capability for advanced use cases (like driving the tool from a mobile app). However, this mode can be optional or a later phase; the primary focus is to get the integrated CLI and web versions working first.

**Summary Mapping:** To summarize, the **Rust architecture** will be organized as follows:

* *Module 1:* **Core Engine** (`opencode_core` crate) – Contains business logic, agent management, LLM API integration, etc. Compiles to:

  * a Rust library (for native, used by CLI),
  * and a WASM module (for web, via wasm-bindgen).
* *Module 2:* **CLI/TUI** (`opencode_cli` crate) – Terminal interface using `ratatui` (and indirectly `crossterm` for terminal control). This is a binary that depends on `opencode_core`.
* *Module 3:* **Web UI** (`opencode_web` crate or a part of core behind feature flags) – Browser-based UI. If using Yew/Leptos, this will be primarily client-side code (possibly with some hydration/server-side rendering options for future). It will interface with the core via wasm-bindgen exports. This can be built as a separate package for npm distribution.
* *Module 4:* **Common Utilities & Models:** We might create sub-modules or internal crates for things like:

  * Data structures (e.g. the opencode JSON config parser using Serde, shared request/response structs if any, etc.),
  * Provider API abstractions (for different LLM providers),
  * This can live inside `opencode_core` or be separate if needed for cleanliness.

This separation into crates aligns with keeping concerns modular and allows building only what’s needed for each target.

## 2. Rust Crate Layout for Dual Native/WASM Targets

We will use a **Cargo workspace** to organize the project, enabling multiple crates that share code. A possible layout:

```
opencode-rust/  (Cargo workspace root)
├── Cargo.toml  (workspace members listed)
├── opencode_core/       (Library crate for core engine logic)
├── opencode_cli/        (Binary crate for CLI, depends on opencode_core)
├── opencode_web/        (WASM/web crate for browser, depends on opencode_core)
└── opencode_shared/ ?   (Optional: for shared types, if needed)
```

**opencode\_core (Library):**

* Will be the heart of the application. Mark this crate as `[lib] crate-type = ["rlib", "cdylib"]` in Cargo.toml. This way, we can produce a standard Rust library (`rlib`) for native and a C-compatible dynamic library (`cdylib`) for WASM (the `wasm-bindgen` tool will consume the cdylib to produce .wasm + JS glue).

* This crate will contain most of the code: agent struct implementation, AI provider clients, config management, etc. It will have no direct UI code, which ensures it’s portable across environments.

* **Conditional Compilation:** To support both native and WASM, `opencode_core` will use conditional compilation for any system-specific calls. For example, file system access (used when the agent reads/writes project files) uses Rust’s `std::fs` on native, but on WASM (browser) there is no real filesystem. We’ll abstract file access behind traits or feature flags:

  * We might create a trait `ProjectIO` with methods to read/write files or list directory contents. On native, implement it with actual FS calls; in the web, implement it with a stub or using browser APIs (if running in browser context, perhaps use a virtual filesystem or require the user to grant access via file picker).
  * Use `#[cfg(target_arch = "wasm32")]` and `#[cfg(not(target_arch = "wasm32"))]` to compile the appropriate implementation. This way the core crate compiles on both targets without errors.

* Similarly, if any subprocess execution or OS-specific functionality is needed (like running tests via a shell), we will guard or adapt those for WASM (which likely cannot spawn processes; such features might be disabled or require a connected server).

* **Feature Flags:** We will define Cargo features to include or exclude certain functionality:

  * e.g. a feature `cli` that enables anything requiring native OS (like using language server via IPC, or certain heavy dependencies that can’t work in WASM).
  * a feature `web` for things needed only in browser (if any).
  * The Leptos framework demonstrates this approach: in a full-stack app, they mark server-only deps (like databases) as optional and activate them only for server builds. We will do similar: for instance, if we use `ratatui` or `crossterm` in core for some reason (though we likely keep it in cli crate), ensure those are behind a non-WASM feature.
  * Example: *“sqlx here is labeled as optional because we don't want sqlx to be compiled to WASM… we enable it only for the server feature”*. By analogy, we might mark certain heavy crates as optional and use features to include them only when building the native CLI. This keeps the WASM bundle lean and free of unusable code.

**opencode\_cli (Binary):**

* Small crate whose `main.rs` initializes the TUI. It will:

  1. Parse command-line arguments (we can use `clap` or similar for commands like `opencode auth login`, etc., replicating opencode’s CLI commands).
  2. Initialize the `opencode_core` (e.g. load configuration, start the engine, possibly spawn background thread for core’s server if needed).
  3. Initialize the Terminal UI (set up the terminal in raw mode, create UI layout with Ratatui).
  4. Enter an event loop to handle user input and display output:

     * It will listen for keypresses (using `crossterm` events via Ratatui) and for messages from the core (e.g. agent producing new output).
     * We can use **Tokio** in the CLI as well, to run the core’s async tasks. Tokio can drive an async event loop that the core uses (for HTTP requests etc.). The TUI drawing can either be done in sync manner or also periodically in an async task. Ratatui typically provides a way to draw frames at intervals or on events.
     * Alternatively, use a separate thread for UI vs core (Tokio runtime in one and UI event loop in main thread, communicating via channels).

  * **UI Layout:** The CLI will closely mimic the original: likely a main panel for conversation, a code diff panel, etc., styled according to themes. Ratatui allows styling with colors and can handle resizing.
  * **Agents UI:** If multiple agents are active, the CLI might allow splitting the window or toggling which agent’s view is shown. (For MVP, we might allow one active agent at a time with the ability to switch context via a keybind.)
  * This crate is **only built for native (target != wasm32)**. We mark it accordingly (for example, in Cargo.toml, specify `#[cfg(not(target_arch = "wasm32"))]` for the main, or simply do not compile this crate in a wasm build process).

**opencode\_web (WASM/Browser UI):**

* This crate (or crate target) is responsible for building the web app. Two potential implementations:

  * **Option A: Yew or Leptos App:** We create a `wasm_bindgen`-powered app in Rust. For example, using Yew’s function components or Leptos’ reactive components to build the UI. The app could have similar components: e.g. a text area or code editor component for the code diffs, a scrolling list for chat messages, input box for user prompts, etc. We’d use `wasm-bindgen` and `web-sys` under the hood via these frameworks to manipulate the DOM.

    * We would need to integrate with the core engine. The core (compiled to WASM) could be included in the same WASM binary, or we split it out. Likely, we compile `opencode_core` to WASM and call its functions directly in the same wasm module (since Rust can produce one .wasm that includes both core and UI code if in one crate or if linked together). If `opencode_web` is separate, it can depend on `opencode_core` as a Rust dependency – when compiling to WASM, cargo/wasm-bindgen will include the core code into the output .wasm.
    * Yew applications typically have a `main` function that calls `yew::Renderer::<App>::new().render()` to mount the app to the DOM. We will implement such an entrypoint. We ensure this only compiles for wasm32 target.
    * We will leverage `wasm-bindgen` features for things like JavaScript interop if needed (for example, to use localStorage for config, or to call alert, etc.).
  * **Option B: Minimal JS + Rust Core:** Another approach is to write a minimal HTML/JS interface that loads the WASM core and presents a text-based interface (for example, a simple web terminal). This is less polished, but faster to implement. However, given the requirement of a “full-featured WebAssembly build”, we lean toward using a Rust web framework (Option A) for a richer UI.
* **Packaging for Browser:** We will use `wasm-pack` to build this for distribution. Likely we’ll run:

  ```bash
  wasm-pack build opencode_web --target bundler --out-dir pkg
  ```

  This produces a `pkg/` directory with `opencode_web_bg.wasm`, and JS files (maybe `opencode_web.js` and `.d.ts` etc.), plus a `package.json`. The bundler target generates ESModule glue code, which can be used with modern build tools or directly via import. We can then publish this as an npm package (see Section 4 on NPX packaging for details).
* **NPM vs. Static Hosting:** The WASM build can serve two purposes:

  1. It can be hosted on a website (e.g. GitHub Pages or the opencode website) for users to run in-browser.
  2. It can be installed via npm for use with Node or served locally. We plan to support the `npx` use-case which effectively means using the npm distribution to run the tool (discussed more below).

In summary, the Rust crate layout will separate core logic from the UI implementations. This allows us to compile the core for both targets easily and to ensure that UI-specific dependencies (Ratatui vs Yew, etc.) do not conflict. The approach follows the pattern used by other projects that target both CLI and Web: for example, the **wasm-terminal-2048** project encapsulated game logic in a library and had two frontends (one using Termion for CLI and one using xterm.js for web) sharing that library. We will do the same for opencode’s coding agent logic and its UIs.

## 3. WebAssembly Support and Tooling

Porting to WebAssembly requires careful use of the Rust -> WASM toolchain:

* **wasm-bindgen:** We will use the `wasm-bindgen` crate and CLI to interface between Rust and JS. In the core engine (and/or web UI crate), we will mark functions we need to call from JS with `#[wasm_bindgen]` exports. For example:

  ```rust
  #[wasm_bindgen]
  pub fn initialize_agent(name: &str) -> Result<AgentHandle, JsValue> { ... }
  ```

  This would expose a `initialize_agent` function to JavaScript. Conversely, we can use `#[wasm_bindgen] extern "C"` to import browser JS functions if needed (like alert or accessing DOM, though a framework like Yew handles DOM for us).

* **wasm-pack:** Our build pipeline will rely on **wasm-pack** to simplify building and bundling for npm. Running `wasm-pack build` does the following: *“compiling your code to wasm and generating a pkg folder with the .wasm binary, a JS wrapper, README, and a package.json”* ready to publish. We will:

  * Use `--target bundler` for compatibility. The bundler target outputs ES6 modules, which can be consumed by bundlers like Webpack or directly by Node’s ESM loader. (Alternatively, `--target nodejs` could be used to output CommonJS modules; however, using the bundler/ESM target is more versatile and is the approach recommended for dual environment support.)
  * Ensure the `package.json` generated has appropriate fields (name, version, etc.). We might need to tweak it (via `wasm-pack` configuration or manual edits) to define a `bin` entry for NPX (see NPX packaging below).
  * The output package will be published to npm, allowing `npx opencode-rust` usage.

* **wasm32-unknown-unknown vs wasm32-wasi:** We will target **wasm32-unknown-unknown** (the typical target for web) for the browser build, using wasm-bindgen. This is because we need to run in a browser (which doesn’t support WASI natively). The npm package will thus contain a `.wasm` meant for the browser/JS environment.

  * We will not target **WASI (wasm32-wasi)** for the browser build, but it’s worth noting an alternative: if we wanted a pure CLI via WASM in Node (without browser), we could compile to WASI and run using Node’s WASI support. However, since we want a single WASM build that covers browser and can be used via npx (in Node), the wasm-bindgen approach is preferable. Node can also instantiate wasm-bindgen modules (especially if we use the bundler/ESM output). We can handle Node-specific needs via conditional logic or small JS glue.

* **wasm-bindgen Futures & Async:** Our core is asynchronous (Tokio). In WASM, we can use the `wasm-bindgen-futures` crate to bridge Rust `Future`->JavaScript `Promise`. For instance, if we expose an async function, we might use `wasm_bindgen_futures::spawn_local` to drive it, or return a `Promise` to JS. The details depend on how we structure calls. Frameworks like Yew manage their own async messages, but if using our own JS glue, we may write something like:

  ```rust
  #[wasm_bindgen]
  pub async fn send_prompt(agent_id: u32, prompt: String) -> Result<(), JsValue> {
      core.send_prompt(agent_id, &prompt).await.map_err(|e| JsValue::from_str(&e.to_string()))
  }
  ```

  With wasm-bindgen, an `async fn` like above will automatically be transformed into a JS function that returns a Promise.

* **Tokio on WASM:** Tokio can compile to wasm32, but it cannot spawn real threads or perform blocking. We will use `tokio::main` or `tokio::runtime` on native. On WASM, we will likely not call Tokio’s multi-threaded runtime; instead, we rely on single-threaded async and the browser event loop. One strategy is to use **feature flags** to adjust Tokio for WASM. For example, compile Tokio with the `"current-thread"` scheduler only. Another approach: use `wasm-bindgen-futures` to poll futures instead of a full Tokio runtime. We might run the core’s futures by chaining them into the UI’s event system (if using Yew, spawn tasks using Yew’s scheduler).

  * We expect to use **reqwest** which is async. As noted, reqwest on wasm uses fetch internally, and it requires running on an async context. This will work as long as we ensure to `.await` those futures from an async function ultimately tied to the browser event loop (for example, Yew’s callbacks or using spawn\_local).
  * We will test that the providers API calls work in a browser context (CORS might be an issue if calling Anthropic/OpenAI directly from browser – possibly requiring proxy or appropriate CORS headers; an **alternate approach** for web might be to have a lightweight proxy or encourage user to run the core in server mode locally for full functionality in web, for security reasons. This can be documented.)

* **Serialization (Serde):** The core will use `serde` for reading config files (like `opencode.json`), and for any data structures that need to be saved or communicated. Serde works on WASM as well, since it’s pure Rust. We might send data to browser in JSON via JS if needed (or directly pass into JS as structured data).

* **UI Crates on WASM:** Both **Yew** and **Leptos** compile to WASM via wasm-bindgen and use web-sys internally for DOM. We should choose one:

  * **Yew** is mature, component-based (React-like). It’s a good choice if we want to build a dynamic web UI in Rust. We’d structure the UI into components (ChatWindow, CodeDiffView, InputBox, etc.) and manage state via message passing or hooks.
  * **Leptos** is newer but focuses on fine-grained reactivity. It’s also suitable and has an advantage if we consider server-side rendering later. Since opencode’s web UI might not need SSR, either is fine. Leptos could allow writing some logic that runs on both client and server (they advertise the ability to reuse code on both sides), but that might be overkill for our use-case.
  * We will mention using either “Yew or Leptos” so we have flexibility. The plan can cite both as options. The final decision can depend on team familiarity. (Using Leptos might align with *SST* (the maintainers) preferences, as SST is a modern company possibly interested in latest tools.)
  * Regardless of framework, the build process remains: compile Rust to WASM, produce JS bundle.

* **Static Assets:** We’ll need to supply an HTML file for the web app (to host the script and provide a container for the app). If using `wasm-pack`, the output is meant to be consumed by bundlers, so we might write a small JS/HTML bootstrap. If using **Trunk** (a Rust web dev tool), it could handle it, but since we want an npm package, we’ll do it manually or via a template.

In summary, WebAssembly support will be achieved through **wasm-bindgen** and **wasm-pack**, producing an npm-packaged module that can run in browsers. By structuring the code with conditional compilation and careful use of async, we can use a single Rust codebase for both environments. Many modern Rust libraries have builtin WASM support (as seen with reqwest’s automatic adaptation), so we will leverage those capabilities.

## 4. NPX Distribution and npm Packaging

One deliverable is a “WASM-based tool via npx”. This means a user can run `npx opencode-rust` (or similar) without prior installation, and get the CLI experience. Under the hood, npx will download our npm package and execute the defined binary entry point. Achieving this involves:

* **npm Package Creation:** Using `wasm-pack` makes an npm package in the `pkg/` dir. We will likely name the package something like `"opencode-rust"` or `"opencode-ai-rust"` (to avoid clashing with the existing opencode package name). The `package.json` will be generated with fields for module/main. We will edit/add a `"bin"` field to specify a command-line executable. For example:

  ```json
  "name": "opencode-ai-rust",
  "version": "0.1.0",
  "bin": {
    "opencode-rust": "cli.js"
  },
  "module": "opencode_web.js",
  "files": [ "...", "opencode_web_bg.wasm", ... ]
  ```

  This indicates that when someone runs `npx opencode-rust`, it will execute `cli.js` from our package as the script.

* **CLI Launcher Script:** We will create a small NodeJS launcher (perhaps the `cli.js` mentioned). This script’s job is to load our WASM and invoke the appropriate function:

  * It will import or require the bundled JS module (the one produced by wasm-bindgen, e.g. `opencode_web.js`). This module, when imported, will in turn load the `.wasm` file (either via fetch in browser or fs in Node, handled by wasm-bindgen’s glue).
  * We might need to use dynamic import (`import()` in an async function) if using ESM, or if CommonJS, use `require` and then wait for the module’s promise (in wasm-pack’s output, the ESM import returns a promise if instantiation is async).
  * Once loaded, we call an exported function that starts the CLI. We will have to **export a special function** from Rust, say `run_cli()`, that essentially initializes the engine and runs the REPL loop. For the web build, we might not normally include TUI code, but for Node usage we *do* want to run a terminal interface. How to reconcile this?

    * **Option 1:** Use the same WASM (which was built for browser UI) to also run a simplified CLI in Node. But our WASM binary likely doesn’t include Ratatui (since that’s not compilable to wasm32-unknown-unknown). So perhaps the WASM binary can only support a very minimal text interaction in Node.
    * **Option 2:** Provide a separate WASM build for Node (WASI or otherwise). However, the user specifically asked for a “WASM-based tool via npx”, implying a single build.
    * **Option 3:** Use the web UI even for npx: i.e., when `npx opencode-rust` runs, it could actually spin up a local web server and open a browser for the UI, instead of using terminal. But that might not be what users expect from npx (they likely expect a terminal tool).
    * Given constraints, we likely plan to support npx in a **headless** manner initially: `npx opencode-rust` will run a Node script that loads the WASM core and perhaps uses basic stdin/stdout to interact. This would not have the full TUI (no curses), but could allow simple Q\&A. Alternatively, the Node script could open the browser UI (print a URL or auto-launch a browser pointing to a locally served file with the WASM).
    * For completeness, we describe the simpler approach: **using Node to run the WASM core** with a minimal interface. Node’s WebAssembly support can read from stdin/out. If our Rust core has a mode to run a single agent with text prompts from stdin and print replies to stdout, this could work for npx scenario. We might implement a basic loop in the Node launcher: read user input (from process.stdin), pass it to a WASM-exported `process_input()` function, and print the result.
    * This approach gives a fallback CLI for environments where you can’t or don’t want to install the native binary. It won’t have full TUI (no fancy text UI), but still provides functionality. We will note that the rich TUI is available via the native binary, while the npx/wasm version might be simplified if full terminal control isn’t feasible in Node WASM.
  * If we find it feasible to compile some kind of text UI to WASM (e.g., the workflow-terminal crate shows it’s possible to unify a terminal interface by coupling with xterm.js for web, but in pure Node context, manipulating the real terminal via WASM might be limited), we will attempt it. There are experiments like running curses in WASM with WASI, but given the scope, the safe route is the simplified approach above.

* **wasm-pack bundler target for Node:** The bundler target outputs ES modules. Modern Node (v14+) can run ES modules if the package.json has `"type": "module"` or by dynamic import. We can configure the package for Node usage by possibly providing a dual import:

  * We could use the `exports` field in package.json to specify different entry points for browser vs Node. E.g., `"exports": {"import": "./opencode_web.js", "require": "./node-cjs.js"}` or similar. But this might be overkill.
  * Another simpler way: use `--target nodejs` for the build intended for npx. That would produce CommonJS output which is straightforward to require in the launcher script. Actually, we might consider running **two** wasm-pack builds:

    1. One with `--target bundler` for browser usage (to publish for web bundlers or for future integration).
    2. One with `--target nodejs` for the npx package specifically, giving us a package optimized for Node.
  * However, maintaining two packages is not ideal. It might be simpler to just use one package and ensure it works in Node. The `bundler` output can work in Node if we load it as an ES module. We can test that.
  * We’ll document that we use `wasm-pack build --target bundler` as requested, but also ensure the npx scenario is handled by our Node wrapper.

* **Publishing Process:** Steps to publish:

  1. Ensure the `pkg/` contains the final files (`.wasm`, `.js`, our custom `cli.js` if added, README, package.json).
  2. Login to npm (one-time setup). Then run `npm publish` in the pkg directory. (Alternatively, use `wasm-pack publish`, which automates build + publish; *“wasm-pack publish… will upload the package to npm”*).
  3. We will integrate this into CI (see section 6). Possibly set up an automated publish on new Git tag if credentials are available, or do it manually for releases.
  4. Verify by running `npx opencode-ai-rust@latest` on various systems.

* **Binary Name:** Note that original opencode installs a binary named `opencode`. To avoid conflict, our npm might call it `opencode-rs` or similar. We can still allow `opencode` as the command if the original npm package isn’t installed. We’ll likely name the npm package distinctly, to avoid confusion with the original JavaScript-based one (if it exists on npm). For this plan, we’ll use **`opencode-ai-rust`** as a tentative name, yielding an `npx opencode-ai-rust` command.

* **Example – Tailwind CSS CLI in Rust via npx:** As a reference, the **tailwindcss-to-rust** tool (a Rust CLI) can be installed via npm/npx, which simply downloads the Rust binary or uses a Node wrapper. Our approach is slightly different in that we package a WASM instead of native binaries (to avoid per-OS binaries in npm). However, using WASM in Node is increasingly feasible as WASM support matures. Users get the advantage of a smaller download (one .wasm \~ a few MB, instead of platform-specific binaries possibly bigger).

* **Post-install Considerations:** We should note that if performance is critical, the native Rust binary will run faster than WASM on Node (due to JIT and possible WASM limitations). But as an AI assistant, the heavy work is calling external APIs, so performance is fine. We’ll document that the npm/npx method is primarily for convenience, and power users can install the native binary (from crates.io or Homebrew) for best experience (especially for full TUI support).

In summary, NPX support will be achieved by publishing an npm package containing our WebAssembly build and providing a NodeJS launcher script that invokes the Rust core. This allows immediate usage: **“npx opencode-rust”** can fetch the package and run the tool. We will leverage `wasm-pack` to streamline creation of this package and ensure it’s properly configured for Node. The packaging process will include testing this flow on CI to guarantee that the user experience is smooth.

## 5. Code Reuse and Conditional Compilation Strategy

To maximize code reuse between native and web versions, we will use Rust’s conditional compilation and feature flags extensively:

* **Shared Core Logic:** The `opencode_core` crate is the single source of truth. All critical logic (prompt handling, AI API calls, config parsing, etc.) will reside here and be used by both the CLI and WASM builds. We avoid duplicating logic in two places. This follows the DRY principle and ensures consistency between the CLI and web experiences. The 2048 example explicitly *“encapsulated game logic in a library that can be shared with different front-ends”* – we will do exactly that for opencode’s logic.

* **Conditional API/OS calls:** Where the core does need to do something that only makes sense on one platform, we’ll isolate it. For instance:

  * File system or shell execution – only for native. We might put such functions behind `#[cfg(not(target_arch = "wasm32"))]`. If a WASM call tries to use them, it will either be a no-op or return an error indicating “not supported in web mode”.
  * Use of environment variables (for provider API keys) – Node can provide env vars, but in browser we have to gather keys via UI input. We will design `auth login` flows accordingly. Possibly in web UI, user pastes API keys which we store in localStorage instead of reading ENV. The core can offer an abstraction like `CredentialStore` that the CLI implements via env file (\~/.local/share/opencode) and the web implements via browser storage.
  * Networking differences – largely handled by reqwest internally, but if something like streaming responses (Server-Sent Events for LLM) is used, we must ensure it’s supported in browser (likely yes via fetch streams).

* **Feature Flags for Dependencies:** We will avoid pulling in unnecessary crates for a given target by using Cargo features:

  * For example, if we include a terminal UI helper in core (like maybe using `ratatui` for rendering diff output in text form), we’d feature-gate it for CLI only.
  * The Leptos example in an admin app highlights using features “hydrate” vs “ssr” to include certain deps only on client or server. We can do similar:

    * Define feature `"browser-ui"` that enables integration with web-specific crates (perhaps not needed if web UI is separate crate).
    * Feature `"native-cli"` to include things like `which`, `notify` (for FS watch) etc. only on native.
    * By default, `opencode_core` could enable all, but when building for WASM we pass `--no-default-features --features browser-ui` to exclude native stuff.
  * This keeps the compiled WASM small and free of unused code (important for web load times).

* **Example of optional dependency:** If in `Cargo.toml` of core we had:

  ```toml
  [dependencies]
  ratatui = { version = "0.x", optional = true }
  ```

  and in `[features]`:

  ```toml
  default = ["native-cli"]  
  native-cli = ["ratatui", "tokio"]  
  browser = []  # browser might not need extra deps besides default-core ones
  ```

  Then, when compiling for WASM, we use `--no-default-features --features browser`. This ensures `ratatui` and even `tokio` (if we decide to not use Tokio on web) are not pulled in. As noted in a real-world case, *“sqlx is optional so it’s not compiled to WASM… enabled only for server feature”*; we will follow that pattern for any similar crates.

* **Testing Shared Code:** By having one core crate, we can write unit tests for the logic that run in both environments. We might use `#[cfg(test)]` and run them under `cargo test` (native) and `wasm-pack test` (web, headless). This ensures our core logic behaves the same regardless of platform.

* **Documentation and Examples:** We can write examples in the core crate that demonstrate usage. For instance, an example could show a minimal program using `opencode_core` to send a prompt and print a reply. We can compile that example for native easily; for WASM, we might have a separate example or test. The goal is to validate that the core API is ergonomic and consistent.

* **Edge Cases:** Some things cannot be truly unified:

  * The actual UI implementation is separate (which is fine). We don’t attempt to write one UI code that runs both in terminal and browser, because that would be very complex. Instead, we accept that we have two UI implementations that both call into the same core.
  * The data encoding for sending data to web (if using postMessage or fetch) might require serialization. But since we likely call the core functions directly in WASM, we don’t need a REST API between UI and core in the browser case – it’s just a function call boundary (which is super efficient).

* **Third-Party Integrations:** If opencode uses external tools (like GPT-4 code interpreter, or calls out to git, etc.), we will consider each in terms of target compatibility. Possibly, some integrations won’t work on web (e.g., running a docker container to execute code – not possible in browser). Those will be flagged as “native only” features. Our design allows that: the core can have a component (say, CodeExecutor) available only on native. If a user tries that feature in web, we’ll show a message that it’s unsupported.

In short, our strategy is to **write core logic once** and use Rust’s cfg attributes and features to compile it appropriately for each platform. This approach is proven in community – many Rust projects create cross-platform libraries with minimal target-specific code. By isolating the target-dependent parts and using optional features for target-specific dependencies, we maximize reuse and maintainability.

## 6. Testing Strategy for Native and WASM Targets

Testing will be important to ensure that the Rust port behaves correctly in both environments. Our testing strategy includes:

* **Unit Tests (Core Logic):** We will write extensive unit tests for `opencode_core` functions. For example:

  * Test the parsing of configuration (`opencode.json`).
  * Test that agent message handling (prompt -> internal representation -> formatting of reply) works for various scenarios (including edge cases like empty input or large code).
  * If we have abstraction for LLM API calls, we will **mock** those for tests. We can use dependency injection or feature flags to supply a dummy LLM client that returns canned responses. This way, tests don’t call external APIs and can run offline.
  * These tests run with `cargo test` on native. We also want to run them under a WASM environment:

    * We can use **wasm-bindgen-test** or **wasm-pack test**. wasm-bindgen provides a test framework integration where tests annotated with `#[wasm_bindgen_test]` can run in a headless browser or node environment.
    * We’ll set up to run `wasm-pack test --headless --browser=chrome` for example, which will launch headless Chrome to execute the tests in a browser context. Alternatively, `wasm-pack test --node` to run them under Node’s WASM. This ensures our core logic behaves identically in WASM (catching any subtle differences).
    * Not all tests might be meaningful in WASM (especially ones expecting file system or threads), but we can conditionally ignore those in that context via `#[cfg(not(target_arch = "wasm32"))]`.

* **Integration Tests (CLI):** For the CLI, we will simulate user interactions:

  * One approach is to use snapshot testing for CLI outputs. For example, run `opencode_cli` with a specific prompt input (using a predetermined small model or a stubbed model) and verify the printed output matches expected. This can be done by spawning the CLI binary with certain env variables or a testing mode that uses a dummy AI response.
  * We can use `assert_cmd` or similar crate to run the compiled binary in tests, or use the library interface of core directly.
  * Testing the interactive TUI directly is tricky, but we can test underlying components (like ensure that the text diff generator produces correct diff, ensure that the prompt parsing yields correct tool invocation, etc.). We may also factor logic out of the UI so it’s testable (e.g., have a function that takes a key press event and returns an action or state change, and test that mapping).

* **Integration Tests (Web):** Testing the web UI can be done with headless browser automation:

  * We could utilize frameworks like `wasm-bindgen-test` for simple component tests. Yew has its own testing utilities (you can instantiate components in a headless mode and verify DOM output).
  * Additionally, we might set up a Cypress or Playwright test in CI that builds the web app and runs it in a headless browser to simulate a user session: e.g., load page, input a prompt, and verify that a response element appears. However, this might be beyond MVP – it’s something to consider for later to prevent regressions in the WASM build.
  * At minimum, we will load the built web app in a real browser manually during development to ensure it functions (the CI can catch panics via wasm tests, but full rendering tests might require manual QA or a more complex setup).

* **Cross-Platform Matrix:** Since Rust is cross-platform, we will test on Linux, macOS, and Windows for the native binary:

  * The CLI should be tested on all OS (ensuring that things like terminal resizing, Unicode, etc., work everywhere).
  * We will also verify the WASM in at least Chrome, Firefox, maybe Safari (Safari’s WASM support sometimes lags, so it’s good to test).
  * We include Windows in testing because building a portable CLI is important (maybe using Crossterm which supports Windows console for ratatui).

* **Performance Tests:** Not exactly tests, but we should try out scenarios with large files or long conversations to ensure performance is acceptable. Rust will likely improve performance over the original (Go/TS mix), but the WASM build should be profiled to ensure no significant bottlenecks (like unoptimized JSON handling or too large WASM binary causing slow loads). If needed, we can enable `wee_alloc` as a global allocator for WASM to reduce binary size, etc., but only if memory is an issue.

* **Security/Behavior Tests:** If the agent can execute code or modify files, we need tests for safety:

  * e.g., given a prompt “delete all files”, ensure the agent doesn’t execute dangerous operations without user confirmation.
  * Test that the sandboxing or confirmation logic works (maybe stub user approval responses).

* **Continuous Integration:** We will configure CI (GitHub Actions or GitLab CI, etc.) to run all the above tests. A typical matrix:

  * Build and run tests on Ubuntu (latest stable Rust).
  * Build and run tests on Windows, Mac.
  * Run `wasm-pack test --node` (Node environment) to test core on WASM.
  * Possibly run `wasm-pack test --browser` with headless Chrome (there is a GitHub Action for that or we can use `wasm-bindgen-test-runner`).
  * Having these in CI ensures that we don’t accidentally break the WASM compatibility as development continues.

* **Manual testing for NPX:** We will have a step in CI (maybe on release pre-publish) that does:

  * `npm pack` the package and then `npx <path-to-packed.tgz> --help` to see if it runs. This can catch if our bin script is working. We might do this on a Linux runner with Node installed.
  * Ensure the `.wasm` can be loaded (maybe Node needs some flag for WASM threads if we ever use them, but likely not, so no flags needed).

By having a comprehensive test suite run in both native and WASM contexts, we can be confident in the reliability of the port. The approach of testing core logic in isolation, as well as doing end-to-end usage tests, will cover both functional correctness and integration of components.

## 7. CI/CD and Release Workflow

To streamline development and releases, we will set up continuous integration (CI) and continuous deployment (CD) pipelines:

* **Version Control:** The project will be hosted on GitHub (likely under the sst organization, if that’s where opencode lives). We’ll use GitHub Actions for CI, as it’s well-supported for Rust and cross-platform.

* **CI Builds and Tests:** On each push/PR, run the test matrix described:

  * **Linting/Formatting:** We can add a step using `cargo fmt --check` and `cargo clippy --all-features -- -D warnings` to enforce code style and catch common mistakes.
  * **Unit/Integration Tests:** As above, test on multiple OS and WASM.
  * **Build Artifacts:** We might have CI produce binary artifacts for releases (for users who want to download without using cargo or npm). For example, using the **Cross** tool or Actions matrix to build Linux x86\_64, macOS, Windows executables and attach to GitHub Releases.
  * We will also ensure that the web build compiles on CI (run `wasm-pack build` as a test, even if we don’t use the output). This catches any compilation errors for WASM target early.

* **CD – Publishing to Crates.io:** When we are ready for a release, we will publish the Rust crate:

  * Since we have multiple crates, we likely publish at least `opencode_core` (library) and `opencode_cli` (binary). Or we might choose to publish a single combined crate that produces the CLI binary (many CLI tools are published on crates.io for `cargo install`).
  * If `opencode_cli` is the main entry, we publish that with a `[package] name = "opencode"` (if available) or `opencode-cli` if naming conflict. That crate’s Cargo.toml will include `[[bin]] name="opencode"` so that `cargo install opencode` gives the `opencode` binary.
  * We will set up manual or automated publishing: possibly using a GitHub Action that triggers on a new Git tag. There’s an action to publish to crates.io given an API token (which we’d store in repo secrets). This action would run `cargo publish` for relevant crates in correct order (core first, then CLI).
  * Ensuring version numbers are bumped in sync (maybe using `cargo release` tool to manage).

* **CD – Publishing to npm:** Similar to crates, we want to publish the WASM package to npm on release:

  * We can integrate `wasm-pack` in CI to do this. There is a `wasm-pack action` for GitHub, or we can just run commands. Steps:

    1. CI job checks out code, installs Rust and npm.
    2. Build the WASM package: e.g. `wasm-pack build opencode_web --target bundler --release`.
    3. Navigate to `pkg/`, adjust package.json if needed (though ideally our repository has it configured or includes a template so minimal change needed).
    4. Use `npm publish`. We’ll need an npm token in CI secrets. Alternatively, `wasm-pack publish` might handle the build+publish in one step.
  * We will likely set this to run when a Git tag is made (for example, tag v1.0.0 triggers publish of crates.io and npm). We must ensure the version in Cargo.toml and package.json are aligned and match the tag.
  * Testing the published package: We might do a dry-run (there’s `npm publish --dry-run`) on CI just to show what would be published.

* **Homebrew and Other Distribution:** The original opencode provides a Homebrew tap and other methods. For our Rust version, once it’s stable, we can update those distribution channels:

  * We can add a job to update the Homebrew formula (perhaps automatically via `brew bump-formula`). This could be done in a separate pipeline, though, and might not be immediate priority.
  * Likewise, the AUR (Arch Linux) could get a -rust package. This might be handled by the community, but we can facilitate by providing binary releases.

* **Documentation Deployment:** If we write documentation (e.g. a mdBook or just README usage docs), we could host them. Possibly integrate with GitHub Pages. However, the user’s request is mainly about porting plan, so documentation is an ancillary concern. Still, we might generate Rustdoc for the library and host it (docs.rs automatically does for published crates).

* **Security and Secrets:** We will ensure the CI processes do not expose secrets (npm token, crates.io token). They will only be available on protected branches/tags (e.g., on the main branch when a release is triggered, not on PRs from forks).

* **Monitoring CI:** We will set up badges (like build passing, crates.io version, npm version) in the README for transparency. Automated tests for each PR will ensure code quality.

By implementing the above CI/CD, every code change is validated, and releasing a new version is as simple as tagging a commit (the automation will handle building and publishing to both registries). This reduces manual effort and potential for error when shipping updates.

## 8. Project Timeline

Below is an estimated timeline for the porting project, broken into phases and milestones:

| **Phase**                                          | **Tasks**                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           | **Timeline (Week)** |
| -------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------- |
| **Phase 1:** Planning & Setup                      | - Finalize requirements and design decisions<br>- Set up Rust workspace and initial crate structure (`opencode_core`, `opencode_cli`, etc.)<br>- Establish CI pipeline (build/test for a simple hello-world in both native and WASM to validate toolchain)                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Weeks 1–2           |
| **Phase 2:** Core Engine Implementation            | - Implement config loading (using Serde, reading `opencode.json`)<br>- Implement Provider API clients (Anthropic, OpenAI, etc., using `reqwest` with async)<br>- Implement basic Agent struct and logic for single prompt -> response cycle (stub out complex features with placeholders initially)<br>- **Testing:** Unit tests for config and a dummy agent logic                                                                                                                                                                                                                                                                                                                                                                 | Weeks 3–5           |
| **Phase 3:** CLI (TUI) Development                 | - Integrate `ratatui` to build a text UI layout (windows for chat, code, input)<br>- Hook up user input handling (via `crossterm` events) and output rendering from agent responses<br>- Connect CLI to core: initialize core engine in background (maybe spawn a Tokio runtime thread), send prompts to core and receive outputs (could use channels or direct calls if not multi-threaded)<br>- Implement command handling (e.g. `opencode auth login` flow in terminal, etc.)<br>- **Testing:** Manual testing in terminal, and automated tests of internal event handling (simulate key presses)                                                                                                                                | Weeks 6–8           |
| **Phase 4:** WebAssembly UI Development            | - Choose web framework (e.g. Yew) and set up basic app structure (mounting point, routing if any)<br>- Implement components for displaying chat messages, code diffs (could use syntax highlighting libraries or highlight.js via JS if needed), and an input form for prompts<br>- Expose necessary core functions via `wasm-bindgen` and ensure they can be called from the UI (e.g., when user submits a prompt, call core’s `send_prompt` and await response)<br>- Handle asynchronous updates (e.g. streaming responses, if supported, or final responses updating UI state)<br>- **Testing:** Run in browser to verify functionality; use `wasm-pack test` for core logic on WASM; possibly write component tests if feasible | Weeks 9–12          |
| **Phase 5:** Packaging and NPX Support             | - Create Node launcher script for npx<br>- Configure `wasm-pack` build and package.json for npm distribution (set bin, etc.)<br>- Test `npx opencode-rust` locally on different OS (ensure it downloads package and runs, printing something or launching UI)<br>- Adjust any issues (like Node ESM/CJS interop, file paths for WASM) until smooth                                                                                                                                                                                                                                                                                                                                                                                  | Week 13             |
| **Phase 6:** Comprehensive Testing & Stabilization | - Perform cross-platform testing of CLI (Windows console quirks, etc.)<br>- Write additional tests for multi-agent scenarios, concurrency (if possible simulate two agents)<br>- Security review (ensure no file escapes in web, validate user confirmation for dangerous ops in CLI)<br>- Benchmark common operations to catch any performance issues (especially in WASM) and optimize (e.g. enable release optimizations, tune `serde_json` vs `simd-json` if needed)                                                                                                                                                                                                                                                            | Weeks 14–15         |
| **Phase 7:** Documentation & Community Preview     | - Write usage documentation: how to install (cargo, npm, brew), how to use features, config reference<br>- Provide examples in README (animated GIF of terminal usage, etc.)<br>- Announce a beta release to community (Discord, etc.) for feedback<br>- Address feedback/bugs reported by early users                                                                                                                                                                                                                                                                                                                                                                                                                              | Weeks 16–17         |
| **Phase 8:** CI/CD and Release 1.0                 | - Set up CI workflows for publishing (ensure credentials, test one dry-run)<br>- Increment version, tag release<br>- Publish crate to crates.io and package to npm (either manually or via CI)<br>- Publish Homebrew formula update (if applicable)<br>- Verify installations (try `cargo install opencode`, `npm i -g opencode-rust`, brew install, etc.) work as expected                                                                                                                                                                                                                                                                                                                                                         | Week 18             |

*(The above timeline is an estimate; actual development may vary based on complexity and feedback.)*

## 9. References and Examples

Throughout this plan, we have drawn on experiences from existing projects that bridge Rust, WebAssembly, and cross-platform CLI development:

* The **original opencode** design emphasizes a terminal UI and client-server separation, which guided our Rust architecture to maintain flexibility for both local and remote usage.
* Projects like *wasm-terminal-2048* demonstrate sharing core logic between a native terminal app and a WASM web app, using Rust for both: *“The game logic is encapsulated in the library… play 2048 in either command line (Termion) or browser (xterm.js)”*. We apply the same principle for opencode.
* The **workflow-terminal** crate provides insight on combining terminal libraries for cross-target support, showing it’s possible to unify a text UI for native and web by abstracting over Termion (native) and xterm.js (web). While we chose to implement separate UIs for richness, this inspires our thinking on possibly sharing some UI logic (like how to process user commands) between TUI and web.
* **Yew and Leptos frameworks:** Yew is described as *“a modern Rust framework for… front-end web apps with WebAssembly”* and it achieves high performance with minimal DOM calls. Leptos similarly pitches itself as *“easy to build interactive web applications in Rust”* with cutting-edge reactivity. These reinforce that our choice to use a Rust-based web UI is viable and can deliver a good user experience without writing JavaScript.
* **Reqwest HTTP client:** The fact that reqwest automatically adapts to WASM (using Fetch API) and notes some features disabled guided us to be mindful of what functionality might differ (timeouts, TLS settings) in the web build.
* **Conditional compilation and features:** The snippet from a Leptos app’s Cargo.toml shows how to mark certain dependencies optional for WASM and enable them only on specific builds. We mirror this approach, ensuring that our crate can toggle off unnecessary components for the web build to keep it lightweight and compatible.

By learning from these examples and following Rust best practices, our plan ensures that the ported **opencode** will be robust, efficient, and accessible in both terminal and browser environments. The end result will be a fully open-source, dual-target AI coding assistant that retains opencode’s original vision (provider-agnostic, terminal-focused usage) while adding the safety, performance, and portability benefits of Rust and WebAssembly.
