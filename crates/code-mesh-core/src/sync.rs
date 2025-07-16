//! Synchronization primitives for Code Mesh Core
//!
//! This module provides cross-platform synchronization primitives that work
//! consistently across native and WASM environments.

use crate::{Error, Result};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(feature = "native")]
use tokio::sync::{Mutex as TokioMutex, RwLock as TokioRwLock};

#[cfg(feature = "wasm")]
use parking_lot::{Mutex as ParkingMutex, RwLock as ParkingRwLock};

/// Cross-platform async mutex
#[cfg(feature = "native")]
pub type AsyncMutex<T> = TokioMutex<T>;

#[cfg(feature = "wasm")]
pub type AsyncMutex<T> = ParkingMutex<T>;

/// Cross-platform async read-write lock
#[cfg(feature = "native")]
pub type AsyncRwLock<T> = TokioRwLock<T>;

#[cfg(feature = "wasm")]
pub type AsyncRwLock<T> = ParkingRwLock<T>;

/// Debouncer for rate-limiting operations
pub struct Debouncer {
    last_execution: Arc<AsyncMutex<Option<Instant>>>,
    delay: Duration,
}

impl Debouncer {
    /// Create a new debouncer with the specified delay
    pub fn new(delay: Duration) -> Self {
        Self {
            last_execution: Arc::new(AsyncMutex::new(None)),
            delay,
        }
    }

    /// Execute a function with debouncing
    #[cfg(feature = "native")]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<Option<T>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let now = Instant::now();
        let mut last_exec = self.last_execution.lock().await;
        
        if let Some(last) = *last_exec {
            if now.duration_since(last) < self.delay {
                return Ok(None);
            }
        }
        
        *last_exec = Some(now);
        drop(last_exec);
        
        f().await.map(Some)
    }

    /// Execute a function with debouncing (WASM version)
    #[cfg(feature = "wasm")]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<Option<T>>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let now = Instant::now();
        let mut last_exec = self.last_execution.lock();
        
        if let Some(last) = *last_exec {
            if now.duration_since(last) < self.delay {
                return Ok(None);
            }
        }
        
        *last_exec = Some(now);
        drop(last_exec);
        
        f().await.map(Some)
    }

    /// Check if enough time has passed since the last execution
    #[cfg(feature = "native")]
    pub async fn should_execute(&self) -> bool {
        let now = Instant::now();
        let last_exec = self.last_execution.lock().await;
        
        if let Some(last) = *last_exec {
            now.duration_since(last) >= self.delay
        } else {
            true
        }
    }

    /// Check if enough time has passed since the last execution (WASM version)
    #[cfg(feature = "wasm")]
    pub async fn should_execute(&self) -> bool {
        let now = Instant::now();
        let last_exec = self.last_execution.lock();
        
        if let Some(last) = *last_exec {
            now.duration_since(last) >= self.delay
        } else {
            true
        }
    }
}

/// Rate limiter for controlling operation frequency
pub struct RateLimiter {
    tokens: Arc<AsyncMutex<f64>>,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Arc<AsyncMutex<Instant>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_tokens: f64, refill_rate: f64) -> Self {
        Self {
            tokens: Arc::new(AsyncMutex::new(max_tokens)),
            max_tokens,
            refill_rate,
            last_refill: Arc::new(AsyncMutex::new(Instant::now())),
        }
    }

    /// Try to acquire a token (non-blocking)
    #[cfg(feature = "native")]
    pub async fn try_acquire(&self, tokens: f64) -> bool {
        self.refill_tokens().await;
        
        let mut current_tokens = self.tokens.lock().await;
        if *current_tokens >= tokens {
            *current_tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Try to acquire a token (non-blocking, WASM version)
    #[cfg(feature = "wasm")]
    pub async fn try_acquire(&self, tokens: f64) -> bool {
        self.refill_tokens().await;
        
        let mut current_tokens = self.tokens.lock();
        if *current_tokens >= tokens {
            *current_tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Acquire a token (blocking until available)
    #[cfg(feature = "native")]
    pub async fn acquire(&self, tokens: f64) -> Result<()> {
        loop {
            if self.try_acquire(tokens).await {
                return Ok(());
            }
            
            // Calculate wait time
            let wait_time = Duration::from_secs_f64(tokens / self.refill_rate);
            tokio::time::sleep(wait_time).await;
        }
    }

    /// Acquire a token (blocking until available, WASM version)
    #[cfg(feature = "wasm")]
    pub async fn acquire(&self, tokens: f64) -> Result<()> {
        loop {
            if self.try_acquire(tokens).await {
                return Ok(());
            }
            
            // In WASM, we can't really sleep, so we yield
            wasm_bindgen_futures::JsFuture::from(
                js_sys::Promise::resolve(&wasm_bindgen::JsValue::UNDEFINED)
            ).await.map_err(|_| Error::Other(anyhow::anyhow!("JS Promise failed")))?;
        }
    }

    /// Refill tokens based on elapsed time
    #[cfg(feature = "native")]
    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.lock().await;
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        
        if elapsed > 0.0 {
            let mut tokens = self.tokens.lock().await;
            let new_tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
            *tokens = new_tokens;
            *last_refill = now;
        }
    }

    /// Refill tokens based on elapsed time (WASM version)
    #[cfg(feature = "wasm")]
    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.last_refill.lock();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        
        if elapsed > 0.0 {
            let mut tokens = self.tokens.lock();
            let new_tokens = (*tokens + elapsed * self.refill_rate).min(self.max_tokens);
            *tokens = new_tokens;
            *last_refill = now;
        }
    }

    /// Get current token count
    #[cfg(feature = "native")]
    pub async fn tokens(&self) -> f64 {
        self.refill_tokens().await;
        *self.tokens.lock().await
    }

    /// Get current token count (WASM version)
    #[cfg(feature = "wasm")]
    pub async fn tokens(&self) -> f64 {
        self.refill_tokens().await;
        *self.tokens.lock()
    }
}

/// Circuit breaker for handling failures gracefully
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

pub struct CircuitBreaker {
    state: Arc<AsyncMutex<CircuitState>>,
    failure_count: Arc<AsyncMutex<u32>>,
    success_count: Arc<AsyncMutex<u32>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: Duration,
    last_failure: Arc<AsyncMutex<Option<Instant>>>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            state: Arc::new(AsyncMutex::new(CircuitState::Closed)),
            failure_count: Arc::new(AsyncMutex::new(0)),
            success_count: Arc::new(AsyncMutex::new(0)),
            failure_threshold,
            success_threshold,
            timeout,
            last_failure: Arc::new(AsyncMutex::new(None)),
        }
    }

    /// Execute a function with circuit breaker protection
    #[cfg(feature = "native")]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit is open
        let state = *self.state.lock().await;
        match state {
            CircuitState::Open => {
                let last_failure = *self.last_failure.lock().await;
                if let Some(failure_time) = last_failure {
                    if Instant::now().duration_since(failure_time) >= self.timeout {
                        // Transition to half-open
                        *self.state.lock().await = CircuitState::HalfOpen;
                        *self.success_count.lock().await = 0;
                    } else {
                        return Err(Error::Other(anyhow::anyhow!("Circuit breaker is open")));
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }

    /// Execute a function with circuit breaker protection (WASM version)
    #[cfg(feature = "wasm")]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // Check if circuit is open
        let state = *self.state.lock();
        match state {
            CircuitState::Open => {
                let last_failure = *self.last_failure.lock();
                if let Some(failure_time) = last_failure {
                    if Instant::now().duration_since(failure_time) >= self.timeout {
                        // Transition to half-open
                        *self.state.lock() = CircuitState::HalfOpen;
                        *self.success_count.lock() = 0;
                    } else {
                        return Err(Error::Other(anyhow::anyhow!("Circuit breaker is open")));
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Execute the function
        match f().await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(error) => {
                self.on_failure().await;
                Err(error)
            }
        }
    }

    /// Handle successful execution
    #[cfg(feature = "native")]
    async fn on_success(&self) {
        let state = *self.state.lock().await;
        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.lock().await;
                *success_count += 1;
                if *success_count >= self.success_threshold {
                    *self.state.lock().await = CircuitState::Closed;
                    *self.failure_count.lock().await = 0;
                }
            }
            CircuitState::Closed => {
                *self.failure_count.lock().await = 0;
            }
            _ => {}
        }
    }

    /// Handle successful execution (WASM version)
    #[cfg(feature = "wasm")]
    async fn on_success(&self) {
        let state = *self.state.lock();
        match state {
            CircuitState::HalfOpen => {
                let mut success_count = self.success_count.lock();
                *success_count += 1;
                if *success_count >= self.success_threshold {
                    *self.state.lock() = CircuitState::Closed;
                    *self.failure_count.lock() = 0;
                }
            }
            CircuitState::Closed => {
                *self.failure_count.lock() = 0;
            }
            _ => {}
        }
    }

    /// Handle failed execution
    #[cfg(feature = "native")]
    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.lock().await;
        *failure_count += 1;
        *self.last_failure.lock().await = Some(Instant::now());

        if *failure_count >= self.failure_threshold {
            *self.state.lock().await = CircuitState::Open;
        }
    }

    /// Handle failed execution (WASM version)
    #[cfg(feature = "wasm")]
    async fn on_failure(&self) {
        let mut failure_count = self.failure_count.lock();
        *failure_count += 1;
        *self.last_failure.lock() = Some(Instant::now());

        if *failure_count >= self.failure_threshold {
            *self.state.lock() = CircuitState::Open;
        }
    }

    /// Get current circuit state
    #[cfg(feature = "native")]
    pub async fn state(&self) -> CircuitState {
        *self.state.lock().await
    }

    /// Get current circuit state (WASM version)
    #[cfg(feature = "wasm")]
    pub async fn state(&self) -> CircuitState {
        *self.state.lock()
    }
}

/// Timeout wrapper for operations
pub struct TimeoutWrapper {
    timeout: Duration,
}

impl TimeoutWrapper {
    /// Create a new timeout wrapper
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    /// Execute a function with timeout
    #[cfg(feature = "native")]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        match tokio::time::timeout(self.timeout, f()).await {
            Ok(result) => result,
            Err(_) => Err(Error::Other(anyhow::anyhow!("Operation timed out"))),
        }
    }

    /// Execute a function with timeout (WASM version - simplified)
    #[cfg(feature = "wasm")]
    pub async fn execute<F, Fut, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // WASM doesn't have real timeouts, so we just execute the function
        f().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_debouncer() {
        let debouncer = Debouncer::new(Duration::from_millis(100));
        let mut counter = 0;

        // First call should execute
        let result = debouncer.execute(|| async {
            counter += 1;
            Ok(counter)
        }).await.unwrap();
        assert_eq!(result, Some(1));

        // Immediate second call should be debounced
        let result = debouncer.execute(|| async {
            counter += 1;
            Ok(counter)
        }).await.unwrap();
        assert_eq!(result, None);

        // Wait for debounce period
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Now it should execute again
        let result = debouncer.execute(|| async {
            counter += 1;
            Ok(counter)
        }).await.unwrap();
        assert_eq!(result, Some(2));
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2.0, 1.0); // 2 tokens, refill 1 per second

        // Should be able to acquire 2 tokens immediately
        assert!(limiter.try_acquire(1.0).await);
        assert!(limiter.try_acquire(1.0).await);
        
        // Third token should fail
        assert!(!limiter.try_acquire(1.0).await);

        // Wait for refill
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // Should be able to acquire one more token
        assert!(limiter.try_acquire(1.0).await);
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(2, 1, Duration::from_millis(100));
        
        // Should start closed
        assert_eq!(breaker.state().await, CircuitState::Closed);

        // Fail twice to open the circuit
        let _result = breaker.execute(|| async { 
            Err::<(), _>(Error::Other(anyhow::anyhow!("Test error")))
        }).await;
        let _result = breaker.execute(|| async { 
            Err::<(), _>(Error::Other(anyhow::anyhow!("Test error")))
        }).await;

        // Circuit should now be open
        assert_eq!(breaker.state().await, CircuitState::Open);

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open on next call
        let _result = breaker.execute(|| async { Ok(()) }).await;
        assert_eq!(breaker.state().await, CircuitState::Closed);
    }
}