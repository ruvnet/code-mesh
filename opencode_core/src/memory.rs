//! Memory management for OpenCode
//!
//! This module provides persistent memory and context storage
//! for agents and sessions across different environments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Memory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique entry ID
    pub id: Uuid,
    
    /// Entry key
    pub key: String,
    
    /// Entry value
    pub value: serde_json::Value,
    
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// Last access timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    
    /// Expiration timestamp (if any)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Entry metadata
    pub metadata: HashMap<String, serde_json::Value>,
    
    /// Access count
    pub access_count: u64,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total entries
    pub total_entries: usize,
    
    /// Total memory usage in bytes
    pub memory_usage: usize,
    
    /// Active entries
    pub active_entries: usize,
    
    /// Expired entries
    pub expired_entries: usize,
    
    /// Total accesses
    pub total_accesses: u64,
    
    /// Average access count per entry
    pub avg_access_count: f64,
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    
    /// Default expiration time in seconds
    pub default_expiration: Option<u64>,
    
    /// Auto-cleanup interval in seconds
    pub cleanup_interval: u64,
    
    /// Enable persistence
    pub persistent: bool,
    
    /// Storage directory for persistent memory
    pub storage_dir: Option<PathBuf>,
}

/// Memory errors
#[derive(thiserror::Error, Debug)]
pub enum MemoryError {
    #[error("Memory entry not found: {0}")]
    NotFound(String),
    
    #[error("Memory limit exceeded")]
    LimitExceeded,
    
    #[error("Memory entry expired: {0}")]
    Expired(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Memory storage trait
#[async_trait::async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Store a memory entry
    async fn store(&self, entry: &MemoryEntry) -> Result<(), MemoryError>;
    
    /// Retrieve a memory entry
    async fn retrieve(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError>;
    
    /// Delete a memory entry
    async fn delete(&self, key: &str) -> Result<(), MemoryError>;
    
    /// List all memory entries
    async fn list(&self) -> Result<Vec<MemoryEntry>, MemoryError>;
    
    /// Clear all memory entries
    async fn clear(&self) -> Result<(), MemoryError>;
    
    /// Get memory statistics
    async fn stats(&self) -> Result<MemoryStats, MemoryError>;
    
    /// Search memory entries
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>, MemoryError>;
}

/// Main memory manager
pub struct MemoryManager {
    /// Memory storage backend
    storage: Arc<dyn MemoryStorage>,
    
    /// Memory configuration
    config: MemoryConfig,
    
    /// In-memory cache
    cache: Arc<RwLock<HashMap<String, MemoryEntry>>>,
    
    /// Cleanup task handle
    #[cfg(feature = "native-runtime")]
    cleanup_task: Option<tokio::task::JoinHandle<()>>,
}

/// In-memory storage implementation
pub struct InMemoryStorage {
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

/// File-based storage implementation
#[cfg(feature = "native-runtime")]
pub struct FileStorage {
    storage_dir: PathBuf,
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

/// Browser localStorage implementation
#[cfg(feature = "wasm-runtime")]
pub struct BrowserStorage {
    prefix: String,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        MemoryConfig {
            max_entries: 10000,
            default_expiration: Some(24 * 60 * 60), // 24 hours
            cleanup_interval: 300, // 5 minutes
            persistent: true,
            storage_dir: None,
        }
    }
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(key: String, value: serde_json::Value) -> Self {
        let now = chrono::Utc::now();
        MemoryEntry {
            id: Uuid::new_v4(),
            key,
            value,
            created_at: now,
            last_accessed: now,
            expires_at: None,
            metadata: HashMap::new(),
            access_count: 0,
        }
    }
    
    /// Create a memory entry with expiration
    pub fn with_expiration(key: String, value: serde_json::Value, expires_in: u64) -> Self {
        let now = chrono::Utc::now();
        MemoryEntry {
            id: Uuid::new_v4(),
            key,
            value,
            created_at: now,
            last_accessed: now,
            expires_at: Some(now + chrono::Duration::seconds(expires_in as i64)),
            metadata: HashMap::new(),
            access_count: 0,
        }
    }
    
    /// Check if the entry is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |expires| chrono::Utc::now() > expires)
    }
    
    /// Update access information
    pub fn access(&mut self) {
        self.last_accessed = chrono::Utc::now();
        self.access_count += 1;
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        let config = MemoryConfig::default();
        let storage = create_default_storage(&config);
        
        MemoryManager {
            storage,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "native-runtime")]
            cleanup_task: None,
        }
    }
    
    /// Create memory manager with custom configuration
    pub fn with_config(config: MemoryConfig) -> Self {
        let storage = create_default_storage(&config);
        
        MemoryManager {
            storage,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "native-runtime")]
            cleanup_task: None,
        }
    }
    
    /// Create memory manager with custom storage
    pub fn with_storage(storage: Arc<dyn MemoryStorage>, config: MemoryConfig) -> Self {
        MemoryManager {
            storage,
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "native-runtime")]
            cleanup_task: None,
        }
    }
    
    /// Store a value in memory
    pub async fn store(&self, key: &str, value: serde_json::Value) -> Result<(), MemoryError> {
        let entry = if let Some(expiration) = self.config.default_expiration {
            MemoryEntry::with_expiration(key.to_string(), value, expiration)
        } else {
            MemoryEntry::new(key.to_string(), value)
        };
        
        self.store_entry(entry).await
    }
    
    /// Store an entry with custom expiration
    pub async fn store_with_expiration(&self, key: &str, value: serde_json::Value, expires_in: u64) -> Result<(), MemoryError> {
        let entry = MemoryEntry::with_expiration(key.to_string(), value, expires_in);
        self.store_entry(entry).await
    }
    
    /// Store a memory entry
    async fn store_entry(&self, entry: MemoryEntry) -> Result<(), MemoryError> {
        // Check if we're at capacity
        let cache = self.cache.read().await;
        if cache.len() >= self.config.max_entries && !cache.contains_key(&entry.key) {
            drop(cache);
            return Err(MemoryError::LimitExceeded);
        }
        drop(cache);
        
        // Store in backend
        self.storage.store(&entry).await?;
        
        // Update cache
        let mut cache = self.cache.write().await;
        cache.insert(entry.key.clone(), entry);
        
        Ok(())
    }
    
    /// Retrieve a value from memory
    pub async fn retrieve(&self, key: &str) -> Result<Option<serde_json::Value>, MemoryError> {
        // Check cache first
        let mut cache = self.cache.write().await;
        if let Some(entry) = cache.get_mut(key) {
            if entry.is_expired() {
                cache.remove(key);
                drop(cache);
                self.storage.delete(key).await?;
                return Err(MemoryError::Expired(key.to_string()));
            }
            
            entry.access();
            return Ok(Some(entry.value.clone()));
        }
        drop(cache);
        
        // Try storage
        if let Some(mut entry) = self.storage.retrieve(key).await? {
            if entry.is_expired() {
                self.storage.delete(key).await?;
                return Err(MemoryError::Expired(key.to_string()));
            }
            
            entry.access();
            let value = entry.value.clone();
            
            // Update cache
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), entry);
            
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
    
    /// Delete a value from memory
    pub async fn delete(&self, key: &str) -> Result<(), MemoryError> {
        // Remove from cache
        let mut cache = self.cache.write().await;
        cache.remove(key);
        drop(cache);
        
        // Remove from storage
        self.storage.delete(key).await?;
        
        Ok(())
    }
    
    /// List all memory entries
    pub async fn list(&self) -> Result<Vec<String>, MemoryError> {
        let entries = self.storage.list().await?;
        Ok(entries.into_iter().map(|e| e.key).collect())
    }
    
    /// Clear all memory
    pub async fn clear(&self) -> Result<(), MemoryError> {
        let mut cache = self.cache.write().await;
        cache.clear();
        drop(cache);
        
        self.storage.clear().await?;
        
        Ok(())
    }
    
    /// Get memory statistics
    pub async fn stats(&self) -> Result<MemoryStats, MemoryError> {
        self.storage.stats().await
    }
    
    /// Search memory entries
    pub async fn search(&self, query: &str) -> Result<Vec<String>, MemoryError> {
        let entries = self.storage.search(query).await?;
        Ok(entries.into_iter().map(|e| e.key).collect())
    }
    
    /// Cleanup expired entries
    pub async fn cleanup(&self) -> Result<usize, MemoryError> {
        let entries = self.storage.list().await?;
        let mut cleaned = 0;
        
        for entry in entries {
            if entry.is_expired() {
                self.delete(&entry.key).await?;
                cleaned += 1;
            }
        }
        
        Ok(cleaned)
    }
    
    /// Start automatic cleanup
    #[cfg(feature = "native-runtime")]
    pub fn start_cleanup(&mut self) {
        if self.cleanup_task.is_some() {
            return;
        }
        
        let storage = self.storage.clone();
        let interval = self.config.cleanup_interval;
        
        let task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(interval));
            
            loop {
                interval.tick().await;
                
                if let Ok(entries) = storage.list().await {
                    for entry in entries {
                        if entry.is_expired() {
                            let _ = storage.delete(&entry.key).await;
                        }
                    }
                }
            }
        });
        
        self.cleanup_task = Some(task);
    }
    
    /// Stop automatic cleanup
    #[cfg(feature = "native-runtime")]
    pub fn stop_cleanup(&mut self) {
        if let Some(task) = self.cleanup_task.take() {
            task.abort();
        }
    }
}

impl Drop for MemoryManager {
    fn drop(&mut self) {
        #[cfg(feature = "native-runtime")]
        {
            self.stop_cleanup();
        }
    }
}

// Storage implementations
impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl MemoryStorage for InMemoryStorage {
    async fn store(&self, entry: &MemoryEntry) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.key.clone(), entry.clone());
        Ok(())
    }
    
    async fn retrieve(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().await;
        Ok(entries.get(key).cloned())
    }
    
    async fn delete(&self, key: &str) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().await;
        entries.remove(key);
        Ok(())
    }
    
    async fn list(&self) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().await;
        Ok(entries.values().cloned().collect())
    }
    
    async fn clear(&self) -> Result<(), MemoryError> {
        let mut entries = self.entries.write().await;
        entries.clear();
        Ok(())
    }
    
    async fn stats(&self) -> Result<MemoryStats, MemoryError> {
        let entries = self.entries.read().await;
        let total_entries = entries.len();
        let active_entries = entries.values().filter(|e| !e.is_expired()).count();
        let expired_entries = total_entries - active_entries;
        let total_accesses = entries.values().map(|e| e.access_count).sum::<u64>();
        let avg_access_count = if total_entries > 0 {
            total_accesses as f64 / total_entries as f64
        } else {
            0.0
        };
        
        // Estimate memory usage
        let memory_usage = entries.values()
            .map(|e| serde_json::to_string(e).unwrap_or_default().len())
            .sum();
        
        Ok(MemoryStats {
            total_entries,
            memory_usage,
            active_entries,
            expired_entries,
            total_accesses,
            avg_access_count,
        })
    }
    
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = self.entries.read().await;
        let results = entries.values()
            .filter(|e| {
                e.key.contains(query) || 
                e.value.to_string().contains(query) ||
                e.metadata.values().any(|v| v.to_string().contains(query))
            })
            .cloned()
            .collect();
        
        Ok(results)
    }
}

#[cfg(feature = "native-runtime")]
impl FileStorage {
    pub fn new(storage_dir: PathBuf) -> Self {
        FileStorage {
            storage_dir,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn load_from_disk(&self) -> Result<(), MemoryError> {
        if !self.storage_dir.exists() {
            tokio::fs::create_dir_all(&self.storage_dir).await?;
            return Ok(());
        }
        
        let mut entries = self.entries.write().await;
        let mut dir = tokio::fs::read_dir(&self.storage_dir).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(memory_entry) = serde_json::from_str::<MemoryEntry>(&content) {
                        entries.insert(memory_entry.key.clone(), memory_entry);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    async fn save_to_disk(&self, entry: &MemoryEntry) -> Result<(), MemoryError> {
        tokio::fs::create_dir_all(&self.storage_dir).await?;
        
        let file_path = self.storage_dir.join(format!("{}.json", entry.key));
        let content = serde_json::to_string_pretty(entry)?;
        tokio::fs::write(file_path, content).await?;
        
        Ok(())
    }
    
    async fn remove_from_disk(&self, key: &str) -> Result<(), MemoryError> {
        let file_path = self.storage_dir.join(format!("{}.json", key));
        if file_path.exists() {
            tokio::fs::remove_file(file_path).await?;
        }
        Ok(())
    }
}

#[cfg(feature = "native-runtime")]
#[async_trait::async_trait]
impl MemoryStorage for FileStorage {
    async fn store(&self, entry: &MemoryEntry) -> Result<(), MemoryError> {
        // Store in memory cache
        let mut entries = self.entries.write().await;
        entries.insert(entry.key.clone(), entry.clone());
        drop(entries);
        
        // Save to disk
        self.save_to_disk(entry).await?;
        
        Ok(())
    }
    
    async fn retrieve(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        // Try memory cache first
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            return Ok(Some(entry.clone()));
        }
        drop(entries);
        
        // Try to load from disk
        self.load_from_disk().await?;
        
        let entries = self.entries.read().await;
        Ok(entries.get(key).cloned())
    }
    
    async fn delete(&self, key: &str) -> Result<(), MemoryError> {
        // Remove from memory cache
        let mut entries = self.entries.write().await;
        entries.remove(key);
        drop(entries);
        
        // Remove from disk
        self.remove_from_disk(key).await?;
        
        Ok(())
    }
    
    async fn list(&self) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.load_from_disk().await?;
        let entries = self.entries.read().await;
        Ok(entries.values().cloned().collect())
    }
    
    async fn clear(&self) -> Result<(), MemoryError> {
        // Clear memory cache
        let mut entries = self.entries.write().await;
        entries.clear();
        drop(entries);
        
        // Clear disk storage
        if self.storage_dir.exists() {
            tokio::fs::remove_dir_all(&self.storage_dir).await?;
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<MemoryStats, MemoryError> {
        self.load_from_disk().await?;
        let entries = self.entries.read().await;
        let total_entries = entries.len();
        let active_entries = entries.values().filter(|e| !e.is_expired()).count();
        let expired_entries = total_entries - active_entries;
        let total_accesses = entries.values().map(|e| e.access_count).sum::<u64>();
        let avg_access_count = if total_entries > 0 {
            total_accesses as f64 / total_entries as f64
        } else {
            0.0
        };
        
        // Estimate memory usage
        let memory_usage = entries.values()
            .map(|e| serde_json::to_string(e).unwrap_or_default().len())
            .sum();
        
        Ok(MemoryStats {
            total_entries,
            memory_usage,
            active_entries,
            expired_entries,
            total_accesses,
            avg_access_count,
        })
    }
    
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>, MemoryError> {
        self.load_from_disk().await?;
        let entries = self.entries.read().await;
        let results = entries.values()
            .filter(|e| {
                e.key.contains(query) || 
                e.value.to_string().contains(query) ||
                e.metadata.values().any(|v| v.to_string().contains(query))
            })
            .cloned()
            .collect();
        
        Ok(results)
    }
}

// Create default storage based on configuration
fn create_default_storage(config: &MemoryConfig) -> Arc<dyn MemoryStorage> {
    #[cfg(feature = "native-runtime")]
    {
        if config.persistent {
            if let Some(storage_dir) = &config.storage_dir {
                return Arc::new(FileStorage::new(storage_dir.clone()));
            } else if let Some(config_dir) = dirs::config_dir() {
                let storage_dir = config_dir.join("opencode").join("memory");
                return Arc::new(FileStorage::new(storage_dir));
            }
        }
    }
    
    #[cfg(feature = "wasm-runtime")]
    {
        if config.persistent {
            return Arc::new(BrowserStorage::new("opencode_memory".to_string()));
        }
    }
    
    Arc::new(InMemoryStorage::new())
}

#[cfg(feature = "wasm-runtime")]
impl BrowserStorage {
    pub fn new(prefix: String) -> Self {
        BrowserStorage { prefix }
    }
    
    fn get_key(&self, key: &str) -> String {
        format!("{}_{}", self.prefix, key)
    }
}

#[cfg(feature = "wasm-runtime")]
#[async_trait::async_trait]
impl MemoryStorage for BrowserStorage {
    async fn store(&self, entry: &MemoryEntry) -> Result<(), MemoryError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| MemoryError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| MemoryError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| MemoryError::Storage("localStorage not available".to_string()))?;
        
        let key = self.get_key(&entry.key);
        let value = serde_json::to_string(entry)?;
        
        storage.set_item(&key, &value)
            .map_err(|_| MemoryError::Storage("Failed to store entry".to_string()))?;
        
        Ok(())
    }
    
    async fn retrieve(&self, key: &str) -> Result<Option<MemoryEntry>, MemoryError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| MemoryError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| MemoryError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| MemoryError::Storage("localStorage not available".to_string()))?;
        
        let storage_key = self.get_key(key);
        
        match storage.get_item(&storage_key) {
            Ok(Some(value)) => {
                let entry: MemoryEntry = serde_json::from_str(&value)?;
                Ok(Some(entry))
            }
            Ok(None) => Ok(None),
            Err(_) => Err(MemoryError::Storage("Failed to retrieve entry".to_string())),
        }
    }
    
    async fn delete(&self, key: &str) -> Result<(), MemoryError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| MemoryError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| MemoryError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| MemoryError::Storage("localStorage not available".to_string()))?;
        
        let storage_key = self.get_key(key);
        storage.remove_item(&storage_key)
            .map_err(|_| MemoryError::Storage("Failed to delete entry".to_string()))?;
        
        Ok(())
    }
    
    async fn list(&self) -> Result<Vec<MemoryEntry>, MemoryError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| MemoryError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| MemoryError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| MemoryError::Storage("localStorage not available".to_string()))?;
        
        let mut entries = Vec::new();
        let length = storage.length()
            .map_err(|_| MemoryError::Storage("Cannot get storage length".to_string()))?;
        
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&format!("{}_", self.prefix)) {
                    if let Ok(Some(value)) = storage.get_item(&key) {
                        if let Ok(entry) = serde_json::from_str::<MemoryEntry>(&value) {
                            entries.push(entry);
                        }
                    }
                }
            }
        }
        
        Ok(entries)
    }
    
    async fn clear(&self) -> Result<(), MemoryError> {
        use wasm_bindgen::prelude::*;
        use web_sys::window;
        
        let window = window().ok_or_else(|| MemoryError::Storage("No window object".to_string()))?;
        let storage = window.local_storage()
            .map_err(|_| MemoryError::Storage("Cannot access localStorage".to_string()))?
            .ok_or_else(|| MemoryError::Storage("localStorage not available".to_string()))?;
        
        let length = storage.length()
            .map_err(|_| MemoryError::Storage("Cannot get storage length".to_string()))?;
        
        let mut keys_to_remove = Vec::new();
        for i in 0..length {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&format!("{}_", self.prefix)) {
                    keys_to_remove.push(key);
                }
            }
        }
        
        for key in keys_to_remove {
            storage.remove_item(&key)
                .map_err(|_| MemoryError::Storage("Failed to remove entry".to_string()))?;
        }
        
        Ok(())
    }
    
    async fn stats(&self) -> Result<MemoryStats, MemoryError> {
        let entries = self.list().await?;
        let total_entries = entries.len();
        let active_entries = entries.iter().filter(|e| !e.is_expired()).count();
        let expired_entries = total_entries - active_entries;
        let total_accesses = entries.iter().map(|e| e.access_count).sum::<u64>();
        let avg_access_count = if total_entries > 0 {
            total_accesses as f64 / total_entries as f64
        } else {
            0.0
        };
        
        // Estimate memory usage
        let memory_usage = entries.iter()
            .map(|e| serde_json::to_string(e).unwrap_or_default().len())
            .sum();
        
        Ok(MemoryStats {
            total_entries,
            memory_usage,
            active_entries,
            expired_entries,
            total_accesses,
            avg_access_count,
        })
    }
    
    async fn search(&self, query: &str) -> Result<Vec<MemoryEntry>, MemoryError> {
        let entries = self.list().await?;
        let results = entries.into_iter()
            .filter(|e| {
                e.key.contains(query) || 
                e.value.to_string().contains(query) ||
                e.metadata.values().any(|v| v.to_string().contains(query))
            })
            .collect();
        
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_entry_creation() {
        let entry = MemoryEntry::new("test_key".to_string(), serde_json::json!("test_value"));
        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.value, serde_json::json!("test_value"));
        assert_eq!(entry.access_count, 0);
        assert!(!entry.is_expired());
    }
    
    #[test]
    fn test_memory_entry_with_expiration() {
        let entry = MemoryEntry::with_expiration("test_key".to_string(), serde_json::json!("test_value"), 1);
        assert_eq!(entry.key, "test_key");
        assert!(entry.expires_at.is_some());
        
        // Should not be expired immediately
        assert!(!entry.is_expired());
        
        // Create an entry that's already expired
        let expired_entry = MemoryEntry::with_expiration("expired".to_string(), serde_json::json!("value"), 0);
        // Give it a moment to expire
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(expired_entry.is_expired());
    }
    
    #[test]
    fn test_memory_entry_access() {
        let mut entry = MemoryEntry::new("test_key".to_string(), serde_json::json!("test_value"));
        assert_eq!(entry.access_count, 0);
        
        entry.access();
        assert_eq!(entry.access_count, 1);
        
        entry.access();
        assert_eq!(entry.access_count, 2);
    }
    
    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();
        let entry = MemoryEntry::new("test_key".to_string(), serde_json::json!("test_value"));
        
        // Store entry
        storage.store(&entry).await.unwrap();
        
        // Retrieve entry
        let retrieved = storage.retrieve("test_key").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, serde_json::json!("test_value"));
        
        // List entries
        let entries = storage.list().await.unwrap();
        assert_eq!(entries.len(), 1);
        
        // Delete entry
        storage.delete("test_key").await.unwrap();
        let retrieved = storage.retrieve("test_key").await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_memory_manager() {
        let mut manager = MemoryManager::new();
        
        // Store value
        manager.store("test_key", serde_json::json!("test_value")).await.unwrap();
        
        // Retrieve value
        let retrieved = manager.retrieve("test_key").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), serde_json::json!("test_value"));
        
        // List keys
        let keys = manager.list().await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "test_key");
        
        // Search
        let results = manager.search("test").await.unwrap();
        assert_eq!(results.len(), 1);
        
        // Delete value
        manager.delete("test_key").await.unwrap();
        let retrieved = manager.retrieve("test_key").await.unwrap();
        assert!(retrieved.is_none());
    }
    
    #[tokio::test]
    async fn test_memory_stats() {
        let storage = InMemoryStorage::new();
        let entry1 = MemoryEntry::new("key1".to_string(), serde_json::json!("value1"));
        let entry2 = MemoryEntry::new("key2".to_string(), serde_json::json!("value2"));
        
        storage.store(&entry1).await.unwrap();
        storage.store(&entry2).await.unwrap();
        
        let stats = storage.stats().await.unwrap();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.active_entries, 2);
        assert_eq!(stats.expired_entries, 0);
        assert!(stats.memory_usage > 0);
    }
}