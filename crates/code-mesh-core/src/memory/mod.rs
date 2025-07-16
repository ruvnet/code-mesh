//! Memory management for Code Mesh

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Memory trait for storing and retrieving data
#[async_trait]
pub trait Memory: Send + Sync {
    /// Store a value with optional TTL
    async fn store(&self, key: &str, value: MemoryValue, ttl: Option<Duration>) -> crate::Result<()>;
    
    /// Retrieve a value
    async fn retrieve(&self, key: &str) -> crate::Result<Option<MemoryValue>>;
    
    /// Delete a value
    async fn delete(&self, key: &str) -> crate::Result<()>;
    
    /// Search for values matching a pattern
    async fn search(&self, pattern: &str, limit: usize) -> crate::Result<Vec<(String, MemoryValue)>>;
    
    /// List all keys in a namespace
    async fn list_namespace(&self, namespace: &str) -> crate::Result<Vec<String>>;
}

/// Value stored in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryValue {
    pub data: serde_json::Value,
    pub metadata: MemoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub tags: Vec<String>,
}