//! Performance benchmarks for code-mesh-core

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use code_mesh_core::{
    auth::*,
    llm::*,
    session::*,
    storage::*,
    tool::*,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::runtime::Runtime;

/// Helper to create async runtime for benchmarks
fn runtime() -> Runtime {
    Runtime::new().unwrap()
}

/// Benchmark session operations
fn bench_session_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("session_operations");
    
    // Benchmark session creation
    group.bench_function("create_session", |b| {
        b.iter(|| {
            Session::new(
                black_box(format!("session-{}", fastrand::u64(..))),
                black_box(format!("user-{}", fastrand::u64(..))),
            )
        })
    });
    
    // Benchmark adding messages
    group.bench_function("add_single_message", |b| {
        let mut session = Session::new("bench-session".to_string(), "user".to_string());
        b.iter(|| {
            session.add_message(black_box(ChatMessage {
                role: ChatRole::User,
                content: format!("Message {}", fastrand::u64(..)),
            }));
        })
    });
    
    // Benchmark adding many messages
    for msg_count in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_multiple_messages", msg_count),
            msg_count,
            |b, &msg_count| {
                b.iter(|| {
                    let mut session = Session::new("bench-session".to_string(), "user".to_string());
                    for i in 0..msg_count {
                        session.add_message(ChatMessage {
                            role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
                            content: format!("Message {}", i),
                        });
                    }
                    black_box(session)
                })
            },
        );
    }
    
    // Benchmark serialization
    group.bench_function("session_serialization", |b| {
        let mut session = Session::new("bench-session".to_string(), "user".to_string());
        for i in 0..100 {
            session.add_message(ChatMessage {
                role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
                content: format!("Message {}", i),
            });
        }
        
        b.iter(|| {
            black_box(serde_json::to_string(&session).unwrap())
        })
    });
    
    group.finish();
}

/// Benchmark storage operations
fn bench_storage_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_operations");
    let rt = runtime();
    
    // Benchmark in-memory storage
    group.bench_function("inmemory_storage_save", |b| {
        let storage = InMemoryStorage::new();
        b.to_async(&rt).iter(|| async {
            storage.save(
                black_box(format!("key-{}", fastrand::u64(..))),
                black_box(serde_json::json!({
                    "data": "test-value",
                    "timestamp": chrono::Utc::now(),
                    "number": fastrand::u64(..)
                }))
            ).await.unwrap()
        })
    });
    
    group.bench_function("inmemory_storage_load", |b| {
        let storage = InMemoryStorage::new();
        rt.block_on(async {
            for i in 0..1000 {
                storage.save(
                    format!("key-{}", i),
                    serde_json::json!({"value": i})
                ).await.unwrap();
            }
        });
        
        b.to_async(&rt).iter(|| async {
            storage.load(black_box(format!("key-{}", fastrand::u64(..1000)))).await.unwrap()
        })
    });
    
    // Benchmark file-based auth storage
    group.bench_function("auth_storage_save_load", |b| {
        let temp_dir = TempDir::new().unwrap();
        let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());
        
        b.to_async(&rt).iter(|| async {
            let token = black_box(format!("token-{}", fastrand::u64(..)));
            auth_storage.save_token(token.clone()).await.unwrap();
            auth_storage.load_token().await.unwrap()
        })
    });
    
    group.finish();
}

/// Benchmark LLM operations
fn bench_llm_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("llm_operations");
    let rt = runtime();
    
    // Benchmark mock LLM provider
    group.bench_function("mock_llm_completion", |b| {
        let provider = MockLLMProvider::with_responses(vec![
            "Response 1".to_string(),
            "Response 2".to_string(),
            "Response 3".to_string(),
        ]);
        
        b.to_async(&rt).iter(|| async {
            let messages = black_box(vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: format!("Question {}", fastrand::u64(..)),
                }
            ]);
            provider.chat_completion(messages).await.unwrap()
        })
    });
    
    // Benchmark with different message counts
    for msg_count in [1, 10, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("llm_completion_with_context", msg_count),
            msg_count,
            |b, &msg_count| {
                let provider = MockLLMProvider::new();
                b.to_async(&rt).iter(|| async {
                    let messages: Vec<_> = (0..msg_count)
                        .map(|i| ChatMessage {
                            role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
                            content: format!("Message {}", i),
                        })
                        .collect();
                    provider.chat_completion(black_box(messages)).await.unwrap()
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark tool operations
fn bench_tool_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_operations");
    let rt = runtime();
    
    // Benchmark tool execution
    group.bench_function("counting_tool_execution", |b| {
        let tool = CountingTool::new("bench-tool");
        b.to_async(&rt).iter(|| async {
            tool.execute(black_box(serde_json::json!({
                "test_param": fastrand::u64(..)
            }))).await.unwrap()
        })
    });
    
    // Benchmark concurrent tool execution
    group.bench_function("concurrent_tool_execution", |b| {
        let tool = Arc::new(CountingTool::new("concurrent-bench-tool"));
        b.to_async(&rt).iter(|| async {
            let handles: Vec<_> = (0..10)
                .map(|i| {
                    let tool = Arc::clone(&tool);
                    tokio::spawn(async move {
                        tool.execute(serde_json::json!({"worker": i})).await
                    })
                })
                .collect();
            
            for handle in handles {
                handle.await.unwrap().unwrap();
            }
        })
    });
    
    group.finish();
}

/// Benchmark serialization operations
fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");
    
    // Create test data
    let session = {
        let mut s = Session::new("bench-session".to_string(), "user".to_string());
        for i in 0..1000 {
            s.add_message(ChatMessage {
                role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
                content: format!("This is message number {} with some content to make it realistic", i),
            });
        }
        s
    };
    
    let chat_completion = ChatCompletion {
        id: "bench-completion".to_string(),
        content: "This is a benchmark response with a reasonable amount of content that simulates a real LLM response.".to_string(),
        model: "benchmark-model".to_string(),
        usage: Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        },
        created_at: chrono::Utc::now(),
    };
    
    // Benchmark session serialization
    group.throughput(Throughput::Elements(session.messages.len() as u64));
    group.bench_function("session_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&session)).unwrap())
    });
    
    group.bench_function("session_deserialize", |b| {
        let serialized = serde_json::to_string(&session).unwrap();
        b.iter(|| {
            let _: Session = serde_json::from_str(black_box(&serialized)).unwrap();
        })
    });
    
    // Benchmark chat completion serialization
    group.bench_function("completion_serialize", |b| {
        b.iter(|| serde_json::to_string(black_box(&chat_completion)).unwrap())
    });
    
    group.bench_function("completion_deserialize", |b| {
        let serialized = serde_json::to_string(&chat_completion).unwrap();
        b.iter(|| {
            let _: ChatCompletion = serde_json::from_str(black_box(&serialized)).unwrap();
        })
    });
    
    group.finish();
}

/// Benchmark memory usage and allocation patterns
fn bench_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    // Benchmark large session creation and cleanup
    group.bench_function("large_session_lifecycle", |b| {
        b.iter(|| {
            let mut session = Session::new("memory-bench".to_string(), "user".to_string());
            for i in 0..10000 {
                session.add_message(ChatMessage {
                    role: if i % 2 == 0 { ChatRole::User } else { ChatRole::Assistant },
                    content: format!("Message {}", i),
                });
            }
            // Session is dropped here, testing cleanup
            black_box(session)
        })
    });
    
    // Benchmark string operations
    group.bench_function("string_operations", |b| {
        b.iter(|| {
            let mut content = String::new();
            for i in 0..1000 {
                content.push_str(&format!("Line {} of content\n", i));
            }
            black_box(content)
        })
    });
    
    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    let rt = runtime();
    
    // Benchmark concurrent session operations
    group.bench_function("concurrent_sessions", |b| {
        b.to_async(&rt).iter(|| async {
            let storage = Arc::new(InMemoryStorage::new());
            let handles: Vec<_> = (0..100)
                .map(|i| {
                    let storage = Arc::clone(&storage);
                    tokio::spawn(async move {
                        let session = Session::new(
                            format!("session-{}", i),
                            format!("user-{}", i),
                        );
                        storage.save(
                            format!("session-{}", i),
                            serde_json::to_value(session).unwrap()
                        ).await.unwrap();
                    })
                })
                .collect();
            
            for handle in handles {
                handle.await.unwrap();
            }
        })
    });
    
    group.finish();
}

// Mock implementations for benchmarking
#[derive(Clone)]
struct InMemoryStorage {
    data: Arc<parking_lot::RwLock<std::collections::HashMap<String, serde_json::Value>>>,
}

impl InMemoryStorage {
    fn new() -> Self {
        Self {
            data: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl Storage for InMemoryStorage {
    async fn save(&self, key: String, data: serde_json::Value) -> code_mesh_core::CodeMeshResult<()> {
        self.data.write().insert(key, data);
        Ok(())
    }

    async fn load(&self, key: String) -> code_mesh_core::CodeMeshResult<Option<serde_json::Value>> {
        Ok(self.data.read().get(&key).cloned())
    }

    async fn delete(&self, key: String) -> code_mesh_core::CodeMeshResult<()> {
        self.data.write().remove(&key);
        Ok(())
    }

    async fn list_keys(&self, prefix: Option<String>) -> code_mesh_core::CodeMeshResult<Vec<String>> {
        let keys: Vec<String> = self
            .data
            .read()
            .keys()
            .filter(|key| {
                prefix
                    .as_ref()
                    .map(|p| key.starts_with(p))
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        Ok(keys)
    }
}

#[derive(Clone)]
struct MockLLMProvider {
    responses: Arc<parking_lot::RwLock<Vec<String>>>,
    current_index: Arc<parking_lot::RwLock<usize>>,
}

impl MockLLMProvider {
    fn new() -> Self {
        Self::with_responses(vec!["Default benchmark response".to_string()])
    }

    fn with_responses(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(parking_lot::RwLock::new(responses)),
            current_index: Arc::new(parking_lot::RwLock::new(0)),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for MockLLMProvider {
    async fn chat_completion(&self, _messages: Vec<ChatMessage>) -> code_mesh_core::CodeMeshResult<ChatCompletion> {
        let responses = self.responses.read();
        let mut index = self.current_index.write();
        
        let response = responses
            .get(*index)
            .unwrap_or(&responses[0])
            .clone();
            
        *index = (*index + 1) % responses.len();

        Ok(ChatCompletion {
            id: format!("bench-completion-{}", *index),
            content: response,
            model: "benchmark-model".to_string(),
            usage: Usage {
                prompt_tokens: 10,
                completion_tokens: 15,
                total_tokens: 25,
            },
            created_at: chrono::Utc::now(),
        })
    }

    async fn stream_completion(
        &self,
        messages: Vec<ChatMessage>,
    ) -> code_mesh_core::CodeMeshResult<Box<dyn futures::Stream<Item = code_mesh_core::CodeMeshResult<String>> + Send + Unpin>> {
        let completion = self.chat_completion(messages).await?;
        let content = completion.content;
        
        let stream = futures::stream::iter(
            content
                .chars()
                .map(|c| Ok(c.to_string()))
                .collect::<Vec<_>>()
        );

        Ok(Box::new(Box::pin(stream)))
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            name: "benchmark-model".to_string(),
            provider: "benchmark".to_string(),
            max_tokens: 4096,
            supports_streaming: true,
            supports_tools: true,
        }
    }

    fn validate_config(&self) -> code_mesh_core::CodeMeshResult<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct CountingTool {
    pub call_count: Arc<parking_lot::RwLock<usize>>,
    pub name: String,
}

impl CountingTool {
    fn new(name: &str) -> Self {
        Self {
            call_count: Arc::new(parking_lot::RwLock::new(0)),
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Tool for CountingTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Benchmark counting tool"
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _parameters: serde_json::Value) -> code_mesh_core::CodeMeshResult<ToolResult> {
        *self.call_count.write() += 1;
        Ok(ToolResult {
            output: "Benchmark tool executed".to_string(),
            metadata: serde_json::json!({}),
        })
    }
}

criterion_group!(
    benches,
    bench_session_operations,
    bench_storage_operations,
    bench_llm_operations,
    bench_tool_operations,
    bench_serialization,
    bench_memory_operations,
    bench_concurrent_operations
);
criterion_main!(benches);