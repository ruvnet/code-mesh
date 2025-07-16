//! Storage abstractions for Code Mesh

use async_trait::async_trait;
use std::path::PathBuf;

mod file;

/// Storage trait for persistent data
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a value with a key
    async fn set(&self, key: &str, value: &[u8]) -> Result<(), StorageError>;
    
    /// Retrieve a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError>;
    
    /// Delete a value by key
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    
    /// List all keys with optional prefix
    async fn list(&self, prefix: Option<&str>) -> Result<Vec<String>, StorageError>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;
}

/// Storage errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Storage error: {0}")]
    Other(String),
}

/// File-based storage implementation
pub struct FileStorage {
    base_path: PathBuf,
}

impl FileStorage {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    
    pub fn default() -> Result<Self, StorageError> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| StorageError::Other("Could not find data directory".to_string()))?;
        let base_path = data_dir.join("code-mesh").join("storage");
        Ok(Self::new(base_path))
    }
    
    fn key_to_path(&self, key: &str) -> PathBuf {
        // Sanitize key to prevent path traversal
        let safe_key = key.replace(['/', '\\'], "_").replace("..", "_");
        self.base_path.join(safe_key)
    }
}