//! Performance monitoring and optimization module
//!
//! This module provides comprehensive performance tracking, profiling,
//! and optimization capabilities for the code-mesh system.

pub mod metrics;
pub mod monitor;
pub mod profiler;
pub mod optimizer;
pub mod cache;
pub mod memory_pool;
pub mod async_optimizer;

pub use metrics::*;
pub use monitor::*;
pub use profiler::*;
pub use optimizer::*;
pub use cache::*;
pub use memory_pool::*;
pub use async_optimizer::*;

use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Global performance tracker
static GLOBAL_PERFORMANCE_TRACKER: once_cell::sync::Lazy<PerformanceTracker> = 
    once_cell::sync::Lazy::new(|| PerformanceTracker::new());

/// Performance targets based on EPIC requirements
pub struct PerformanceTargets {
    /// Target: 2x faster than TypeScript implementation
    pub speed_multiplier: f64,
    /// Target: <5MB WASM bundle size
    pub max_wasm_size_mb: f64,
    /// Target: 50% memory reduction vs TypeScript
    pub memory_reduction_target: f64,
    /// Maximum acceptable latency for tool operations
    pub max_tool_latency_ms: u64,
    /// Maximum acceptable latency for LLM operations
    pub max_llm_latency_ms: u64,
}

impl Default for PerformanceTargets {
    fn default() -> Self {
        Self {
            speed_multiplier: 2.0,
            max_wasm_size_mb: 5.0,
            memory_reduction_target: 0.5,
            max_tool_latency_ms: 100,
            max_llm_latency_ms: 5000,
        }
    }
}

/// Main performance tracking interface
pub struct PerformanceTracker {
    metrics: Arc<MetricsCollector>,
    monitor: Arc<SystemMonitor>,
    profiler: Arc<Profiler>,
    optimizer: Arc<Optimizer>,
}

impl PerformanceTracker {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(MetricsCollector::new()),
            monitor: Arc::new(SystemMonitor::new()),
            profiler: Arc::new(Profiler::new()),
            optimizer: Arc::new(Optimizer::new()),
        }
    }

    /// Get the global performance tracker instance
    pub fn global() -> &'static PerformanceTracker {
        &GLOBAL_PERFORMANCE_TRACKER
    }

    /// Start tracking an operation
    pub fn start_operation(&self, operation_type: &str) -> OperationTracker {
        OperationTracker::new(operation_type, &self.metrics)
    }

    /// Record a performance measurement
    pub fn record_measurement(&self, metric_name: &str, value: f64, unit: MetricUnit) {
        self.metrics.record(metric_name, value, unit);
    }

    /// Get current performance report
    pub fn get_performance_report(&self) -> PerformanceReport {
        PerformanceReport {
            metrics: self.metrics.get_summary(),
            system_stats: self.monitor.get_current_stats(),
            memory_usage: self.get_memory_usage(),
            targets: PerformanceTargets::default(),
            timestamp: Instant::now(),
        }
    }

    /// Check if performance targets are being met
    pub fn validate_targets(&self) -> TargetValidationResult {
        let targets = PerformanceTargets::default();
        let current_metrics = self.metrics.get_summary();
        
        TargetValidationResult {
            speed_target_met: self.check_speed_target(&targets, &current_metrics),
            memory_target_met: self.check_memory_target(&targets),
            latency_targets_met: self.check_latency_targets(&targets, &current_metrics),
            overall_score: self.calculate_overall_score(&targets, &current_metrics),
        }
    }

    fn get_memory_usage(&self) -> MemoryUsage {
        use memory_stats::memory_stats;
        
        let stats = memory_stats().unwrap_or_default();
        MemoryUsage {
            physical_mem: stats.physical_mem,
            virtual_mem: stats.virtual_mem,
        }
    }

    fn check_speed_target(&self, targets: &PerformanceTargets, metrics: &MetricsSummary) -> bool {
        // Compare against baseline TypeScript performance
        if let Some(avg_latency) = metrics.get_average_latency("tool_execution") {
            avg_latency < (1000.0 / targets.speed_multiplier) // Target: 2x faster
        } else {
            false
        }
    }

    fn check_memory_target(&self, targets: &PerformanceTargets) -> bool {
        let current_memory = self.get_memory_usage();
        // Compare against TypeScript baseline (would need to be measured)
        // For now, check against absolute threshold
        current_memory.physical_mem < (100 * 1024 * 1024) // 100MB threshold
    }

    fn check_latency_targets(&self, targets: &PerformanceTargets, metrics: &MetricsSummary) -> bool {
        let tool_latency_ok = metrics.get_average_latency("tool_execution")
            .map(|lat| lat < targets.max_tool_latency_ms as f64)
            .unwrap_or(false);
            
        let llm_latency_ok = metrics.get_average_latency("llm_request")
            .map(|lat| lat < targets.max_llm_latency_ms as f64)
            .unwrap_or(false);
            
        tool_latency_ok && llm_latency_ok
    }

    fn calculate_overall_score(&self, targets: &PerformanceTargets, metrics: &MetricsSummary) -> f64 {
        let mut score = 0.0;
        let mut components = 0;

        // Speed component (40% weight)
        if self.check_speed_target(targets, metrics) {
            score += 40.0;
        }
        components += 1;

        // Memory component (30% weight)
        if self.check_memory_target(targets) {
            score += 30.0;
        }
        components += 1;

        // Latency component (30% weight)
        if self.check_latency_targets(targets, metrics) {
            score += 30.0;
        }
        components += 1;

        score
    }
}

/// Tracks individual operation performance
pub struct OperationTracker {
    operation_type: String,
    start_time: Instant,
    metrics_collector: Arc<MetricsCollector>,
}

impl OperationTracker {
    fn new(operation_type: &str, metrics_collector: &Arc<MetricsCollector>) -> Self {
        Self {
            operation_type: operation_type.to_string(),
            start_time: Instant::now(),
            metrics_collector: metrics_collector.clone(),
        }
    }

    /// Finish tracking and record the operation
    pub fn finish(self) {
        let duration = self.start_time.elapsed();
        self.metrics_collector.record(
            &format!("{}_duration", self.operation_type),
            duration.as_millis() as f64,
            MetricUnit::Milliseconds,
        );
    }

    /// Finish with custom metadata
    pub fn finish_with_metadata(self, metadata: std::collections::HashMap<String, String>) {
        let duration = self.start_time.elapsed();
        self.metrics_collector.record_with_metadata(
            &format!("{}_duration", self.operation_type),
            duration.as_millis() as f64,
            MetricUnit::Milliseconds,
            metadata,
        );
    }
}

/// Performance validation results
#[derive(Debug, Clone)]
pub struct TargetValidationResult {
    pub speed_target_met: bool,
    pub memory_target_met: bool,
    pub latency_targets_met: bool,
    pub overall_score: f64,
}

/// Memory usage information
#[derive(Debug, Clone, Default)]
pub struct MemoryUsage {
    pub physical_mem: usize,
    pub virtual_mem: usize,
}

/// Complete performance report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub metrics: MetricsSummary,
    pub system_stats: SystemStats,
    pub memory_usage: MemoryUsage,
    pub targets: PerformanceTargets,
    pub timestamp: Instant,
}

impl PerformanceReport {
    /// Generate a human-readable performance report
    pub fn to_string(&self) -> String {
        format!(
            "Performance Report ({})\n\
             =================================\n\
             Tool Operations: {:.2}ms avg\n\
             LLM Requests: {:.2}ms avg\n\
             Memory Usage: {:.2}MB\n\
             CPU Usage: {:.1}%\n\
             =================================",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            self.metrics.get_average_latency("tool_execution").unwrap_or(0.0),
            self.metrics.get_average_latency("llm_request").unwrap_or(0.0),
            self.memory_usage.physical_mem as f64 / (1024.0 * 1024.0),
            self.system_stats.cpu_usage
        )
    }
}

/// Convenience macro for timing operations
#[macro_export]
macro_rules! time_operation {
    ($operation_type:expr, $block:block) => {{
        let tracker = $crate::performance::PerformanceTracker::global()
            .start_operation($operation_type);
        let result = $block;
        tracker.finish();
        result
    }};
}

/// Convenience macro for timing async operations
#[macro_export]
macro_rules! time_async_operation {
    ($operation_type:expr, $async_block:expr) => {{
        let tracker = $crate::performance::PerformanceTracker::global()
            .start_operation($operation_type);
        let result = $async_block.await;
        tracker.finish();
        result
    }};
}