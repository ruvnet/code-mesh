//! Memory pooling for reduced allocations and improved performance

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::marker::PhantomData;

/// Object pool for reusing expensive-to-create objects
pub struct ObjectPool<T> {
    objects: Arc<Mutex<VecDeque<T>>>,
    factory: Box<dyn Fn() -> T + Send + Sync>,
    reset_fn: Option<Box<dyn Fn(&mut T) + Send + Sync>>,
    max_size: usize,
}

impl<T> ObjectPool<T>
where
    T: Send + 'static,
{
    /// Create a new object pool
    pub fn new<F>(factory: F, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            objects: Arc::new(Mutex::new(VecDeque::new())),
            factory: Box::new(factory),
            reset_fn: None,
            max_size,
        }
    }

    /// Create a new object pool with a reset function
    pub fn with_reset<F, R>(factory: F, reset_fn: R, max_size: usize) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
        R: Fn(&mut T) + Send + Sync + 'static,
    {
        Self {
            objects: Arc::new(Mutex::new(VecDeque::new())),
            factory: Box::new(factory),
            reset_fn: Some(Box::new(reset_fn)),
            max_size,
        }
    }

    /// Get an object from the pool or create a new one
    pub fn get(&self) -> PooledObject<T> {
        let object = {
            let mut objects = self.objects.lock().unwrap();
            objects.pop_front().unwrap_or_else(|| (self.factory)())
        };

        PooledObject {
            object: Some(object),
            pool: self.objects.clone(),
            reset_fn: self.reset_fn.as_ref().map(|f| f.as_ref()),
            max_size: self.max_size,
        }
    }

    /// Get current pool size
    pub fn size(&self) -> usize {
        self.objects.lock().unwrap().len()
    }

    /// Clear the pool
    pub fn clear(&self) {
        self.objects.lock().unwrap().clear();
    }
}

/// RAII wrapper for pooled objects
pub struct PooledObject<T> {
    object: Option<T>,
    pool: Arc<Mutex<VecDeque<T>>>,
    reset_fn: Option<&'static dyn Fn(&mut T)>,
    max_size: usize,
}

impl<T> PooledObject<T> {
    /// Get a reference to the wrapped object
    pub fn as_ref(&self) -> &T {
        self.object.as_ref().unwrap()
    }

    /// Get a mutable reference to the wrapped object
    pub fn as_mut(&mut self) -> &mut T {
        self.object.as_mut().unwrap()
    }

    /// Take ownership of the object (will not be returned to pool)
    pub fn take(mut self) -> T {
        self.object.take().unwrap()
    }
}

impl<T> std::ops::Deref for PooledObject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> std::ops::DerefMut for PooledObject<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(mut object) = self.object.take() {
            // Reset the object if a reset function is provided
            if let Some(reset_fn) = self.reset_fn {
                reset_fn(&mut object);
            }

            // Return to pool if there's space
            let mut pool = self.pool.lock().unwrap();
            if pool.len() < self.max_size {
                pool.push_back(object);
            }
            // Otherwise, object is dropped
        }
    }
}

/// Memory pool for byte buffers
pub struct BufferPool {
    pool: ObjectPool<Vec<u8>>,
}

impl BufferPool {
    /// Create a new buffer pool
    pub fn new(initial_capacity: usize, max_pool_size: usize) -> Self {
        let pool = ObjectPool::with_reset(
            move || Vec::with_capacity(initial_capacity),
            |buffer| buffer.clear(),
            max_pool_size,
        );

        Self { pool }
    }

    /// Get a buffer from the pool
    pub fn get(&self) -> PooledObject<Vec<u8>> {
        self.pool.get()
    }

    /// Get a buffer with specific capacity
    pub fn get_with_capacity(&self, capacity: usize) -> PooledObject<Vec<u8>> {
        let mut buffer = self.pool.get();
        if buffer.capacity() < capacity {
            buffer.reserve(capacity - buffer.len());
        }
        buffer
    }
}

/// String pool for reducing string allocations
pub struct StringPool {
    pool: ObjectPool<String>,
}

impl StringPool {
    /// Create a new string pool
    pub fn new(max_pool_size: usize) -> Self {
        let pool = ObjectPool::with_reset(
            || String::new(),
            |string| string.clear(),
            max_pool_size,
        );

        Self { pool }
    }

    /// Get a string from the pool
    pub fn get(&self) -> PooledObject<String> {
        self.pool.get()
    }

    /// Get a string with specific capacity
    pub fn get_with_capacity(&self, capacity: usize) -> PooledObject<String> {
        let mut string = self.pool.get();
        if string.capacity() < capacity {
            string.reserve(capacity - string.len());
        }
        string
    }
}

/// Pool manager for coordinating multiple pools
pub struct PoolManager {
    buffer_pool: BufferPool,
    string_pool: StringPool,
    stats: Arc<Mutex<PoolStats>>,
}

impl PoolManager {
    /// Create a new pool manager
    pub fn new() -> Self {
        Self {
            buffer_pool: BufferPool::new(8192, 100), // 8KB initial, max 100 buffers
            string_pool: StringPool::new(50),
            stats: Arc::new(Mutex::new(PoolStats::new())),
        }
    }

    /// Get buffer pool
    pub fn buffer_pool(&self) -> &BufferPool {
        &self.buffer_pool
    }

    /// Get string pool
    pub fn string_pool(&self) -> &StringPool {
        &self.string_pool
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        self.stats.lock().unwrap().clone()
    }

    /// Clear all pools
    pub fn clear_all(&self) {
        self.buffer_pool.pool.clear();
        self.string_pool.pool.clear();
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub buffer_pool_size: usize,
    pub string_pool_size: usize,
    pub total_allocations_saved: u64,
    pub memory_reused_bytes: u64,
}

impl PoolStats {
    fn new() -> Self {
        Self {
            buffer_pool_size: 0,
            string_pool_size: 0,
            total_allocations_saved: 0,
            memory_reused_bytes: 0,
        }
    }
}

/// Global pool manager instance
static GLOBAL_POOL_MANAGER: once_cell::sync::Lazy<PoolManager> = 
    once_cell::sync::Lazy::new(|| PoolManager::new());

/// Get the global pool manager
pub fn global_pools() -> &'static PoolManager {
    &GLOBAL_POOL_MANAGER
}

/// Specialized pools for common use cases

/// Pool for HTTP request/response bodies
pub type HttpBodyPool = BufferPool;

/// Pool for file content buffers
pub type FileContentPool = BufferPool;

/// Pool for tokenization results
pub type TokenPool = ObjectPool<Vec<String>>;

/// Create specialized token pool
pub fn create_token_pool() -> TokenPool {
    ObjectPool::with_reset(
        || Vec::with_capacity(1000),
        |tokens| tokens.clear(),
        20,
    )
}

/// Pool factory for creating optimized pools
pub struct PoolFactory;

impl PoolFactory {
    /// Create an optimized HTTP body pool
    pub fn create_http_body_pool() -> HttpBodyPool {
        BufferPool::new(32768, 50) // 32KB initial, max 50 buffers
    }

    /// Create an optimized file content pool
    pub fn create_file_content_pool() -> FileContentPool {
        BufferPool::new(65536, 25) // 64KB initial, max 25 buffers
    }

    /// Create an optimized response buffer pool
    pub fn create_response_pool() -> BufferPool {
        BufferPool::new(16384, 75) // 16KB initial, max 75 buffers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_object_pool_basic() {
        let pool = ObjectPool::new(|| Vec::<i32>::new(), 5);
        
        let mut obj1 = pool.get();
        obj1.push(1);
        obj1.push(2);
        
        drop(obj1); // Should return to pool
        
        let obj2 = pool.get();
        // Should be empty due to reset function
        assert_eq!(obj2.len(), 0);
    }

    #[test]
    fn test_buffer_pool() {
        let pool = BufferPool::new(1024, 3);
        
        let mut buffer = pool.get();
        buffer.extend_from_slice(b"test data");
        drop(buffer);
        
        let buffer2 = pool.get();
        assert_eq!(buffer2.len(), 0); // Should be cleared
        assert!(buffer2.capacity() >= 1024);
    }

    #[test]
    fn test_string_pool() {
        let pool = StringPool::new(3);
        
        let mut string = pool.get();
        string.push_str("test");
        drop(string);
        
        let string2 = pool.get();
        assert_eq!(string2.len(), 0); // Should be cleared
    }

    #[test]
    fn test_pool_size_limit() {
        let pool = ObjectPool::new(|| Vec::<i32>::new(), 2);
        
        let obj1 = pool.get();
        let obj2 = pool.get();
        let obj3 = pool.get();
        
        drop(obj1);
        drop(obj2);
        drop(obj3); // This should not be retained due to size limit
        
        assert_eq!(pool.size(), 2);
    }
}