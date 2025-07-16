use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use code_mesh_core::llm::*;
use tokio::runtime::Runtime;

// Benchmark LLM operations focusing on async performance and memory efficiency

fn benchmark_provider_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("llm_provider");
    
    // Test different batch sizes for request throughput
    for batch_size in [1, 5, 10, 20].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_requests", batch_size),
            batch_size,
            |b, &batch_size| {
                b.to_async(&rt).iter(|| async {
                    let provider = AnthropicProvider::new("test-key".to_string());
                    
                    // Simulate multiple concurrent requests
                    let requests: Vec<_> = (0..batch_size)
                        .map(|i| {
                            let model = Model::Claude3Sonnet;
                            let messages = vec![Message {
                                role: Role::User,
                                content: format!("Test message {}", i),
                            }];
                            provider.create_completion(&model, &messages, None)
                        })
                        .collect();
                    
                    // Execute requests concurrently
                    let results = futures::future::join_all(requests).await;
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_message_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_processing");
    
    // Benchmark message serialization/deserialization
    group.bench_function("serialize_large_conversation", |b| {
        let conversation = create_large_conversation(1000);
        b.iter(|| {
            let serialized = serde_json::to_string(&conversation).unwrap();
            let deserialized: Vec<Message> = serde_json::from_str(&serialized).unwrap();
            black_box(deserialized)
        })
    });
    
    // Benchmark message content processing
    group.bench_function("process_large_content", |b| {
        let large_content = "word ".repeat(100_000); // ~500KB content
        let message = Message {
            role: Role::User,
            content: large_content,
        };
        
        b.iter(|| {
            let processed = process_message_content(&message);
            black_box(processed)
        })
    });
    
    group.finish();
}

fn benchmark_streaming_performance(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("streaming");
    
    group.bench_function("stream_processing", |b| {
        b.to_async(&rt).iter(|| async {
            // Simulate streaming response processing
            let stream_data = generate_stream_data(10000);
            let mut processed_tokens = 0;
            
            for chunk in stream_data {
                processed_tokens += process_stream_chunk(&chunk);
            }
            
            black_box(processed_tokens)
        })
    });
    
    group.finish();
}

fn create_large_conversation(message_count: usize) -> Vec<Message> {
    (0..message_count)
        .map(|i| Message {
            role: if i % 2 == 0 { Role::User } else { Role::Assistant },
            content: format!("This is message number {} in the conversation", i),
        })
        .collect()
}

fn process_message_content(message: &Message) -> usize {
    // Simulate content processing (tokenization, etc.)
    message.content.split_whitespace().count()
}

fn generate_stream_data(chunk_count: usize) -> Vec<String> {
    (0..chunk_count)
        .map(|i| format!("chunk_{}", i))
        .collect()
}

fn process_stream_chunk(chunk: &str) -> usize {
    // Simulate stream chunk processing
    chunk.len()
}

criterion_group!(
    benches,
    benchmark_provider_operations,
    benchmark_message_processing,
    benchmark_streaming_performance
);
criterion_main!(benches);