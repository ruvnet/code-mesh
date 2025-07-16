use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use code_mesh_core::tool::*;

// Benchmark tool operations against OpenCode TypeScript implementation
// Target: 2x performance improvement

fn benchmark_file_operations(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    let large_content = "a".repeat(1_000_000); // 1MB test file
    
    fs::write(&test_file, &large_content).unwrap();
    
    let mut group = c.benchmark_group("file_operations");
    
    // Read tool benchmark
    group.bench_function("read_large_file", |b| {
        b.iter(|| {
            let reader = ReadTool::new();
            black_box(reader.execute(test_file.to_str().unwrap(), None, None))
        })
    });
    
    // Write tool benchmark
    group.bench_function("write_large_file", |b| {
        let write_path = temp_dir.path().join("write_test.txt");
        b.iter(|| {
            let writer = WriteTool::new();
            black_box(writer.execute(write_path.to_str().unwrap(), &large_content))
        })
    });
    
    // Edit tool benchmark
    group.bench_function("edit_operations", |b| {
        b.iter(|| {
            let editor = EditTool::new();
            black_box(editor.execute(
                test_file.to_str().unwrap(),
                "old_text",
                "new_text",
                false
            ))
        })
    });
    
    group.finish();
}

fn benchmark_search_operations(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let test_files = create_test_codebase(&temp_dir);
    
    let mut group = c.benchmark_group("search_operations");
    
    // Grep tool benchmark
    group.bench_function("grep_search", |b| {
        b.iter(|| {
            let grep = GrepTool::new();
            black_box(grep.execute(
                "function",
                Some(temp_dir.path().to_str().unwrap()),
                Some("*.rs"),
                None,
                false,
                false,
                false,
                false,
                None,
                None,
                None,
                None
            ))
        })
    });
    
    // Glob tool benchmark
    group.bench_function("glob_search", |b| {
        b.iter(|| {
            let glob = GlobTool::new();
            black_box(glob.execute(
                "**/*.rs",
                Some(temp_dir.path().to_str().unwrap())
            ))
        })
    });
    
    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    use memory_stats::memory_stats;
    
    let mut group = c.benchmark_group("memory_usage");
    
    group.bench_function("tool_instantiation", |b| {
        b.iter(|| {
            let start_memory = memory_stats().unwrap().physical_mem;
            
            // Create multiple tool instances
            let tools: Vec<Box<dyn Tool>> = vec![
                Box::new(ReadTool::new()),
                Box::new(WriteTool::new()),
                Box::new(EditTool::new()),
                Box::new(GrepTool::new()),
                Box::new(GlobTool::new()),
            ];
            
            let end_memory = memory_stats().unwrap().physical_mem;
            black_box(end_memory - start_memory)
        })
    });
    
    group.finish();
}

fn create_test_codebase(temp_dir: &TempDir) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    
    // Create a realistic codebase structure
    for i in 0..100 {
        let file_path = temp_dir.path().join(format!("file_{}.rs", i));
        let content = format!(
            "fn function_{}() {{\n    println!(\"Hello from function {}\");\n}}\n",
            i, i
        );
        fs::write(&file_path, content).unwrap();
        files.push(file_path);
    }
    
    files
}

criterion_group!(
    benches,
    benchmark_file_operations,
    benchmark_search_operations,
    benchmark_memory_usage
);
criterion_main!(benches);