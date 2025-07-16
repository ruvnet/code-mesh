//! High-performance caching system for code-mesh

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime};
use lru::LruCache;
use std::num::NonZeroUsize;

/// Multi-level cache system with intelligent eviction
pub struct MultiLevelCache<K, V> 
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    l1_cache: Arc<RwLock<LruCache<K, CacheEntry<V>>>>, // Hot cache (fast access)
    l2_cache: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,  // Warm cache (larger capacity)
    config: CacheConfig,
    stats: Arc<RwLock<CacheStats>>,
}

impl<K, V> MultiLevelCache<K, V>
where
    K: Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(config: CacheConfig) -> Self {
        let l1_size = NonZeroUsize::new(config.l1_size).unwrap_or(NonZeroUsize::new(100).unwrap());
        
        Self {
            l1_cache: Arc::new(RwLock::new(LruCache::new(l1_size))),
            l2_cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::new())),
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        let start_time = Instant::now();
        
        // Try L1 cache first (hot cache)
        {
            let mut l1 = self.l1_cache.write().unwrap();
            if let Some(entry) = l1.get(key) {
                if !entry.is_expired() {
                    self.record_hit(CacheLevel::L1, start_time.elapsed());
                    return Some(entry.value.clone());
                } else {
                    // Remove expired entry
                    l1.pop(key);
                }
            }
        }

        // Try L2 cache (warm cache)
        {
            let mut l2 = self.l2_cache.write().unwrap();
            if let Some(entry) = l2.get(key) {
                if !entry.is_expired() {
                    // Promote to L1 cache
                    let mut l1 = self.l1_cache.write().unwrap();
                    l1.put(key.clone(), entry.clone());
                    
                    self.record_hit(CacheLevel::L2, start_time.elapsed());
                    return Some(entry.value.clone());
                } else {
                    // Remove expired entry
                    l2.remove(key);
                }
            }
        }

        // Cache miss
        self.record_miss(start_time.elapsed());
        None
    }

    /// Put a value into the cache
    pub fn put(&self, key: K, value: V) {
        self.put_with_ttl(key, value, self.config.default_ttl);
    }

    /// Put a value with custom TTL
    pub fn put_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let entry = CacheEntry {
            value,
            inserted_at: SystemTime::now(),
            ttl,
            access_count: 1,
            last_accessed: SystemTime::now(),
        };

        // Always insert into L1 cache for hot access
        {
            let mut l1 = self.l1_cache.write().unwrap();
            l1.put(key, entry);
        }

        self.record_insert();
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        // Remove from both levels
        let l1_result = {
            let mut l1 = self.l1_cache.write().unwrap();
            l1.pop(key)
        };

        let l2_result = {
            let mut l2 = self.l2_cache.write().unwrap();
            l2.remove(key)
        };

        l1_result.or(l2_result).map(|entry| entry.value)
    }

    /// Clear all entries
    pub fn clear(&self) {
        {
            let mut l1 = self.l1_cache.write().unwrap();
            l1.clear();
        }
        {
            let mut l2 = self.l2_cache.write().unwrap();
            l2.clear();
        }
        
        {
            let mut stats = self.stats.write().unwrap();
            *stats = CacheStats::new();
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Cleanup expired entries
    pub fn cleanup_expired(&self) {
        let now = SystemTime::now();
        
        // Cleanup L1
        {
            let mut l1 = self.l1_cache.write().unwrap();
            let keys_to_remove: Vec<K> = l1.iter()
                .filter(|(_, entry)| entry.is_expired_at(now))
                .map(|(key, _)| key.clone())
                .collect();
            
            for key in keys_to_remove {
                l1.pop(&key);
            }
        }

        // Cleanup L2
        {
            let mut l2 = self.l2_cache.write().unwrap();
            l2.retain(|_, entry| !entry.is_expired_at(now));
        }
    }

    /// Get cache size information
    pub fn size_info(&self) -> CacheSizeInfo {
        let l1_size = self.l1_cache.read().unwrap().len();
        let l2_size = self.l2_cache.read().unwrap().len();
        
        CacheSizeInfo {
            l1_entries: l1_size,
            l2_entries: l2_size,
            total_entries: l1_size + l2_size,
            l1_capacity: self.config.l1_size,
            l2_capacity: self.config.l2_size,
        }
    }

    fn record_hit(&self, level: CacheLevel, duration: Duration) {
        let mut stats = self.stats.write().unwrap();
        stats.hits += 1;
        stats.total_access_time += duration;
        
        match level {
            CacheLevel::L1 => stats.l1_hits += 1,
            CacheLevel::L2 => stats.l2_hits += 1,
        }
    }

    fn record_miss(&self, duration: Duration) {
        let mut stats = self.stats.write().unwrap();
        stats.misses += 1;
        stats.total_access_time += duration;
    }

    fn record_insert(&self) {
        let mut stats = self.stats.write().unwrap();
        stats.inserts += 1;
    }
}

/// Cache entry with metadata
#[derive(Clone, Debug)]
struct CacheEntry<V> {
    value: V,
    inserted_at: SystemTime,
    ttl: Duration,
    access_count: u64,
    last_accessed: SystemTime,
}

impl<V> CacheEntry<V> {
    fn is_expired(&self) -> bool {
        self.is_expired_at(SystemTime::now())
    }

    fn is_expired_at(&self, now: SystemTime) -> bool {
        if let Ok(elapsed) = now.duration_since(self.inserted_at) {
            elapsed > self.ttl
        } else {
            false
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub l1_size: usize,        // Hot cache size
    pub l2_size: usize,        // Warm cache size  
    pub default_ttl: Duration, // Default time-to-live
    pub cleanup_interval: Duration, // How often to cleanup expired entries
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 100,
            l2_size: 1000,
            default_ttl: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub l1_hits: u64,
    pub l2_hits: u64,
    pub inserts: u64,
    pub total_access_time: Duration,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: 0,
            misses: 0,
            l1_hits: 0,
            l2_hits: 0,
            inserts: 0,
            total_access_time: Duration::default(),
        }
    }

    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    pub fn l1_hit_rate(&self) -> f64 {
        if self.hits == 0 {
            0.0
        } else {
            self.l1_hits as f64 / self.hits as f64
        }
    }

    pub fn average_access_time(&self) -> Duration {
        let total_accesses = self.hits + self.misses;
        if total_accesses == 0 {
            Duration::default()
        } else {
            self.total_access_time / total_accesses as u32
        }
    }
}

/// Cache size information
#[derive(Debug, Clone)]
pub struct CacheSizeInfo {
    pub l1_entries: usize,
    pub l2_entries: usize,
    pub total_entries: usize,
    pub l1_capacity: usize,
    pub l2_capacity: usize,
}

/// Cache level enum
enum CacheLevel {
    L1,
    L2,
}

/// Specialized cache implementations
pub type ResponseCache = MultiLevelCache<String, String>;
pub type FileCache = MultiLevelCache<std::path::PathBuf, Vec<u8>>;
pub type TokenCache = MultiLevelCache<String, Vec<String>>;

/// Factory for creating optimized caches
pub struct CacheFactory;

impl CacheFactory {
    /// Create a cache optimized for API responses
    pub fn create_response_cache() -> ResponseCache {
        let config = CacheConfig {
            l1_size: 50,
            l2_size: 500,
            default_ttl: Duration::from_secs(300), // 5 minutes
            cleanup_interval: Duration::from_secs(60),
        };
        MultiLevelCache::new(config)
    }

    /// Create a cache optimized for file content
    pub fn create_file_cache() -> FileCache {
        let config = CacheConfig {
            l1_size: 20,
            l2_size: 200,
            default_ttl: Duration::from_secs(600), // 10 minutes
            cleanup_interval: Duration::from_secs(120),
        };
        MultiLevelCache::new(config)
    }

    /// Create a cache optimized for tokens
    pub fn create_token_cache() -> TokenCache {
        let config = CacheConfig {
            l1_size: 100,
            l2_size: 1000,
            default_ttl: Duration::from_secs(1800), // 30 minutes
            cleanup_interval: Duration::from_secs(300),
        };
        MultiLevelCache::new(config)
    }
}

/// Cache manager for coordinating multiple caches
pub struct CacheManager {
    response_cache: ResponseCache,
    file_cache: FileCache,
    token_cache: TokenCache,
}

impl CacheManager {
    pub fn new() -> Self {
        Self {
            response_cache: CacheFactory::create_response_cache(),
            file_cache: CacheFactory::create_file_cache(),
            token_cache: CacheFactory::create_token_cache(),
        }
    }

    pub fn response_cache(&self) -> &ResponseCache {
        &self.response_cache
    }

    pub fn file_cache(&self) -> &FileCache {
        &self.file_cache
    }

    pub fn token_cache(&self) -> &TokenCache {
        &self.token_cache
    }

    /// Get overall cache statistics
    pub fn overall_stats(&self) -> OverallCacheStats {
        OverallCacheStats {
            response_cache: self.response_cache.stats(),
            file_cache: self.file_cache.stats(),
            token_cache: self.token_cache.stats(),
        }
    }

    /// Cleanup all caches
    pub fn cleanup_all(&self) {
        self.response_cache.cleanup_expired();
        self.file_cache.cleanup_expired();
        self.token_cache.cleanup_expired();
    }
}

/// Overall cache statistics
#[derive(Debug, Clone)]
pub struct OverallCacheStats {
    pub response_cache: CacheStats,
    pub file_cache: CacheStats,
    pub token_cache: CacheStats,
}

impl OverallCacheStats {
    pub fn total_hit_rate(&self) -> f64 {
        let total_hits = self.response_cache.hits + self.file_cache.hits + self.token_cache.hits;
        let total_accesses = total_hits + 
            self.response_cache.misses + self.file_cache.misses + self.token_cache.misses;
        
        if total_accesses == 0 {
            0.0
        } else {
            total_hits as f64 / total_accesses as f64
        }
    }
}