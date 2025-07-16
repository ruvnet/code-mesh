use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use code_mesh_core::*;
use tokio::runtime::Runtime;
use std::time::Instant;

// Integration benchmarks comparing end-to-end performance with OpenCode TypeScript

fn benchmark_full_tool_execution_pipeline(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("integration_pipeline");
    
    // Benchmark complete tool execution workflow
    group.bench_function("complete_workflow", |b| {
        b.to_async(&rt).iter(|| async {
            let start = Instant::now();
            
            // Simulate complete workflow:
            // 1. Read files
            // 2. Process with LLM
            // 3. Execute tools
            // 4. Write results
            
            // 1. File operations
            let temp_dir = tempfile::TempDir::new().unwrap();
            let test_file = temp_dir.path().join("test.rs");
            std::fs::write(&test_file, "fn main() { println!(\"Hello\"); }").unwrap();
            
            let read_tool = tool::ReadTool::new();
            let content = read_tool.execute(test_file.to_str().unwrap(), None, None).unwrap();
            
            // 2. LLM processing simulation
            let model = llm::Model::Claude3Sonnet;
            let messages = vec![llm::Message {
                role: llm::Role::User,
                content: format!("Analyze this code: {}", content),
            }];
            
            // 3. Tool execution
            let edit_tool = tool::EditTool::new();
            let _edit_result = edit_tool.execute(
                test_file.to_str().unwrap(),
                "main",
                "main_function",
                false
            );
            
            // 4. Write results
            let output_file = temp_dir.path().join("output.rs");
            let write_tool = tool::WriteTool::new();
            let _write_result = write_tool.execute(
                output_file.to_str().unwrap(),
                "fn main_function() { println!(\"Hello, World!\"); }"
            );
            
            let duration = start.elapsed();
            black_box(duration)
        })
    });
    
    group.finish();
}

fn benchmark_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_operations");
    
    for concurrency_level in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_file_ops", concurrency_level),
            concurrency_level,
            |b, &concurrency_level| {
                b.to_async(&rt).iter(|| async {
                    let temp_dir = tempfile::TempDir::new().unwrap();
                    
                    // Create concurrent file operations
                    let tasks: Vec<_> = (0..concurrency_level)
                        .map(|i| {
                            let temp_dir_path = temp_dir.path().to_owned();
                            tokio::spawn(async move {
                                let file_path = temp_dir_path.join(format!("file_{}.txt", i));
                                let content = format!("Content for file {}", i);
                                
                                // Write file
                                let write_tool = tool::WriteTool::new();
                                write_tool.execute(
                                    file_path.to_str().unwrap(),
                                    &content
                                ).unwrap();
                                
                                // Read file back
                                let read_tool = tool::ReadTool::new();
                                read_tool.execute(
                                    file_path.to_str().unwrap(),
                                    None,
                                    None
                                ).unwrap()
                            })
                        })
                        .collect();
                    
                    let results = futures::future::join_all(tasks).await;
                    black_box(results)
                })
            },
        );
    }
    
    group.finish();
}

fn benchmark_session_management(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("session_management");
    
    group.bench_function("session_lifecycle", |b| {
        b.to_async(&rt).iter(|| async {
            // Test session creation, operations, and cleanup
            let session_manager = session::SessionManager::new();
            
            // Create session
            let session_id = session_manager.create_session().await.unwrap();
            
            // Perform operations
            for i in 0..100 {
                session_manager.add_message(
                    &session_id,
                    llm::Message {
                        role: llm::Role::User,
                        content: format!("Message {}", i),
                    }
                ).await.unwrap();
            }
            
            // Retrieve session data
            let messages = session_manager.get_messages(&session_id).await.unwrap();
            
            // Cleanup
            session_manager.delete_session(&session_id).await.unwrap();
            
            black_box(messages.len())
        })
    });
    
    group.finish();
}

fn benchmark_memory_pressure(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_pressure");
    
    group.bench_function("high_memory_load", |b| {
        b.to_async(&rt).iter(|| async {
            use memory_stats::memory_stats;
            
            let start_memory = memory_stats().unwrap().physical_mem;
            
            // Create high memory pressure scenario
            let mut large_data = Vec::new();
            
            // Simulate processing large files
            for i in 0..100 {
                let large_content = "data ".repeat(10_000); // ~50KB per iteration
                large_data.push(large_content);
                
                // Process with tools
                let temp_dir = tempfile::TempDir::new().unwrap();
                let file_path = temp_dir.path().join(format!("large_file_{}.txt", i));
                
                let write_tool = tool::WriteTool::new();
                write_tool.execute(
                    file_path.to_str().unwrap(),
                    &large_data[i]
                ).unwrap();
            }
            
            let end_memory = memory_stats().unwrap().physical_mem;
            let memory_used = end_memory - start_memory;
            
            black_box(memory_used)
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_full_tool_execution_pipeline,
    benchmark_concurrent_operations,
    benchmark_session_management,
    benchmark_memory_pressure
);
criterion_main!(benches);