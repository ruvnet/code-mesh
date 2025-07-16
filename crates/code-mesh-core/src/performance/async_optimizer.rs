//! Async runtime optimizations and connection pooling

use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use futures::future::BoxFuture;
use std::collections::HashMap;
use parking_lot::RwLock;

/// Async runtime optimizer for improved performance
pub struct AsyncOptimizer {
    connection_pools: Arc<RwLock<HashMap<String, Arc<ConnectionPool>>>>,
    request_batcher: Arc<RequestBatcher>,
    task_scheduler: Arc<TaskScheduler>,
}

impl AsyncOptimizer {
    pub fn new() -> Self {
        Self {
            connection_pools: Arc::new(RwLock::new(HashMap::new())),
            request_batcher: Arc::new(RequestBatcher::new()),
            task_scheduler: Arc::new(TaskScheduler::new()),
        }
    }

    /// Get or create a connection pool for a specific host
    pub fn get_connection_pool(&self, host: &str) -> Arc<ConnectionPool> {
        let pools = self.connection_pools.read();
        if let Some(pool) = pools.get(host) {
            return pool.clone();
        }
        drop(pools);

        let mut pools = self.connection_pools.write();
        let pool = Arc::new(ConnectionPool::new(host, ConnectionPoolConfig::default()));
        pools.insert(host.to_string(), pool.clone());
        pool
    }

    /// Batch multiple requests for efficiency
    pub async fn batch_requests<T, F>(&self, requests: Vec<F>) -> Vec<T>
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.request_batcher.batch_execute(requests).await
    }

    /// Schedule a task with optimization
    pub async fn schedule_task<T, F>(&self, task: F) -> T
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.task_scheduler.schedule(task).await
    }
}

/// HTTP connection pool for efficient connection reuse
pub struct ConnectionPool {
    host: String,
    config: ConnectionPoolConfig,
    active_connections: Arc<RwLock<Vec<Connection>>>,
    idle_connections: Arc<RwLock<Vec<Connection>>>,
    stats: Arc<RwLock<ConnectionPoolStats>>,
}

impl ConnectionPool {
    pub fn new(host: &str, config: ConnectionPoolConfig) -> Self {
        Self {
            host: host.to_string(),
            config,
            active_connections: Arc::new(RwLock::new(Vec::new())),
            idle_connections: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(ConnectionPoolStats::new())),
        }
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self) -> Result<PooledConnection, ConnectionError> {
        // Try to get an idle connection first
        {
            let mut idle = self.idle_connections.write();
            if let Some(conn) = idle.pop() {
                if conn.is_healthy() {
                    let mut active = self.active_connections.write();
                    active.push(conn.clone());
                    
                    let mut stats = self.stats.write();
                    stats.connections_reused += 1;
                    
                    return Ok(PooledConnection::new(conn, self.clone()));
                }
            }
        }

        // Create a new connection if under limit
        {
            let active = self.active_connections.read();
            if active.len() < self.config.max_connections {
                drop(active);
                
                let conn = self.create_connection().await?;
                let mut active = self.active_connections.write();
                active.push(conn.clone());
                
                let mut stats = self.stats.write();
                stats.connections_created += 1;
                
                return Ok(PooledConnection::new(conn, self.clone()));
            }
        }

        // Wait for a connection to become available
        let start_time = Instant::now();
        loop {
            if start_time.elapsed() > self.config.connection_timeout {
                return Err(ConnectionError::Timeout);
            }

            tokio::time::sleep(Duration::from_millis(10)).await;

            let mut idle = self.idle_connections.write();
            if let Some(conn) = idle.pop() {
                if conn.is_healthy() {
                    let mut active = self.active_connections.write();
                    active.push(conn.clone());
                    
                    let mut stats = self.stats.write();
                    stats.connections_reused += 1;
                    
                    return Ok(PooledConnection::new(conn, self.clone()));
                }
            }
        }
    }

    /// Return a connection to the pool
    pub fn return_connection(&self, connection: Connection) {
        let mut active = self.active_connections.write();
        if let Some(pos) = active.iter().position(|c| c.id == connection.id) {
            active.remove(pos);
        }

        if connection.is_healthy() && connection.can_be_reused() {
            let mut idle = self.idle_connections.write();
            if idle.len() < self.config.max_idle_connections {
                idle.push(connection);
            }
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> ConnectionPoolStats {
        self.stats.read().clone()
    }

    async fn create_connection(&self) -> Result<Connection, ConnectionError> {
        let client = reqwest::Client::builder()
            .timeout(self.config.request_timeout)
            .pool_max_idle_per_host(self.config.max_idle_connections)
            .pool_idle_timeout(self.config.idle_timeout)
            .build()
            .map_err(|_| ConnectionError::CreationFailed)?;

        Ok(Connection {
            id: uuid::Uuid::new_v4(),
            host: self.host.clone(),
            client,
            created_at: Instant::now(),
            last_used: Instant::now(),
            request_count: 0,
        })
    }
}

/// Configuration for connection pools
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    pub max_connections: usize,
    pub max_idle_connections: usize,
    pub connection_timeout: Duration,
    pub request_timeout: Duration,
    pub idle_timeout: Duration,
    pub max_connection_lifetime: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 20,
            max_idle_connections: 10,
            connection_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(300),
            max_connection_lifetime: Duration::from_secs(3600),
        }
    }
}

/// A connection in the pool
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: uuid::Uuid,
    pub host: String,
    pub client: reqwest::Client,
    pub created_at: Instant,
    pub last_used: Instant,
    pub request_count: u64,
}

impl Connection {
    pub fn is_healthy(&self) -> bool {
        // Check if connection is still healthy
        let age = self.created_at.elapsed();
        let idle_time = self.last_used.elapsed();
        
        age < Duration::from_secs(3600) && // Max 1 hour age
        idle_time < Duration::from_secs(300) // Max 5 minutes idle
    }

    pub fn can_be_reused(&self) -> bool {
        self.request_count < 1000 // Max 1000 requests per connection
    }
}

/// Pooled connection wrapper
pub struct PooledConnection {
    connection: Option<Connection>,
    pool: ConnectionPool,
}

impl PooledConnection {
    fn new(connection: Connection, pool: ConnectionPool) -> Self {
        Self {
            connection: Some(connection),
            pool,
        }
    }

    pub fn client(&self) -> &reqwest::Client {
        &self.connection.as_ref().unwrap().client
    }
}

impl Drop for PooledConnection {
    fn drop(&mut self) {
        if let Some(mut connection) = self.connection.take() {
            connection.last_used = Instant::now();
            connection.request_count += 1;
            self.pool.return_connection(connection);
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub connections_created: u64,
    pub connections_reused: u64,
    pub connections_dropped: u64,
    pub total_requests: u64,
    pub average_request_time: Duration,
}

impl ConnectionPoolStats {
    fn new() -> Self {
        Self {
            connections_created: 0,
            connections_reused: 0,
            connections_dropped: 0,
            total_requests: 0,
            average_request_time: Duration::default(),
        }
    }

    pub fn connection_reuse_rate(&self) -> f64 {
        let total = self.connections_created + self.connections_reused;
        if total == 0 {
            0.0
        } else {
            self.connections_reused as f64 / total as f64
        }
    }
}

/// Request batcher for grouping similar requests
pub struct RequestBatcher {
    batch_size: usize,
    batch_timeout: Duration,
}

impl RequestBatcher {
    pub fn new() -> Self {
        Self {
            batch_size: 10,
            batch_timeout: Duration::from_millis(100),
        }
    }

    /// Execute multiple futures concurrently in batches
    pub async fn batch_execute<T, F>(&self, futures: Vec<F>) -> Vec<T>
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let mut results = Vec::with_capacity(futures.len());
        
        // Process futures in batches
        for chunk in futures.chunks(self.batch_size) {
            let batch_futures: Vec<_> = chunk.into_iter().collect();
            let batch_results = futures::future::join_all(batch_futures).await;
            results.extend(batch_results);
        }

        results
    }

    /// Batch similar requests together
    pub async fn batch_similar_requests<T, F, K>(&self, requests: Vec<(K, F)>) -> HashMap<K, T>
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + Clone + 'static,
        K: std::hash::Hash + Eq + Clone + Send + 'static,
    {
        // Group requests by key and deduplicate
        let mut grouped: HashMap<K, F> = HashMap::new();
        for (key, future) in requests {
            grouped.insert(key, future);
        }

        // Execute deduplicated requests
        let mut results = HashMap::new();
        let futures: Vec<_> = grouped.into_iter().collect();
        
        for chunk in futures.chunks(self.batch_size) {
            let batch_futures: Vec<_> = chunk.iter()
                .map(|(key, future)| async move {
                    let result = future.await;
                    (key.clone(), result)
                })
                .collect();
            
            let batch_results = futures::future::join_all(batch_futures).await;
            for (key, result) in batch_results {
                results.insert(key, result);
            }
        }

        results
    }
}

/// Optimized task scheduler
pub struct TaskScheduler {
    high_priority_queue: Arc<RwLock<Vec<BoxFuture<'static, ()>>>>,
    normal_priority_queue: Arc<RwLock<Vec<BoxFuture<'static, ()>>>>,
    low_priority_queue: Arc<RwLock<Vec<BoxFuture<'static, ()>>>>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            high_priority_queue: Arc::new(RwLock::new(Vec::new())),
            normal_priority_queue: Arc::new(RwLock::new(Vec::new())),
            low_priority_queue: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Schedule a task with normal priority
    pub async fn schedule<T, F>(&self, future: F) -> T
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // For now, just execute directly
        // In a full implementation, this would use a proper scheduler
        future.await
    }

    /// Schedule a high-priority task
    pub async fn schedule_high_priority<T, F>(&self, future: F) -> T
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        future.await
    }

    /// Schedule a low-priority task
    pub async fn schedule_low_priority<T, F>(&self, future: F) -> T
    where
        F: futures::Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        future.await
    }
}

/// Connection errors
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("Connection timeout")]
    Timeout,
    #[error("Failed to create connection")]
    CreationFailed,
    #[error("Pool exhausted")]
    PoolExhausted,
    #[error("Connection unhealthy")]
    Unhealthy,
}

/// Global async optimizer instance
static GLOBAL_ASYNC_OPTIMIZER: once_cell::sync::Lazy<AsyncOptimizer> = 
    once_cell::sync::Lazy::new(|| AsyncOptimizer::new());

/// Get the global async optimizer
pub fn global_async_optimizer() -> &'static AsyncOptimizer {
    &GLOBAL_ASYNC_OPTIMIZER
}

/// Convenience macros for async optimization
#[macro_export]
macro_rules! optimized_request {
    ($host:expr, $request:expr) => {{
        let pool = $crate::performance::async_optimizer::global_async_optimizer()
            .get_connection_pool($host);
        let connection = pool.get_connection().await?;
        $request(connection.client()).await
    }};
}

#[macro_export]
macro_rules! batch_requests {
    ($($request:expr),*) => {{
        let requests = vec![$($request),*];
        $crate::performance::async_optimizer::global_async_optimizer()
            .batch_requests(requests).await
    }};
}