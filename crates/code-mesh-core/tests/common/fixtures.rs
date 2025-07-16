//! Test fixtures and sample data

use code_mesh_core::{
    llm::{ChatMessage, ChatRole},
    session::Session,
};
use serde_json::Value;
use std::collections::HashMap;

/// Sample chat messages for testing
pub struct ChatFixtures;

impl ChatFixtures {
    pub fn simple_conversation() -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are a helpful coding assistant.".to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Help me write a function to calculate fibonacci numbers.".to_string(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "I'll help you write a fibonacci function. Here's an efficient implementation...".to_string(),
            },
        ]
    }

    pub fn tool_conversation() -> Vec<ChatMessage> {
        vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are an AI assistant with access to file operations.".to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Read the contents of README.md".to_string(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "I'll read the README.md file for you.".to_string(),
            },
        ]
    }

    pub fn long_conversation() -> Vec<ChatMessage> {
        let mut messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are a helpful assistant.".to_string(),
            }
        ];

        for i in 1..=10 {
            messages.push(ChatMessage {
                role: ChatRole::User,
                content: format!("Question {}: What is {}?", i, i),
            });
            messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: format!("Answer {}: {} is a number.", i, i),
            });
        }

        messages
    }
}

/// Sample session data for testing
pub struct SessionFixtures;

impl SessionFixtures {
    pub fn basic_session() -> Session {
        Session::new("test-session-123".to_string(), "user-456".to_string())
    }

    pub fn session_with_history() -> Session {
        let mut session = Self::basic_session();
        session.add_message(ChatMessage {
            role: ChatRole::User,
            content: "Hello!".to_string(),
        });
        session.add_message(ChatMessage {
            role: ChatRole::Assistant,
            content: "Hi there! How can I help you today?".to_string(),
        });
        session
    }

    pub fn expired_session() -> Session {
        let mut session = Self::basic_session();
        session.created_at = chrono::Utc::now() - chrono::Duration::days(30);
        session
    }
}

/// File content fixtures for tool testing
pub struct FileFixtures;

impl FileFixtures {
    pub fn rust_source() -> &'static str {
        r#"
use std::collections::HashMap;

/// A simple calculator struct
pub struct Calculator {
    memory: HashMap<String, f64>,
}

impl Calculator {
    pub fn new() -> Self {
        Self {
            memory: HashMap::new(),
        }
    }

    pub fn add(&mut self, a: f64, b: f64) -> f64 {
        a + b
    }

    pub fn subtract(&mut self, a: f64, b: f64) -> f64 {
        a - b
    }

    pub fn store(&mut self, key: String, value: f64) {
        self.memory.insert(key, value);
    }

    pub fn recall(&self, key: &str) -> Option<f64> {
        self.memory.get(key).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition() {
        let mut calc = Calculator::new();
        assert_eq!(calc.add(2.0, 3.0), 5.0);
    }

    #[test]
    fn test_subtraction() {
        let mut calc = Calculator::new();
        assert_eq!(calc.subtract(5.0, 3.0), 2.0);
    }

    #[test]
    fn test_memory() {
        let mut calc = Calculator::new();
        calc.store("result".to_string(), 42.0);
        assert_eq!(calc.recall("result"), Some(42.0));
        assert_eq!(calc.recall("nonexistent"), None);
    }
}
"#
    }

    pub fn javascript_source() -> &'static str {
        r#"
class TodoList {
    constructor() {
        this.items = [];
        this.nextId = 1;
    }

    addItem(text) {
        const item = {
            id: this.nextId++,
            text: text,
            completed: false,
            createdAt: new Date()
        };
        this.items.push(item);
        return item;
    }

    completeItem(id) {
        const item = this.items.find(item => item.id === id);
        if (item) {
            item.completed = true;
            item.completedAt = new Date();
        }
        return item;
    }

    removeItem(id) {
        const index = this.items.findIndex(item => item.id === id);
        if (index !== -1) {
            return this.items.splice(index, 1)[0];
        }
        return null;
    }

    getItems() {
        return [...this.items];
    }

    getCompletedItems() {
        return this.items.filter(item => item.completed);
    }

    getPendingItems() {
        return this.items.filter(item => !item.completed);
    }
}

module.exports = TodoList;
"#
    }

    pub fn markdown_content() -> &'static str {
        r#"
# Project Documentation

## Overview

This is a comprehensive documentation for our project.

## Features

- Feature 1: Advanced functionality
- Feature 2: User-friendly interface
- Feature 3: High performance

## Installation

```bash
npm install
npm start
```

## Usage

```javascript
const app = new App();
app.start();
```

## Contributing

Please read our contributing guidelines before submitting pull requests.

### Code Style

We follow these conventions:
- Use 2 spaces for indentation
- Use semicolons
- Use single quotes for strings

## License

MIT License
"#
    }

    pub fn json_config() -> Value {
        serde_json::json!({
            "name": "test-project",
            "version": "1.0.0",
            "author": "Test Author",
            "license": "MIT",
            "dependencies": {
                "lodash": "^4.17.21",
                "express": "^4.18.2",
                "react": "^18.2.0"
            },
            "scripts": {
                "start": "node index.js",
                "test": "jest",
                "build": "webpack --mode production"
            },
            "keywords": ["test", "example", "demo"]
        })
    }

    pub fn yaml_config() -> &'static str {
        r#"
name: CI/CD Pipeline
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '18'
      - name: Install dependencies
        run: npm ci
      - name: Run tests
        run: npm test
      - name: Upload coverage
        uses: codecov/codecov-action@v3

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build application
        run: npm run build
      - name: Deploy
        if: github.ref == 'refs/heads/main'
        run: npm run deploy
"#
    }
}

/// Tool parameter fixtures
pub struct ToolFixtures;

impl ToolFixtures {
    pub fn read_file_params() -> Value {
        serde_json::json!({
            "file_path": "/path/to/file.txt"
        })
    }

    pub fn write_file_params() -> Value {
        serde_json::json!({
            "file_path": "/path/to/output.txt",
            "content": "Hello, World!"
        })
    }

    pub fn edit_file_params() -> Value {
        serde_json::json!({
            "file_path": "/path/to/file.txt",
            "old_string": "old content",
            "new_string": "new content"
        })
    }

    pub fn bash_command_params() -> Value {
        serde_json::json!({
            "command": "ls -la",
            "working_directory": "/tmp"
        })
    }

    pub fn web_search_params() -> Value {
        serde_json::json!({
            "query": "rust programming language",
            "num_results": 10
        })
    }

    pub fn glob_pattern_params() -> Value {
        serde_json::json!({
            "pattern": "**/*.rs",
            "exclude": ["target/**", ".git/**"]
        })
    }
}

/// Error scenario fixtures for testing error handling
pub struct ErrorFixtures;

impl ErrorFixtures {
    pub fn network_error_response() -> reqwest::Error {
        // This is a bit tricky to create directly, so we'll use a helper
        reqwest::Error::from(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "Connection refused"
        ))
    }

    pub fn invalid_json() -> &'static str {
        r#"{ "invalid": json, missing quotes }"#
    }

    pub fn malformed_chat_message() -> Value {
        serde_json::json!({
            "role": "invalid_role",
            "content": null
        })
    }

    pub fn oversized_content() -> String {
        "x".repeat(10_000_000) // 10MB of x's
    }
}

/// Performance testing fixtures
pub struct PerformanceFixtures;

impl PerformanceFixtures {
    pub fn large_file_content(size_mb: usize) -> String {
        let line = "This is a line of text for performance testing.\n";
        let lines_needed = (size_mb * 1024 * 1024) / line.len();
        line.repeat(lines_needed)
    }

    pub fn many_small_files() -> Vec<(String, String)> {
        (0..1000)
            .map(|i| {
                (
                    format!("file_{:04}.txt", i),
                    format!("Content of file number {}", i),
                )
            })
            .collect()
    }

    pub fn deep_directory_structure() -> Vec<String> {
        let mut paths = Vec::new();
        for depth in 1..=20 {
            let path = (0..depth)
                .map(|i| format!("level_{}", i))
                .collect::<Vec<_>>()
                .join("/");
            paths.push(format!("{}/file.txt", path));
        }
        paths
    }
}
"#