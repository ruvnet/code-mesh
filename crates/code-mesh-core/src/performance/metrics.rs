//! Performance metrics collection and analysis

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};

/// Units for metrics
#[derive(Debug, Clone, PartialEq)]
pub enum MetricUnit {
    Milliseconds,
    Seconds,
    Bytes,
    Kilobytes,
    Megabytes,
    Count,
    Percentage,
}

/// Individual metric data point
#[derive(Debug, Clone)]
pub struct MetricPoint {
    pub timestamp: SystemTime,
    pub value: f64,
    pub unit: MetricUnit,
    pub metadata: HashMap<String, String>,
}

/// Aggregated metric statistics
#[derive(Debug, Clone)]
pub struct MetricStats {
    pub count: usize,
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
    pub unit: MetricUnit,
}

impl MetricStats {
    fn from_values(values: &[f64], unit: MetricUnit) -> Self {
        if values.is_empty() {
            return Self {
                count: 0,
                sum: 0.0,
                min: 0.0,
                max: 0.0,
                avg: 0.0,
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
                unit,
            };
        }

        let mut sorted_values = values.to_vec();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let count = values.len();
        let sum = values.iter().sum();
        let min = sorted_values[0];
        let max = sorted_values[count - 1];
        let avg = sum / count as f64;

        let p50 = percentile(&sorted_values, 50.0);
        let p95 = percentile(&sorted_values, 95.0);
        let p99 = percentile(&sorted_values, 99.0);

        Self {
            count,
            sum,
            min,
            max,
            avg,
            p50,
            p95,
            p99,
            unit,
        }
    }
}

fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }

    let rank = (percentile / 100.0) * (sorted_values.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        sorted_values[lower_index]
    } else {
        let lower_value = sorted_values[lower_index];
        let upper_value = sorted_values[upper_index];
        let weight = rank - rank.floor();
        lower_value + (upper_value - lower_value) * weight
    }
}

/// Metrics collection system
pub struct MetricsCollector {
    metrics: Arc<RwLock<HashMap<String, Vec<MetricPoint>>>>,
    start_time: Instant,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Record a metric value
    pub fn record(&self, name: &str, value: f64, unit: MetricUnit) {
        self.record_with_metadata(name, value, unit, HashMap::new());
    }

    /// Record a metric value with metadata
    pub fn record_with_metadata(
        &self,
        name: &str,
        value: f64,
        unit: MetricUnit,
        metadata: HashMap<String, String>,
    ) {
        let point = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            unit,
            metadata,
        };

        let mut metrics = self.metrics.write().unwrap();
        metrics.entry(name.to_string()).or_insert_with(Vec::new).push(point);
    }

    /// Increment a counter metric
    pub fn increment_counter(&self, name: &str) {
        self.record(name, 1.0, MetricUnit::Count);
    }

    /// Record a timing measurement
    pub fn record_timing(&self, name: &str, duration: Duration) {
        self.record(name, duration.as_millis() as f64, MetricUnit::Milliseconds);
    }

    /// Record memory usage
    pub fn record_memory(&self, name: &str, bytes: usize) {
        self.record(name, bytes as f64, MetricUnit::Bytes);
    }

    /// Get statistics for a specific metric
    pub fn get_stats(&self, name: &str) -> Option<MetricStats> {
        let metrics = self.metrics.read().unwrap();
        if let Some(points) = metrics.get(name) {
            if points.is_empty() {
                return None;
            }

            let values: Vec<f64> = points.iter().map(|p| p.value).collect();
            let unit = points[0].unit.clone();
            Some(MetricStats::from_values(&values, unit))
        } else {
            None
        }
    }

    /// Get all metric names
    pub fn get_metric_names(&self) -> Vec<String> {
        let metrics = self.metrics.read().unwrap();
        metrics.keys().cloned().collect()
    }

    /// Get summary of all metrics
    pub fn get_summary(&self) -> MetricsSummary {
        let metrics = self.metrics.read().unwrap();
        let mut summary = MetricsSummary::new();

        for (name, points) in metrics.iter() {
            if !points.is_empty() {
                let values: Vec<f64> = points.iter().map(|p| p.value).collect();
                let unit = points[0].unit.clone();
                let stats = MetricStats::from_values(&values, unit);
                summary.metrics.insert(name.clone(), stats);
            }
        }

        summary.uptime = self.start_time.elapsed();
        summary
    }

    /// Clear all metrics
    pub fn clear(&self) {
        let mut metrics = self.metrics.write().unwrap();
        metrics.clear();
    }

    /// Get recent metrics (last N points)
    pub fn get_recent(&self, name: &str, count: usize) -> Vec<MetricPoint> {
        let metrics = self.metrics.read().unwrap();
        if let Some(points) = metrics.get(name) {
            let start_idx = if points.len() > count {
                points.len() - count
            } else {
                0
            };
            points[start_idx..].to_vec()
        } else {
            Vec::new()
        }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let metrics = self.metrics.read().unwrap();
        let mut output = String::new();

        for (name, points) in metrics.iter() {
            if points.is_empty() {
                continue;
            }

            let stats = MetricStats::from_values(
                &points.iter().map(|p| p.value).collect::<Vec<_>>(),
                points[0].unit.clone(),
            );

            // Export as Prometheus histogram
            output.push_str(&format!(
                "# TYPE {} histogram\n",
                name.replace(".", "_")
            ));
            output.push_str(&format!(
                "{}_count {}\n",
                name.replace(".", "_"),
                stats.count
            ));
            output.push_str(&format!(
                "{}_sum {}\n",
                name.replace(".", "_"),
                stats.sum
            ));
            output.push_str(&format!(
                "{}_bucket{{le=\"{:.2}\"}} {}\n",
                name.replace(".", "_"),
                stats.p50,
                stats.count / 2
            ));
            output.push_str(&format!(
                "{}_bucket{{le=\"{:.2}\"}} {}\n",
                name.replace(".", "_"),
                stats.p95,
                (stats.count as f64 * 0.95) as usize
            ));
            output.push_str(&format!(
                "{}_bucket{{le=\"+Inf\"}} {}\n",
                name.replace(".", "_"),
                stats.count
            ));
        }

        output
    }
}

/// Summary of all collected metrics
#[derive(Debug, Clone)]
pub struct MetricsSummary {
    pub metrics: HashMap<String, MetricStats>,
    pub uptime: Duration,
}

impl MetricsSummary {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
            uptime: Duration::default(),
        }
    }

    /// Get average latency for a specific operation
    pub fn get_average_latency(&self, operation: &str) -> Option<f64> {
        let metric_name = format!("{}_duration", operation);
        self.metrics.get(&metric_name).map(|stats| stats.avg)
    }

    /// Get total count for a metric
    pub fn get_total_count(&self, metric_name: &str) -> Option<usize> {
        self.metrics.get(metric_name).map(|stats| stats.count)
    }

    /// Get 95th percentile latency
    pub fn get_p95_latency(&self, operation: &str) -> Option<f64> {
        let metric_name = format!("{}_duration", operation);
        self.metrics.get(&metric_name).map(|stats| stats.p95)
    }

    /// Check if any metrics exceed thresholds
    pub fn check_thresholds(&self, thresholds: &MetricThresholds) -> Vec<ThresholdViolation> {
        let mut violations = Vec::new();

        for (metric_name, stats) in &self.metrics {
            if let Some(threshold) = thresholds.get_threshold(metric_name) {
                if stats.avg > threshold.warning_level {
                    violations.push(ThresholdViolation {
                        metric_name: metric_name.clone(),
                        current_value: stats.avg,
                        threshold_value: threshold.warning_level,
                        severity: if stats.avg > threshold.critical_level {
                            Severity::Critical
                        } else {
                            Severity::Warning
                        },
                    });
                }
            }
        }

        violations
    }
}

/// Performance threshold configuration
#[derive(Debug, Clone)]
pub struct MetricThresholds {
    thresholds: HashMap<String, Threshold>,
}

impl MetricThresholds {
    pub fn new() -> Self {
        let mut thresholds = HashMap::new();
        
        // Default thresholds based on performance targets
        thresholds.insert("tool_execution_duration".to_string(), Threshold {
            warning_level: 100.0, // 100ms
            critical_level: 500.0, // 500ms
        });
        
        thresholds.insert("llm_request_duration".to_string(), Threshold {
            warning_level: 3000.0, // 3s
            critical_level: 10000.0, // 10s
        });
        
        thresholds.insert("memory_usage".to_string(), Threshold {
            warning_level: 100.0 * 1024.0 * 1024.0, // 100MB
            critical_level: 500.0 * 1024.0 * 1024.0, // 500MB
        });

        Self { thresholds }
    }

    pub fn get_threshold(&self, metric_name: &str) -> Option<&Threshold> {
        self.thresholds.get(metric_name)
    }

    pub fn set_threshold(&mut self, metric_name: String, threshold: Threshold) {
        self.thresholds.insert(metric_name, threshold);
    }
}

#[derive(Debug, Clone)]
pub struct Threshold {
    pub warning_level: f64,
    pub critical_level: f64,
}

#[derive(Debug, Clone)]
pub struct ThresholdViolation {
    pub metric_name: String,
    pub current_value: f64,
    pub threshold_value: f64,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Warning,
    Critical,
}