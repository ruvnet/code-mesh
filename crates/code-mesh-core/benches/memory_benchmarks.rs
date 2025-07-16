use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use code_mesh_core::memory::*;
use std::collections::HashMap;
use memory_stats::memory_stats;

// Memory optimization benchmarks targeting 50% reduction vs TypeScript

fn benchmark_memory_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    // Compare different allocation strategies
    group.bench_function("vector_preallocation", |b| {
        b.iter(|| {
            let mut vec = Vec::with_capacity(10000);
            for i in 0..10000 {
                vec.push(i);
            }
            black_box(vec)
        })
    });
    
    group.bench_function("vector_incremental", |b| {
        b.iter(|| {
            let mut vec = Vec::new();
            for i in 0..10000 {
                vec.push(i);
            }
            black_box(vec)
        })
    });
    
    // String vs bytes performance
    group.bench_function("string_operations", |b| {
        let data = "test ".repeat(1000);
        b.iter(|| {
            let mut result = String::new();
            for _ in 0..100 {
                result.push_str(&data);
            }
            black_box(result)
        })
    });
    
    group.bench_function("bytes_operations", |b| {
        let data = bytes::Bytes::from("test ".repeat(1000));
        b.iter(|| {
            let mut result = Vec::new();
            for _ in 0..100 {
                result.extend_from_slice(&data);
            }
            black_box(result)
        })
    });
    
    group.finish();
}

fn benchmark_caching_strategies(c: &mut Criterion) {
    let mut group = c.benchmark_group("caching");
    
    // LRU cache performance
    group.bench_function("lru_cache_operations", |b| {
        let mut cache = lru::LruCache::new(std::num::NonZeroUsize::new(1000).unwrap());
        b.iter(|| {
            for i in 0..100 {
                cache.put(i, format!("value_{}", i));
            }
            for i in 0..100 {
                black_box(cache.get(&i));
            }
        })
    });
    
    // HashMap vs DashMap for concurrent access
    group.bench_function("hashmap_operations", |b| {
        let mut map = HashMap::new();
        b.iter(|| {
            for i in 0..1000 {
                map.insert(i, format!("value_{}", i));
            }
            for i in 0..1000 {
                black_box(map.get(&i));
            }
        })
    });
    
    group.bench_function("dashmap_operations", |b| {
        let map = dashmap::DashMap::new();
        b.iter(|| {
            for i in 0..1000 {
                map.insert(i, format!("value_{}", i));
            }
            for i in 0..1000 {
                black_box(map.get(&i));
            }
        })
    });
    
    group.finish();
}

fn benchmark_memory_pools(c: &mut Criterion) {
    use std::sync::{Arc, Mutex};
    
    let mut group = c.benchmark_group("memory_pools");
    
    // Object pool vs direct allocation
    group.bench_function("direct_allocation", |b| {
        b.iter(|| {
            let objects: Vec<String> = (0..1000)
                .map(|i| format!("object_{}", i))
                .collect();
            black_box(objects)
        })
    });
    
    group.bench_function("object_pool", |b| {
        let pool = Arc::new(Mutex::new(Vec::<String>::new()));
        
        b.iter(|| {
            let mut objects = Vec::new();
            
            // Try to reuse objects from pool
            for i in 0..1000 {
                let obj = {
                    let mut pool_guard = pool.lock().unwrap();
                    pool_guard.pop().unwrap_or_else(|| String::new())
                };
                
                let mut reused_obj = obj;
                reused_obj.clear();
                reused_obj.push_str(&format!("object_{}", i));
                objects.push(reused_obj);
            }
            
            // Return objects to pool
            {
                let mut pool_guard = pool.lock().unwrap();
                pool_guard.extend(objects);
            }
            
            black_box(())
        })
    });
    
    group.finish();
}

fn benchmark_memory_usage_tracking(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_tracking");
    
    group.bench_function("measure_memory_overhead", |b| {
        b.iter(|| {
            let start_mem = memory_stats().map(|s| s.physical_mem).unwrap_or(0);
            
            // Allocate various data structures
            let _strings: Vec<String> = (0..1000)
                .map(|i| format!("string_{}", i))
                .collect();
            
            let _hashmap: HashMap<usize, String> = (0..1000)
                .map(|i| (i, format!("value_{}", i)))
                .collect();
            
            let end_mem = memory_stats().map(|s| s.physical_mem).unwrap_or(0);
            black_box(end_mem.saturating_sub(start_mem))
        })
    });
    
    group.finish();
}

criterion_group!(
    benches,
    benchmark_memory_allocation_patterns,
    benchmark_caching_strategies,
    benchmark_memory_pools,
    benchmark_memory_usage_tracking
);
criterion_main!(benches);