# Code Mesh WebAssembly Integration Strategy

## Executive Summary

This document outlines the comprehensive WebAssembly compilation strategy and cross-platform integration approach for the Code Mesh project, designed to enable dual-target compilation (native CLI + browser WASM) while maintaining unified functionality.

## 1. wasm-bindgen Integration Strategy

### 1.1 Core Architecture

```rust
// Core module structure for dual-target compilation
// code-mesh-core/src/lib.rs

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct CodeMeshWasm {
    orchestrator: Orchestrator,
    session: Session,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl CodeMeshWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            orchestrator: Orchestrator::new(),
            session: Session::new(),
        }
    }
    
    #[wasm_bindgen]
    pub async fn execute_prompt(&mut self, prompt: &str) -> Result<String, JsValue> {
        self.orchestrator.execute_prompt(prompt)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    
    #[wasm_bindgen]
    pub fn get_status(&self) -> String {
        serde_json::to_string(&self.session.status()).unwrap()
    }
}
```

### 1.2 Conditional Compilation Strategy

```rust
// Conditional compilation for platform-specific features
// code-mesh-core/src/fs.rs

pub trait ProjectFS {
    async fn read_file(&self, path: &str) -> Result<String, Error>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), Error>;
    async fn list_files(&self, path: &str) -> Result<Vec<String>, Error>;
}

#[cfg(not(target_arch = "wasm32"))]
pub struct NativeFS {
    root: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProjectFS for NativeFS {
    async fn read_file(&self, path: &str) -> Result<String, Error> {
        tokio::fs::read_to_string(self.root.join(path)).await
    }
    
    async fn write_file(&self, path: &str, content: &str) -> Result<(), Error> {
        tokio::fs::write(self.root.join(path), content).await
    }
    
    async fn list_files(&self, path: &str) -> Result<Vec<String>, Error> {
        // Native directory listing implementation
        Ok(vec![])
    }
}

#[cfg(target_arch = "wasm32")]
pub struct BrowserFS {
    storage: web_sys::Storage,
}

#[cfg(target_arch = "wasm32")]
impl ProjectFS for BrowserFS {
    async fn read_file(&self, path: &str) -> Result<String, Error> {
        self.storage.get_item(path)
            .map_err(|_| Error::FileNotFound)?
            .ok_or(Error::FileNotFound)
    }
    
    async fn write_file(&self, path: &str, content: &str) -> Result<(), Error> {
        self.storage.set_item(path, content)
            .map_err(|_| Error::WriteError)
    }
    
    async fn list_files(&self, path: &str) -> Result<Vec<String>, Error> {
        // Browser-based file listing using IndexedDB
        Ok(vec![])
    }
}
```

## 2. wasm-pack Configuration Strategy

### 2.1 Cargo.toml Configuration

```toml
# code-mesh-core/Cargo.toml
[package]
name = "code-mesh-core"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }

# WASM-specific dependencies
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "console",
    "Window",
    "Storage",
    "IndexedDb",
    "IdbDatabase",
    "IdbObjectStore",
    "IdbTransaction",
    "IdbKeyRange",
    "IdbCursorWithValue",
] }
js-sys = "0.3"
gloo-timers = { version = "0.2", features = ["futures"] }

# Native-specific dependencies
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
clap = { version = "4.0", features = ["derive"] }
ratatui = "0.24"
crossterm = "0.27"
notify = "6.0"

[features]
default = ["cli"]
cli = []
browser = []
```

### 2.2 Build Scripts and Automation

```bash
#!/bin/bash
# scripts/build-wasm.sh

# Build for browser target
wasm-pack build code-mesh-core \
  --target bundler \
  --out-dir pkg \
  --no-typescript \
  --features browser

# Build for Node.js target
wasm-pack build code-mesh-core \
  --target nodejs \
  --out-dir pkg-node \
  --no-typescript \
  --features browser

# Optimize WASM size
wasm-opt -Oz -o pkg/code_mesh_core_bg.wasm pkg/code_mesh_core_bg.wasm
wasm-opt -Oz -o pkg-node/code_mesh_core_bg.wasm pkg-node/code_mesh_core_bg.wasm
```

## 3. Dependency Management Strategy

### 3.1 Native vs WASM Dependencies

```rust
// code-mesh-core/src/lib.rs

#[cfg(not(target_arch = "wasm32"))]
pub mod native {
    pub use clap;
    pub use ratatui;
    pub use crossterm;
    pub use notify;
    pub use tokio;
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    pub use wasm_bindgen;
    pub use wasm_bindgen_futures;
    pub use web_sys;
    pub use js_sys;
    pub use gloo_timers;
}

// Unified HTTP client that works on both platforms
pub mod http {
    pub use reqwest::Client;
    
    pub fn create_client() -> Client {
        #[cfg(target_arch = "wasm32")]
        {
            Client::builder()
                .build()
                .expect("Failed to create HTTP client")
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client")
        }
    }
}
```

### 3.2 Feature Flag Management

```rust
// code-mesh-core/src/llm.rs

use crate::http::create_client;

pub struct LLMClient {
    client: reqwest::Client,
    provider: String,
}

impl LLMClient {
    pub fn new(provider: &str) -> Self {
        Self {
            client: create_client(),
            provider: provider.to_string(),
        }
    }
    
    pub async fn complete(&self, prompt: &str) -> Result<String, Error> {
        #[cfg(target_arch = "wasm32")]
        {
            // Browser-specific implementation
            self.complete_browser(prompt).await
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native implementation
            self.complete_native(prompt).await
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    async fn complete_browser(&self, prompt: &str) -> Result<String, Error> {
        // Use fetch API with CORS handling
        let response = self.client
            .post(&format!("https://api.{}.com/v1/completions", self.provider))
            .json(&serde_json::json!({
                "prompt": prompt,
                "max_tokens": 1000
            }))
            .send()
            .await?;
            
        Ok(response.text().await?)
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    async fn complete_native(&self, prompt: &str) -> Result<String, Error> {
        // Native implementation with full HTTP client features
        let response = self.client
            .post(&format!("https://api.{}.com/v1/completions", self.provider))
            .header("User-Agent", "code-mesh/0.1.0")
            .json(&serde_json::json!({
                "prompt": prompt,
                "max_tokens": 1000
            }))
            .send()
            .await?;
            
        Ok(response.text().await?)
    }
}
```

## 4. Async/Await Integration Strategy

### 4.1 Tokio vs Browser Event Loop

```rust
// code-mesh-core/src/runtime.rs

#[cfg(not(target_arch = "wasm32"))]
pub fn create_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new()
        .expect("Failed to create Tokio runtime")
}

#[cfg(target_arch = "wasm32")]
pub fn spawn_local<F>(future: F)
where
    F: std::future::Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

// Unified async executor
pub struct AsyncExecutor {
    #[cfg(not(target_arch = "wasm32"))]
    runtime: tokio::runtime::Runtime,
}

impl AsyncExecutor {
    pub fn new() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime: create_runtime(),
        }
    }
    
    pub async fn execute_agents(&self, agents: Vec<Agent>) -> Result<Vec<AgentResult>, Error> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Use Tokio's join_all for parallel execution
            let tasks: Vec<_> = agents.into_iter()
                .map(|agent| self.runtime.spawn(agent.execute()))
                .collect();
                
            let results = futures::future::join_all(tasks).await;
            Ok(results.into_iter().collect::<Result<Vec<_>, _>>()?)
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // Sequential execution in browser (or use web workers for parallelism)
            let mut results = Vec::new();
            for agent in agents {
                results.push(agent.execute().await?);
            }
            Ok(results)
        }
    }
}
```

### 4.2 Cross-Platform Timer Implementation

```rust
// code-mesh-core/src/timer.rs

use std::time::Duration;

pub struct Timer;

impl Timer {
    pub async fn sleep(duration: Duration) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::time::sleep(duration).await;
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32).await;
        }
    }
    
    pub async fn timeout<F, T>(duration: Duration, future: F) -> Result<T, TimeoutError>
    where
        F: std::future::Future<Output = T>,
    {
        #[cfg(not(target_arch = "wasm32"))]
        {
            tokio::time::timeout(duration, future)
                .await
                .map_err(|_| TimeoutError)
        }
        
        #[cfg(target_arch = "wasm32")]
        {
            // Browser timeout implementation
            let timeout_future = gloo_timers::future::TimeoutFuture::new(duration.as_millis() as u32);
            
            futures::select! {
                result = future.fuse() => Ok(result),
                _ = timeout_future.fuse() => Err(TimeoutError),
            }
        }
    }
}
```

## 5. NPX Distribution Mechanism

### 5.1 Package.json Configuration

```json
{
  "name": "code-mesh",
  "version": "0.1.0",
  "description": "AI-powered code mesh with multi-agent orchestration",
  "main": "index.js",
  "bin": {
    "code-mesh": "cli.js"
  },
  "files": [
    "code_mesh_core_bg.wasm",
    "code_mesh_core.js",
    "index.js",
    "cli.js",
    "package.json"
  ],
  "scripts": {
    "build": "wasm-pack build --target bundler",
    "build:node": "wasm-pack build --target nodejs",
    "postinstall": "node postinstall.js"
  },
  "keywords": ["ai", "code", "mesh", "wasm", "cli"],
  "author": "Code Mesh Team",
  "license": "MIT",
  "dependencies": {
    "commander": "^9.0.0"
  }
}
```

### 5.2 Node.js CLI Launcher

```javascript
#!/usr/bin/env node
// cli.js

const { program } = require('commander');
const { CodeMeshWasm } = require('./code_mesh_core');

async function main() {
    program
        .name('code-mesh')
        .description('AI-powered code mesh with multi-agent orchestration')
        .version('0.1.0');

    program
        .command('init')
        .description('Initialize a new code mesh project')
        .argument('[path]', 'Project path', '.')
        .action(async (path) => {
            const codeMesh = new CodeMeshWasm();
            await codeMesh.init_project(path);
        });

    program
        .command('run')
        .description('Execute a single prompt')
        .argument('<prompt>', 'Prompt to execute')
        .option('-m, --model <model>', 'Model to use', 'gpt-4')
        .action(async (prompt, options) => {
            const codeMesh = new CodeMeshWasm();
            const result = await codeMesh.execute_prompt(prompt);
            console.log(result);
        });

    program
        .command('status')
        .description('Show current status')
        .action(async () => {
            const codeMesh = new CodeMeshWasm();
            const status = codeMesh.get_status();
            console.log(JSON.parse(status));
        });

    program
        .command('interactive')
        .description('Start interactive session')
        .action(async () => {
            const codeMesh = new CodeMeshWasm();
            await startInteractiveSession(codeMesh);
        });

    await program.parseAsync(process.argv);
}

async function startInteractiveSession(codeMesh) {
    const readline = require('readline');
    const rl = readline.createInterface({
        input: process.stdin,
        output: process.stdout
    });

    console.log('Code Mesh Interactive Session');
    console.log('Type "exit" to quit\n');

    while (true) {
        const prompt = await new Promise(resolve => {
            rl.question('>>> ', resolve);
        });

        if (prompt.toLowerCase() === 'exit') {
            break;
        }

        try {
            const result = await codeMesh.execute_prompt(prompt);
            console.log(result);
        } catch (error) {
            console.error('Error:', error.message);
        }
    }

    rl.close();
}

main().catch(console.error);
```

## 6. Cross-Platform File System Abstraction

### 6.1 Unified File System Interface

```rust
// code-mesh-core/src/fs/mod.rs

pub mod native;
pub mod browser;

use async_trait::async_trait;
use std::path::Path;

#[async_trait]
pub trait FileSystem: Send + Sync {
    async fn read_file(&self, path: &str) -> Result<String, FileError>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), FileError>;
    async fn list_files(&self, path: &str) -> Result<Vec<String>, FileError>;
    async fn exists(&self, path: &str) -> bool;
    async fn create_dir(&self, path: &str) -> Result<(), FileError>;
    async fn remove_file(&self, path: &str) -> Result<(), FileError>;
    async fn get_metadata(&self, path: &str) -> Result<FileMetadata, FileError>;
}

pub struct FileMetadata {
    pub size: u64,
    pub modified: u64,
    pub is_dir: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Browser storage error: {0}")]
    BrowserError(String),
}

pub fn create_filesystem() -> Box<dyn FileSystem> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Box::new(native::NativeFileSystem::new())
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        Box::new(browser::BrowserFileSystem::new())
    }
}
```

### 6.2 Browser File System Implementation

```rust
// code-mesh-core/src/fs/browser.rs

use super::{FileSystem, FileError, FileMetadata};
use async_trait::async_trait;
use wasm_bindgen::prelude::*;
use web_sys::{IdbDatabase, IdbObjectStore, IdbTransaction};

pub struct BrowserFileSystem {
    db: IdbDatabase,
}

impl BrowserFileSystem {
    pub fn new() -> Self {
        // Initialize IndexedDB
        let window = web_sys::window().unwrap();
        let indexed_db = window.indexed_db().unwrap().unwrap();
        
        // Create database request
        let db_request = indexed_db.open("code-mesh-fs").unwrap();
        
        // Handle database setup
        let db = Self::setup_database(db_request);
        
        Self { db }
    }
    
    fn setup_database(request: web_sys::IdbOpenDbRequest) -> IdbDatabase {
        // Database setup logic
        // This would involve creating object stores for files
        todo!("Implement IndexedDB setup")
    }
    
    async fn get_file_store(&self) -> Result<IdbObjectStore, FileError> {
        let transaction = self.db
            .transaction_with_str_sequence_and_mode(
                &js_sys::Array::of1(&"files".into()),
                web_sys::IdbTransactionMode::Readonly,
            )
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        Ok(transaction.object_store("files").unwrap())
    }
}

#[async_trait]
impl FileSystem for BrowserFileSystem {
    async fn read_file(&self, path: &str) -> Result<String, FileError> {
        let store = self.get_file_store().await?;
        
        // Use wasm-bindgen-futures to handle IndexedDB promises
        let request = store.get(&path.into()).unwrap();
        let result = wasm_bindgen_futures::JsFuture::from(request)
            .await
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        if result.is_undefined() {
            return Err(FileError::NotFound(path.to_string()));
        }
        
        // Extract file content from IndexedDB result
        let content = js_sys::Reflect::get(&result, &"content".into())
            .unwrap()
            .as_string()
            .unwrap();
            
        Ok(content)
    }
    
    async fn write_file(&self, path: &str, content: &str) -> Result<(), FileError> {
        let transaction = self.db
            .transaction_with_str_sequence_and_mode(
                &js_sys::Array::of1(&"files".into()),
                web_sys::IdbTransactionMode::Readwrite,
            )
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        let store = transaction.object_store("files").unwrap();
        
        // Create file object
        let file_obj = js_sys::Object::new();
        js_sys::Reflect::set(&file_obj, &"path".into(), &path.into()).unwrap();
        js_sys::Reflect::set(&file_obj, &"content".into(), &content.into()).unwrap();
        js_sys::Reflect::set(&file_obj, &"modified".into(), &js_sys::Date::now().into()).unwrap();
        
        let request = store.put(&file_obj).unwrap();
        wasm_bindgen_futures::JsFuture::from(request)
            .await
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        Ok(())
    }
    
    async fn list_files(&self, path: &str) -> Result<Vec<String>, FileError> {
        let store = self.get_file_store().await?;
        
        // Use cursor to iterate through files
        let request = store.open_cursor().unwrap();
        let mut files = Vec::new();
        
        // This would require implementing a cursor iteration
        // For now, return empty vector
        Ok(files)
    }
    
    async fn exists(&self, path: &str) -> bool {
        self.read_file(path).await.is_ok()
    }
    
    async fn create_dir(&self, _path: &str) -> Result<(), FileError> {
        // Directory concept doesn't apply to IndexedDB
        Ok(())
    }
    
    async fn remove_file(&self, path: &str) -> Result<(), FileError> {
        let transaction = self.db
            .transaction_with_str_sequence_and_mode(
                &js_sys::Array::of1(&"files".into()),
                web_sys::IdbTransactionMode::Readwrite,
            )
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        let store = transaction.object_store("files").unwrap();
        let request = store.delete(&path.into()).unwrap();
        
        wasm_bindgen_futures::JsFuture::from(request)
            .await
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        Ok(())
    }
    
    async fn get_metadata(&self, path: &str) -> Result<FileMetadata, FileError> {
        let store = self.get_file_store().await?;
        let request = store.get(&path.into()).unwrap();
        let result = wasm_bindgen_futures::JsFuture::from(request)
            .await
            .map_err(|e| FileError::BrowserError(format!("{:?}", e)))?;
            
        if result.is_undefined() {
            return Err(FileError::NotFound(path.to_string()));
        }
        
        let modified = js_sys::Reflect::get(&result, &"modified".into())
            .unwrap()
            .as_f64()
            .unwrap() as u64;
            
        let content = js_sys::Reflect::get(&result, &"content".into())
            .unwrap()
            .as_string()
            .unwrap();
            
        Ok(FileMetadata {
            size: content.len() as u64,
            modified,
            is_dir: false,
        })
    }
}
```

## 7. Testing Strategy

### 7.1 Cross-Platform Test Configuration

```rust
// code-mesh-core/tests/integration.rs

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_native_execution() {
        let code_mesh = CodeMesh::new();
        let result = code_mesh.execute_prompt("Hello, world!").await;
        assert!(result.is_ok());
    }
    
    #[wasm_bindgen_test]
    #[cfg(target_arch = "wasm32")]
    async fn test_wasm_execution() {
        let code_mesh = CodeMeshWasm::new();
        let result = code_mesh.execute_prompt("Hello, world!").await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    #[cfg(not(target_arch = "wasm32"))]
    async fn test_file_operations_native() {
        let fs = create_filesystem();
        let content = "test content";
        
        fs.write_file("test.txt", content).await.unwrap();
        let read_content = fs.read_file("test.txt").await.unwrap();
        assert_eq!(content, read_content);
        
        fs.remove_file("test.txt").await.unwrap();
    }
    
    #[wasm_bindgen_test]
    #[cfg(target_arch = "wasm32")]
    async fn test_file_operations_wasm() {
        let fs = create_filesystem();
        let content = "test content";
        
        fs.write_file("test.txt", content).await.unwrap();
        let read_content = fs.read_file("test.txt").await.unwrap();
        assert_eq!(content, read_content);
        
        fs.remove_file("test.txt").await.unwrap();
    }
}
```

### 7.2 CI/CD Pipeline Configuration

```yaml
# .github/workflows/ci.yml
name: CI/CD Pipeline

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test-native:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
    - name: Run native tests
      run: cargo test --features cli
      
  test-wasm:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v3
    - uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
        profile: minimal
        override: true
        target: wasm32-unknown-unknown
    - name: Install wasm-pack
      run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
    - name: Run WASM tests
      run: wasm-pack test --headless --firefox --features browser
      
  build-and-publish:
    needs: [test-native, test-wasm]
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
    - uses: actions/checkout@v3
    - name: Build WASM package
      run: |
        wasm-pack build --target bundler --features browser
        wasm-pack build --target nodejs --out-dir pkg-node --features browser
    - name: Publish to npm
      run: |
        cd pkg
        npm publish
      env:
        NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}
```

## 8. Performance Optimization

### 8.1 WASM Size Optimization

```toml
# Cargo.toml optimization settings
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"

[profile.release.package."*"]
opt-level = "s"
```

### 8.2 Bundle Size Analysis

```bash
#!/bin/bash
# scripts/analyze-bundle.sh

# Build optimized WASM
wasm-pack build --target bundler --features browser --release

# Analyze bundle size
echo "WASM file size:"
ls -lh pkg/*.wasm

echo "JS file size:"
ls -lh pkg/*.js

echo "Total package size:"
du -sh pkg/

# Run wasm-opt for further optimization
wasm-opt -Oz -o pkg/optimized.wasm pkg/code_mesh_core_bg.wasm

echo "Optimized WASM size:"
ls -lh pkg/optimized.wasm
```

## 9. Future Enhancements

### 9.1 Web Workers Integration

```rust
// Future: Web Workers for parallel execution
#[cfg(target_arch = "wasm32")]
pub struct WebWorkerAgent {
    worker: web_sys::Worker,
}

#[cfg(target_arch = "wasm32")]
impl WebWorkerAgent {
    pub fn new() -> Self {
        let worker = web_sys::Worker::new("agent-worker.js").unwrap();
        Self { worker }
    }
    
    pub async fn execute_in_worker(&self, task: &str) -> Result<String, JsValue> {
        // Send task to web worker
        self.worker.post_message(&task.into())?;
        
        // Wait for response
        let (sender, receiver) = futures::channel::oneshot::channel();
        
        // Set up message handler
        let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let result = event.data().as_string().unwrap();
            sender.send(result).unwrap();
        }) as Box<dyn FnMut(_)>);
        
        self.worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        
        receiver.await.map_err(|e| JsValue::from_str(&e.to_string()))
    }
}
```

### 9.2 Progressive Web App Integration

```javascript
// service-worker.js for PWA support
self.addEventListener('install', event => {
    event.waitUntil(
        caches.open('code-mesh-v1').then(cache => {
            return cache.addAll([
                '/',
                '/code_mesh_core_bg.wasm',
                '/code_mesh_core.js',
                '/index.html',
                '/style.css'
            ]);
        })
    );
});

self.addEventListener('fetch', event => {
    event.respondWith(
        caches.match(event.request).then(response => {
            return response || fetch(event.request);
        })
    );
});
```

## Conclusion

This comprehensive WebAssembly integration strategy provides a robust foundation for building Code Mesh as a dual-target system. The approach emphasizes:

1. **Conditional Compilation**: Clean separation between native and WASM code paths
2. **Unified Interfaces**: Consistent APIs across platforms using trait abstractions
3. **Optimized Dependencies**: Platform-specific dependency management
4. **Async Compatibility**: Seamless integration with both Tokio and browser event loops
5. **Distribution Strategy**: NPX-ready packaging with Node.js launcher
6. **File System Abstraction**: Cross-platform file operations with IndexedDB fallback
7. **Testing Strategy**: Comprehensive test coverage for both targets
8. **Performance Optimization**: Size-optimized WASM builds

This strategy enables the Code Mesh project to deliver a consistent user experience across native CLI and browser environments while maintaining the performance and safety benefits of Rust.