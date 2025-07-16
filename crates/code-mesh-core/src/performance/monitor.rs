//! Real-time system monitoring and performance tracking

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use std::thread;

/// System performance monitor
pub struct SystemMonitor {
    stats_history: Arc<RwLock<VecDeque<SystemStats>>>,
    is_running: Arc<std::sync::atomic::AtomicBool>,
    config: MonitorConfig,
}

impl SystemMonitor {
    /// Create a new system monitor
    pub fn new() -> Self {
        Self::with_config(MonitorConfig::default())
    }

    /// Create a system monitor with custom configuration
    pub fn with_config(config: MonitorConfig) -> Self {
        Self {
            stats_history: Arc::new(RwLock::new(VecDeque::new())),
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            config,
        }
    }

    /// Start monitoring in the background
    pub fn start(&self) {
        if self.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            return; // Already running
        }

        self.is_running.store(true, std::sync::atomic::Ordering::Relaxed);
        
        let stats_history = self.stats_history.clone();
        let is_running = self.is_running.clone();
        let interval = self.config.collection_interval;
        let max_history = self.config.max_history_size;

        thread::spawn(move || {
            while is_running.load(std::sync::atomic::Ordering::Relaxed) {
                let stats = Self::collect_system_stats();
                
                {
                    let mut history = stats_history.write().unwrap();
                    history.push_back(stats);
                    
                    // Keep history size bounded
                    while history.len() > max_history {
                        history.pop_front();
                    }
                }

                thread::sleep(interval);
            }
        });
    }

    /// Stop monitoring
    pub fn stop(&self) {
        self.is_running.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get current system statistics
    pub fn get_current_stats(&self) -> SystemStats {
        Self::collect_system_stats()
    }

    /// Get statistics history
    pub fn get_stats_history(&self) -> Vec<SystemStats> {
        self.stats_history.read().unwrap().iter().cloned().collect()
    }

    /// Get average statistics over a time period
    pub fn get_average_stats(&self, duration: Duration) -> Option<SystemStats> {
        let history = self.stats_history.read().unwrap();
        let cutoff_time = SystemTime::now() - duration;
        
        let relevant_stats: Vec<&SystemStats> = history
            .iter()
            .filter(|stats| stats.timestamp >= cutoff_time)
            .collect();

        if relevant_stats.is_empty() {
            return None;
        }

        let count = relevant_stats.len() as f64;
        let avg_cpu = relevant_stats.iter().map(|s| s.cpu_usage).sum::<f64>() / count;
        let avg_memory = relevant_stats.iter().map(|s| s.memory_usage).sum::<u64>() / relevant_stats.len() as u64;
        let avg_disk_read = relevant_stats.iter().map(|s| s.disk_read_bytes).sum::<u64>() / relevant_stats.len() as u64;
        let avg_disk_write = relevant_stats.iter().map(|s| s.disk_write_bytes).sum::<u64>() / relevant_stats.len() as u64;
        let avg_network_rx = relevant_stats.iter().map(|s| s.network_rx_bytes).sum::<u64>() / relevant_stats.len() as u64;
        let avg_network_tx = relevant_stats.iter().map(|s| s.network_tx_bytes).sum::<u64>() / relevant_stats.len() as u64;

        Some(SystemStats {
            timestamp: SystemTime::now(),
            cpu_usage: avg_cpu,
            memory_usage: avg_memory,
            disk_read_bytes: avg_disk_read,
            disk_write_bytes: avg_disk_write,
            network_rx_bytes: avg_network_rx,
            network_tx_bytes: avg_network_tx,
            thread_count: relevant_stats.last().unwrap().thread_count,
            uptime: relevant_stats.last().unwrap().uptime,
        })
    }

    /// Detect performance anomalies
    pub fn detect_anomalies(&self) -> Vec<PerformanceAnomaly> {
        let recent_stats = self.get_recent_stats(Duration::from_minutes(5));
        let mut anomalies = Vec::new();

        if recent_stats.len() < 2 {
            return anomalies;
        }

        // Check for CPU spikes
        let avg_cpu = recent_stats.iter().map(|s| s.cpu_usage).sum::<f64>() / recent_stats.len() as f64;
        let latest_cpu = recent_stats.last().unwrap().cpu_usage;
        
        if latest_cpu > avg_cpu * 2.0 && latest_cpu > 80.0 {
            anomalies.push(PerformanceAnomaly {
                anomaly_type: AnomalyType::CpuSpike,
                severity: if latest_cpu > 95.0 { Severity::Critical } else { Severity::Warning },
                description: format!("CPU usage spike: {:.1}% (avg: {:.1}%)", latest_cpu, avg_cpu),
                timestamp: SystemTime::now(),
                metric_value: latest_cpu,
                threshold: 80.0,
            });
        }

        // Check for memory pressure
        let latest_memory = recent_stats.last().unwrap().memory_usage;
        let memory_mb = latest_memory as f64 / (1024.0 * 1024.0);
        
        if memory_mb > 500.0 { // 500MB threshold
            anomalies.push(PerformanceAnomaly {
                anomaly_type: AnomalyType::MemoryPressure,
                severity: if memory_mb > 1000.0 { Severity::Critical } else { Severity::Warning },
                description: format!("High memory usage: {:.1}MB", memory_mb),
                timestamp: SystemTime::now(),
                metric_value: memory_mb,
                threshold: 500.0,
            });
        }

        // Check for excessive disk I/O
        let recent_disk_activity: u64 = recent_stats.iter()
            .map(|s| s.disk_read_bytes + s.disk_write_bytes)
            .sum();
        
        let disk_mb_per_sec = (recent_disk_activity as f64 / (1024.0 * 1024.0)) / recent_stats.len() as f64;
        
        if disk_mb_per_sec > 50.0 { // 50MB/s threshold
            anomalies.push(PerformanceAnomaly {
                anomaly_type: AnomalyType::DiskIoSpike,
                severity: if disk_mb_per_sec > 100.0 { Severity::Critical } else { Severity::Warning },
                description: format!("High disk I/O: {:.1}MB/s", disk_mb_per_sec),
                timestamp: SystemTime::now(),
                metric_value: disk_mb_per_sec,
                threshold: 50.0,
            });
        }

        anomalies
    }

    fn get_recent_stats(&self, duration: Duration) -> Vec<SystemStats> {
        let history = self.stats_history.read().unwrap();
        let cutoff_time = SystemTime::now() - duration;
        
        history
            .iter()
            .filter(|stats| stats.timestamp >= cutoff_time)
            .cloned()
            .collect()
    }

    fn collect_system_stats() -> SystemStats {
        use memory_stats::memory_stats;
        
        let memory_info = memory_stats().unwrap_or_default();
        
        SystemStats {
            timestamp: SystemTime::now(),
            cpu_usage: Self::get_cpu_usage(),
            memory_usage: memory_info.physical_mem as u64,
            disk_read_bytes: 0, // Would need platform-specific implementation
            disk_write_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            thread_count: Self::get_thread_count(),
            uptime: Self::get_uptime(),
        }
    }

    fn get_cpu_usage() -> f64 {
        // Platform-specific CPU usage implementation would go here
        // For now, return a placeholder
        0.0
    }

    fn get_thread_count() -> u32 {
        // Platform-specific thread count implementation
        std::thread::available_parallelism()
            .map(|p| p.get() as u32)
            .unwrap_or(1)
    }

    fn get_uptime() -> Duration {
        // Platform-specific uptime implementation
        Duration::from_secs(0)
    }
}

/// System statistics snapshot
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub timestamp: SystemTime,
    pub cpu_usage: f64,        // Percentage (0-100)
    pub memory_usage: u64,     // Bytes
    pub disk_read_bytes: u64,  // Bytes per second
    pub disk_write_bytes: u64, // Bytes per second
    pub network_rx_bytes: u64, // Bytes per second
    pub network_tx_bytes: u64, // Bytes per second
    pub thread_count: u32,
    pub uptime: Duration,
}

impl Default for SystemStats {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            cpu_usage: 0.0,
            memory_usage: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            network_rx_bytes: 0,
            network_tx_bytes: 0,
            thread_count: 1,
            uptime: Duration::default(),
        }
    }
}

/// Monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    pub collection_interval: Duration,
    pub max_history_size: usize,
    pub enable_anomaly_detection: bool,
    pub anomaly_check_interval: Duration,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(1),
            max_history_size: 3600, // 1 hour of 1-second intervals
            enable_anomaly_detection: true,
            anomaly_check_interval: Duration::from_secs(10),
        }
    }
}

/// Performance anomaly detection
#[derive(Debug, Clone)]
pub struct PerformanceAnomaly {
    pub anomaly_type: AnomalyType,
    pub severity: Severity,
    pub description: String,
    pub timestamp: SystemTime,
    pub metric_value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnomalyType {
    CpuSpike,
    MemoryPressure,
    DiskIoSpike,
    NetworkSpike,
    ThreadCountSpike,
    ResponseTimeSpike,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

/// Real-time performance dashboard data
#[derive(Debug, Clone)]
pub struct DashboardData {
    pub current_stats: SystemStats,
    pub recent_history: Vec<SystemStats>,
    pub anomalies: Vec<PerformanceAnomaly>,
    pub performance_score: f64,
    pub recommendations: Vec<String>,
}

impl DashboardData {
    pub fn new(monitor: &SystemMonitor) -> Self {
        let current_stats = monitor.get_current_stats();
        let recent_history = monitor.get_recent_stats(Duration::from_minutes(5));
        let anomalies = monitor.detect_anomalies();
        
        let performance_score = Self::calculate_performance_score(&current_stats);
        let recommendations = Self::generate_recommendations(&current_stats, &anomalies);

        Self {
            current_stats,
            recent_history,
            anomalies,
            performance_score,
            recommendations,
        }
    }

    fn calculate_performance_score(stats: &SystemStats) -> f64 {
        // Calculate a performance score from 0-100 based on various metrics
        let cpu_score = ((100.0 - stats.cpu_usage) / 100.0) * 30.0;
        let memory_score = if stats.memory_usage < 100 * 1024 * 1024 { 30.0 } else { 15.0 };
        let responsiveness_score = 40.0; // Would be based on actual response times
        
        cpu_score + memory_score + responsiveness_score
    }

    fn generate_recommendations(stats: &SystemStats, anomalies: &[PerformanceAnomaly]) -> Vec<String> {
        let mut recommendations = Vec::new();

        if stats.cpu_usage > 80.0 {
            recommendations.push("Consider optimizing CPU-intensive operations".to_string());
        }

        if stats.memory_usage > 500 * 1024 * 1024 {
            recommendations.push("Memory usage is high, consider enabling more aggressive caching".to_string());
        }

        if anomalies.iter().any(|a| a.severity == Severity::Critical) {
            recommendations.push("Critical performance issues detected, immediate attention required".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("System performance is optimal".to_string());
        }

        recommendations
    }
}

/// Performance alerting system
pub struct AlertingSystem {
    subscribers: Arc<RwLock<Vec<Box<dyn AlertSubscriber + Send + Sync>>>>,
}

impl AlertingSystem {
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn subscribe(&self, subscriber: Box<dyn AlertSubscriber + Send + Sync>) {
        let mut subscribers = self.subscribers.write().unwrap();
        subscribers.push(subscriber);
    }

    pub fn send_alert(&self, anomaly: &PerformanceAnomaly) {
        let subscribers = self.subscribers.read().unwrap();
        for subscriber in subscribers.iter() {
            subscriber.on_alert(anomaly);
        }
    }
}

/// Trait for alert subscribers
pub trait AlertSubscriber {
    fn on_alert(&self, anomaly: &PerformanceAnomaly);
}

/// Console alert subscriber
pub struct ConsoleAlertSubscriber;

impl AlertSubscriber for ConsoleAlertSubscriber {
    fn on_alert(&self, anomaly: &PerformanceAnomaly) {
        println!(
            "[{}] {:?}: {}",
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
            anomaly.severity,
            anomaly.description
        );
    }
}