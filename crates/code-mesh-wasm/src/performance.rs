//! WASM-specific performance optimizations

use wasm_bindgen::prelude::*;
use js_sys::{Performance, Date};
use web_sys::console;
use std::collections::HashMap;

/// WASM performance monitor and optimizer
#[wasm_bindgen]
pub struct WasmPerformanceMonitor {
    metrics: HashMap<String, Vec<f64>>,
    start_time: f64,
}

#[wasm_bindgen]
impl WasmPerformanceMonitor {
    /// Create a new WASM performance monitor
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        
        Self {
            metrics: HashMap::new(),
            start_time: get_current_time(),
        }
    }

    /// Start timing an operation
    #[wasm_bindgen]
    pub fn start_timer(&self, operation: &str) -> f64 {
        get_current_time()
    }

    /// End timing and record the measurement
    #[wasm_bindgen]
    pub fn end_timer(&mut self, operation: &str, start_time: f64) {
        let duration = get_current_time() - start_time;
        self.record_metric(operation, duration);
    }

    /// Record a performance metric
    #[wasm_bindgen]
    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.metrics.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Get average for a metric
    #[wasm_bindgen]
    pub fn get_average(&self, name: &str) -> f64 {
        if let Some(values) = self.metrics.get(name) {
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        } else {
            0.0
        }
    }

    /// Get the current memory usage (approximation)
    #[wasm_bindgen]
    pub fn get_memory_usage(&self) -> usize {
        // WASM memory is linear, we can estimate usage
        let memory = wasm_bindgen::memory();
        memory.buffer().byte_length() as usize
    }

    /// Get performance report as JSON string
    #[wasm_bindgen]
    pub fn get_report(&self) -> String {
        let mut report = HashMap::new();
        
        for (name, values) in &self.metrics {
            let avg = if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            };
            
            let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            report.insert(name.clone(), serde_json::json!({
                "average": avg,
                "min": min,
                "max": max,
                "count": values.len()
            }));
        }

        report.insert("uptime".to_string(), serde_json::json!(get_current_time() - self.start_time));
        report.insert("memory_usage".to_string(), serde_json::json!(self.get_memory_usage()));

        serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string())
    }

    /// Clear all metrics
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.metrics.clear();
    }

    /// Log performance data to console
    #[wasm_bindgen]
    pub fn log_performance(&self) {
        console::log_1(&format!("Performance Report: {}", self.get_report()).into());
    }
}

/// WASM-optimized cache for reducing JavaScript interop
#[wasm_bindgen]
pub struct WasmCache {
    data: HashMap<String, String>,
    max_size: usize,
    access_count: HashMap<String, u32>,
}

#[wasm_bindgen]
impl WasmCache {
    /// Create a new WASM cache
    #[wasm_bindgen(constructor)]
    pub fn new(max_size: usize) -> Self {
        Self {
            data: HashMap::new(),
            max_size,
            access_count: HashMap::new(),
        }
    }

    /// Get a value from the cache
    #[wasm_bindgen]
    pub fn get(&mut self, key: &str) -> Option<String> {
        if let Some(value) = self.data.get(key) {
            // Update access count for LRU
            *self.access_count.entry(key.to_string()).or_insert(0) += 1;
            Some(value.clone())
        } else {
            None
        }
    }

    /// Set a value in the cache
    #[wasm_bindgen]
    pub fn set(&mut self, key: &str, value: &str) {
        // If at capacity, remove least recently used
        if self.data.len() >= self.max_size && !self.data.contains_key(key) {
            self.evict_lru();
        }

        self.data.insert(key.to_string(), value.to_string());
        self.access_count.insert(key.to_string(), 1);
    }

    /// Remove a value from the cache
    #[wasm_bindgen]
    pub fn remove(&mut self, key: &str) -> bool {
        let removed = self.data.remove(key).is_some();
        self.access_count.remove(key);
        removed
    }

    /// Get cache size
    #[wasm_bindgen]
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Clear the cache
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.data.clear();
        self.access_count.clear();
    }

    /// Get cache hit rate
    #[wasm_bindgen]
    pub fn hit_rate(&self) -> f64 {
        // This would need to track hits/misses separately for accurate calculation
        // For now, return a placeholder
        0.0
    }

    fn evict_lru(&mut self) {
        if let Some((lru_key, _)) = self.access_count.iter()
            .min_by_key(|(_, &count)| count)
            .map(|(key, count)| (key.clone(), *count)) {
            self.data.remove(&lru_key);
            self.access_count.remove(&lru_key);
        }
    }
}

/// Optimized string operations for WASM
#[wasm_bindgen]
pub struct WasmStringProcessor {
    buffer: Vec<u8>,
}

#[wasm_bindgen]
impl WasmStringProcessor {
    /// Create a new string processor
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(8192), // 8KB initial capacity
        }
    }

    /// Process text with minimal allocations
    #[wasm_bindgen]
    pub fn process_text(&mut self, input: &str) -> String {
        self.buffer.clear();
        
        // Perform text processing directly on bytes for efficiency
        for byte in input.bytes() {
            // Example processing: convert to uppercase
            if byte >= b'a' && byte <= b'z' {
                self.buffer.push(byte - 32);
            } else {
                self.buffer.push(byte);
            }
        }

        String::from_utf8_lossy(&self.buffer).to_string()
    }

    /// Split text efficiently
    #[wasm_bindgen]
    pub fn split_lines(&self, input: &str) -> js_sys::Array {
        let array = js_sys::Array::new();
        
        for line in input.lines() {
            array.push(&JsValue::from_str(line));
        }

        array
    }

    /// Count words efficiently
    #[wasm_bindgen]
    pub fn count_words(&self, input: &str) -> usize {
        input.split_whitespace().count()
    }
}

/// WASM batch processor for reducing JavaScript roundtrips
#[wasm_bindgen]
pub struct WasmBatchProcessor {
    operations: Vec<String>,
    results: Vec<String>,
}

#[wasm_bindgen]
impl WasmBatchProcessor {
    /// Create a new batch processor
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            results: Vec::new(),
        }
    }

    /// Add an operation to the batch
    #[wasm_bindgen]
    pub fn add_operation(&mut self, operation: &str) {
        self.operations.push(operation.to_string());
    }

    /// Process all batched operations at once
    #[wasm_bindgen]
    pub fn process_batch(&mut self) -> js_sys::Array {
        self.results.clear();

        for operation in &self.operations {
            // Process each operation (example: uppercase)
            let result = operation.to_uppercase();
            self.results.push(result);
        }

        // Convert results to JavaScript array
        let array = js_sys::Array::new();
        for result in &self.results {
            array.push(&JsValue::from_str(result));
        }

        self.operations.clear();
        array
    }

    /// Get batch size
    #[wasm_bindgen]
    pub fn batch_size(&self) -> usize {
        self.operations.len()
    }

    /// Clear the batch
    #[wasm_bindgen]
    pub fn clear(&mut self) {
        self.operations.clear();
        self.results.clear();
    }
}

/// Memory pool for WASM to reduce allocations
#[wasm_bindgen]
pub struct WasmMemoryPool {
    buffers: Vec<Vec<u8>>,
    strings: Vec<String>,
    max_pool_size: usize,
}

#[wasm_bindgen]
impl WasmMemoryPool {
    /// Create a new memory pool
    #[wasm_bindgen(constructor)]
    pub fn new(max_pool_size: usize) -> Self {
        Self {
            buffers: Vec::new(),
            strings: Vec::new(),
            max_pool_size,
        }
    }

    /// Get a buffer from the pool
    #[wasm_bindgen]
    pub fn get_buffer(&mut self, size: usize) -> js_sys::Uint8Array {
        let mut buffer = if let Some(mut buf) = self.buffers.pop() {
            buf.clear();
            buf.reserve(size);
            buf
        } else {
            Vec::with_capacity(size)
        };

        buffer.resize(size, 0);
        
        // Convert to Uint8Array for JavaScript
        unsafe {
            js_sys::Uint8Array::view(&buffer)
        }
    }

    /// Return a buffer to the pool
    #[wasm_bindgen]
    pub fn return_buffer(&mut self, buffer: js_sys::Uint8Array) {
        if self.buffers.len() < self.max_pool_size {
            let vec = buffer.to_vec();
            self.buffers.push(vec);
        }
    }

    /// Get pool statistics
    #[wasm_bindgen]
    pub fn get_stats(&self) -> String {
        serde_json::json!({
            "buffer_pool_size": self.buffers.len(),
            "string_pool_size": self.strings.len(),
            "max_pool_size": self.max_pool_size
        }).to_string()
    }
}

// Helper function to get current time
fn get_current_time() -> f64 {
    Date::now()
}

/// Initialize WASM performance optimizations
#[wasm_bindgen(start)]
pub fn init_wasm_performance() {
    console_error_panic_hook::set_once();
    
    console::log_1(&"WASM Performance Module Initialized".into());
    
    // Log initial memory usage
    let memory = wasm_bindgen::memory();
    console::log_1(&format!("Initial WASM Memory: {} bytes", memory.buffer().byte_length()).into());
}

/// Global WASM performance utilities
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = performance)]
    fn now() -> f64;
    
    #[wasm_bindgen(js_namespace = performance, js_name = mark)]
    fn mark(name: &str);
    
    #[wasm_bindgen(js_namespace = performance, js_name = measure)]
    fn measure(name: &str, start_mark: &str, end_mark: &str);
}

/// High-performance timer for WASM
#[wasm_bindgen]
pub fn wasm_now() -> f64 {
    now()
}

/// Create a performance mark
#[wasm_bindgen]
pub fn wasm_mark(name: &str) {
    mark(name);
}

/// Measure performance between marks
#[wasm_bindgen]
pub fn wasm_measure(name: &str, start_mark: &str, end_mark: &str) {
    measure(name, start_mark, end_mark);
}