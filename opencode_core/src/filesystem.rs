//! File system abstraction for OpenCode
//!
//! This module provides a unified interface for file system operations
//! that works across native and WebAssembly environments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// File metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File path
    pub path: PathBuf,
    
    /// File size in bytes
    pub size: u64,
    
    /// File type
    pub file_type: FileType,
    
    /// Creation timestamp
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Last modified timestamp
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
    
    /// File permissions (Unix-style)
    pub permissions: Option<u32>,
    
    /// Whether the file is hidden
    pub hidden: bool,
    
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// File type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    /// Regular file
    File,
    
    /// Directory
    Directory,
    
    /// Symbolic link
    Symlink,
    
    /// Unknown type
    Unknown,
}

/// File system entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// File metadata
    pub metadata: FileMetadata,
    
    /// File content (if it's a file)
    pub content: Option<String>,
    
    /// Directory entries (if it's a directory)
    pub children: Option<Vec<FileEntry>>,
}

/// File system events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSystemEvent {
    /// File was created
    Created {
        path: PathBuf,
        metadata: FileMetadata,
    },
    
    /// File was modified
    Modified {
        path: PathBuf,
        metadata: FileMetadata,
    },
    
    /// File was deleted
    Deleted {
        path: PathBuf,
    },
    
    /// File was renamed
    Renamed {
        from: PathBuf,
        to: PathBuf,
    },
}

/// File system errors
#[derive(thiserror::Error, Debug)]
pub enum FileSystemError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),
    
    #[error("File already exists: {0}")]
    AlreadyExists(PathBuf),
    
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Not supported in current environment: {0}")]
    NotSupported(String),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// File system trait
#[async_trait::async_trait]
pub trait FileSystem: Send + Sync {
    /// Read file content
    async fn read_file(&self, path: &Path) -> Result<String, FileSystemError>;
    
    /// Write file content
    async fn write_file(&self, path: &Path, content: &str) -> Result<(), FileSystemError>;
    
    /// Create directory
    async fn create_dir(&self, path: &Path) -> Result<(), FileSystemError>;
    
    /// Remove file or directory
    async fn remove(&self, path: &Path) -> Result<(), FileSystemError>;
    
    /// List directory contents
    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>, FileSystemError>;
    
    /// Get file metadata
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError>;
    
    /// Check if path exists
    async fn exists(&self, path: &Path) -> Result<bool, FileSystemError>;
    
    /// Copy file or directory
    async fn copy(&self, from: &Path, to: &Path) -> Result<(), FileSystemError>;
    
    /// Move/rename file or directory
    async fn move_file(&self, from: &Path, to: &Path) -> Result<(), FileSystemError>;
    
    /// Watch for file system changes
    async fn watch(&self, path: &Path) -> Result<Box<dyn futures::Stream<Item = FileSystemEvent> + Send + Unpin>, FileSystemError>;
    
    /// Get current working directory
    async fn current_dir(&self) -> Result<PathBuf, FileSystemError>;
    
    /// Set current working directory
    async fn set_current_dir(&self, path: &Path) -> Result<(), FileSystemError>;
}

/// Native file system implementation
#[cfg(feature = "native-runtime")]
pub struct NativeFileSystem {
    current_dir: Arc<RwLock<PathBuf>>,
}

/// Virtual file system (for WASM)
pub struct VirtualFileSystem {
    files: Arc<RwLock<HashMap<PathBuf, FileEntry>>>,
    current_dir: Arc<RwLock<PathBuf>>,
}

/// Browser file system (limited functionality)
#[cfg(feature = "wasm-runtime")]
pub struct BrowserFileSystem {
    current_dir: Arc<RwLock<PathBuf>>,
}

/// File system manager
pub struct FileSystemManager {
    fs: Arc<dyn FileSystem>,
    watchers: Arc<RwLock<HashMap<PathBuf, Box<dyn futures::Stream<Item = FileSystemEvent> + Send + Unpin>>>>,
}

impl FileType {
    /// Convert from std::fs::FileType
    #[cfg(feature = "native-runtime")]
    pub fn from_std_file_type(file_type: std::fs::FileType) -> Self {
        if file_type.is_file() {
            FileType::File
        } else if file_type.is_dir() {
            FileType::Directory
        } else if file_type.is_symlink() {
            FileType::Symlink
        } else {
            FileType::Unknown
        }
    }
}

impl FileMetadata {
    /// Create new file metadata
    pub fn new(path: PathBuf, file_type: FileType) -> Self {
        FileMetadata {
            path,
            size: 0,
            file_type,
            created: None,
            modified: None,
            permissions: None,
            hidden: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::File
    }
    
    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }
    
    /// Check if this is a symlink
    pub fn is_symlink(&self) -> bool {
        self.file_type == FileType::Symlink
    }
}

// Native file system implementation
#[cfg(feature = "native-runtime")]
impl NativeFileSystem {
    /// Create a new native file system
    pub fn new() -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));
        
        NativeFileSystem {
            current_dir: Arc::new(RwLock::new(current_dir)),
        }
    }
    
    /// Convert std::fs::Metadata to FileMetadata
    fn convert_metadata(&self, path: PathBuf, metadata: std::fs::Metadata) -> FileMetadata {
        use std::os::unix::fs::MetadataExt;
        
        let file_type = FileType::from_std_file_type(metadata.file_type());
        
        FileMetadata {
            path: path.clone(),
            size: metadata.len(),
            file_type,
            created: metadata.created().ok().map(|t| {
                chrono::DateTime::from(t)
            }),
            modified: metadata.modified().ok().map(|t| {
                chrono::DateTime::from(t)
            }),
            permissions: Some(metadata.mode()),
            hidden: path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with('.'))
                .unwrap_or(false),
            metadata: HashMap::new(),
        }
    }
}

#[cfg(feature = "native-runtime")]
#[async_trait::async_trait]
impl FileSystem for NativeFileSystem {
    async fn read_file(&self, path: &Path) -> Result<String, FileSystemError> {
        let content = tokio::fs::read_to_string(path).await?;
        Ok(content)
    }
    
    async fn write_file(&self, path: &Path, content: &str) -> Result<(), FileSystemError> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(path, content).await?;
        Ok(())
    }
    
    async fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        tokio::fs::create_dir_all(path).await?;
        Ok(())
    }
    
    async fn remove(&self, path: &Path) -> Result<(), FileSystemError> {
        let metadata = tokio::fs::metadata(path).await?;
        if metadata.is_dir() {
            tokio::fs::remove_dir_all(path).await?;
        } else {
            tokio::fs::remove_file(path).await?;
        }
        Ok(())
    }
    
    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>, FileSystemError> {
        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;
        
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            let metadata = entry.metadata().await?;
            let file_metadata = self.convert_metadata(path, metadata);
            entries.push(file_metadata);
        }
        
        Ok(entries)
    }
    
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError> {
        let metadata = tokio::fs::metadata(path).await?;
        Ok(self.convert_metadata(path.to_path_buf(), metadata))
    }
    
    async fn exists(&self, path: &Path) -> Result<bool, FileSystemError> {
        Ok(tokio::fs::try_exists(path).await?)
    }
    
    async fn copy(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        let metadata = tokio::fs::metadata(from).await?;
        if metadata.is_dir() {
            self.copy_dir(from, to).await?;
        } else {
            if let Some(parent) = to.parent() {
                tokio::fs::create_dir_all(parent).await?;
            }
            tokio::fs::copy(from, to).await?;
        }
        Ok(())
    }
    
    async fn move_file(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        if let Some(parent) = to.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(from, to).await?;
        Ok(())
    }
    
    async fn watch(&self, path: &Path) -> Result<Box<dyn futures::Stream<Item = FileSystemEvent> + Send + Unpin>, FileSystemError> {
        #[cfg(feature = "file-watching")]
        {
            use notify::{Watcher, RecursiveMode, Event};
            use tokio::sync::mpsc;
            
            let (tx, rx) = mpsc::unbounded_channel();
            let mut watcher = notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    // Convert notify events to FileSystemEvent
                    for path in event.paths {
                        let fs_event = match event.kind {
                            notify::EventKind::Create(_) => {
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    let file_metadata = FileMetadata {
                                        path: path.clone(),
                                        size: metadata.len(),
                                        file_type: FileType::from_std_file_type(metadata.file_type()),
                                        created: metadata.created().ok().map(chrono::DateTime::from),
                                        modified: metadata.modified().ok().map(chrono::DateTime::from),
                                        permissions: None,
                                        hidden: false,
                                        metadata: HashMap::new(),
                                    };
                                    FileSystemEvent::Created { path, metadata: file_metadata }
                                } else {
                                    continue;
                                }
                            }
                            notify::EventKind::Modify(_) => {
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    let file_metadata = FileMetadata {
                                        path: path.clone(),
                                        size: metadata.len(),
                                        file_type: FileType::from_std_file_type(metadata.file_type()),
                                        created: metadata.created().ok().map(chrono::DateTime::from),
                                        modified: metadata.modified().ok().map(chrono::DateTime::from),
                                        permissions: None,
                                        hidden: false,
                                        metadata: HashMap::new(),
                                    };
                                    FileSystemEvent::Modified { path, metadata: file_metadata }
                                } else {
                                    continue;
                                }
                            }
                            notify::EventKind::Remove(_) => {
                                FileSystemEvent::Deleted { path }
                            }
                            _ => continue,
                        };
                        
                        let _ = tx.send(fs_event);
                    }
                }
            }).map_err(|e| FileSystemError::Generic(e.to_string()))?;
            
            watcher.watch(path, RecursiveMode::Recursive)
                .map_err(|e| FileSystemError::Generic(e.to_string()))?;
            
            let stream = tokio_stream::wrappers::UnboundedReceiverStream::new(rx);
            Ok(Box::new(Box::pin(stream)))
        }
        
        #[cfg(not(feature = "file-watching"))]
        {
            Err(FileSystemError::NotSupported("File watching not available".to_string()))
        }
    }
    
    async fn current_dir(&self) -> Result<PathBuf, FileSystemError> {
        let current_dir = self.current_dir.read().await;
        Ok(current_dir.clone())
    }
    
    async fn set_current_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        if !self.exists(path).await? {
            return Err(FileSystemError::NotFound(path.to_path_buf()));
        }
        
        let metadata = self.metadata(path).await?;
        if !metadata.is_dir() {
            return Err(FileSystemError::Generic("Path is not a directory".to_string()));
        }
        
        let mut current_dir = self.current_dir.write().await;
        *current_dir = path.to_path_buf();
        Ok(())
    }
}

#[cfg(feature = "native-runtime")]
impl NativeFileSystem {
    async fn copy_dir(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        self.create_dir(to).await?;
        
        let mut dir = tokio::fs::read_dir(from).await?;
        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            let file_name = entry.file_name();
            let target_path = to.join(file_name);
            
            if entry_path.is_dir() {
                self.copy_dir(&entry_path, &target_path).await?;
            } else {
                tokio::fs::copy(&entry_path, &target_path).await?;
            }
        }
        
        Ok(())
    }
}

// Virtual file system implementation
impl VirtualFileSystem {
    /// Create a new virtual file system
    pub fn new() -> Self {
        VirtualFileSystem {
            files: Arc::new(RwLock::new(HashMap::new())),
            current_dir: Arc::new(RwLock::new(PathBuf::from("/"))),
        }
    }
    
    /// Normalize path
    fn normalize_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            PathBuf::from("/").join(path)
        }
    }
}

#[async_trait::async_trait]
impl FileSystem for VirtualFileSystem {
    async fn read_file(&self, path: &Path) -> Result<String, FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let files = self.files.read().await;
        
        if let Some(entry) = files.get(&normalized_path) {
            if let Some(content) = &entry.content {
                Ok(content.clone())
            } else {
                Err(FileSystemError::Generic("Path is not a file".to_string()))
            }
        } else {
            Err(FileSystemError::NotFound(normalized_path))
        }
    }
    
    async fn write_file(&self, path: &Path, content: &str) -> Result<(), FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let mut files = self.files.write().await;
        
        // Create parent directories if needed
        if let Some(parent) = normalized_path.parent() {
            self.ensure_parent_dirs(&mut files, parent);
        }
        
        let metadata = FileMetadata::new(normalized_path.clone(), FileType::File);
        let entry = FileEntry {
            metadata,
            content: Some(content.to_string()),
            children: None,
        };
        
        files.insert(normalized_path, entry);
        Ok(())
    }
    
    async fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let mut files = self.files.write().await;
        
        let metadata = FileMetadata::new(normalized_path.clone(), FileType::Directory);
        let entry = FileEntry {
            metadata,
            content: None,
            children: Some(Vec::new()),
        };
        
        files.insert(normalized_path, entry);
        Ok(())
    }
    
    async fn remove(&self, path: &Path) -> Result<(), FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let mut files = self.files.write().await;
        
        if files.remove(&normalized_path).is_some() {
            Ok(())
        } else {
            Err(FileSystemError::NotFound(normalized_path))
        }
    }
    
    async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>, FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let files = self.files.read().await;
        
        let mut entries = Vec::new();
        for (file_path, entry) in files.iter() {
            if let Some(parent) = file_path.parent() {
                if parent == normalized_path {
                    entries.push(entry.metadata.clone());
                }
            }
        }
        
        Ok(entries)
    }
    
    async fn metadata(&self, path: &Path) -> Result<FileMetadata, FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let files = self.files.read().await;
        
        if let Some(entry) = files.get(&normalized_path) {
            Ok(entry.metadata.clone())
        } else {
            Err(FileSystemError::NotFound(normalized_path))
        }
    }
    
    async fn exists(&self, path: &Path) -> Result<bool, FileSystemError> {
        let normalized_path = self.normalize_path(path);
        let files = self.files.read().await;
        Ok(files.contains_key(&normalized_path))
    }
    
    async fn copy(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        let from_normalized = self.normalize_path(from);
        let to_normalized = self.normalize_path(to);
        let mut files = self.files.write().await;
        
        if let Some(entry) = files.get(&from_normalized).cloned() {
            let mut new_entry = entry;
            new_entry.metadata.path = to_normalized.clone();
            files.insert(to_normalized, new_entry);
            Ok(())
        } else {
            Err(FileSystemError::NotFound(from_normalized))
        }
    }
    
    async fn move_file(&self, from: &Path, to: &Path) -> Result<(), FileSystemError> {
        self.copy(from, to).await?;
        self.remove(from).await?;
        Ok(())
    }
    
    async fn watch(&self, _path: &Path) -> Result<Box<dyn futures::Stream<Item = FileSystemEvent> + Send + Unpin>, FileSystemError> {
        // Virtual file system doesn't support watching
        Err(FileSystemError::NotSupported("File watching not supported in virtual file system".to_string()))
    }
    
    async fn current_dir(&self) -> Result<PathBuf, FileSystemError> {
        let current_dir = self.current_dir.read().await;
        Ok(current_dir.clone())
    }
    
    async fn set_current_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        let normalized_path = self.normalize_path(path);
        
        if !self.exists(&normalized_path).await? {
            return Err(FileSystemError::NotFound(normalized_path));
        }
        
        let metadata = self.metadata(&normalized_path).await?;
        if !metadata.is_dir() {
            return Err(FileSystemError::Generic("Path is not a directory".to_string()));
        }
        
        let mut current_dir = self.current_dir.write().await;
        *current_dir = normalized_path;
        Ok(())
    }
}

impl VirtualFileSystem {
    fn ensure_parent_dirs(&self, files: &mut HashMap<PathBuf, FileEntry>, path: &Path) {
        let mut current = PathBuf::from("/");
        
        for component in path.components() {
            current.push(component);
            
            if !files.contains_key(&current) {
                let metadata = FileMetadata::new(current.clone(), FileType::Directory);
                let entry = FileEntry {
                    metadata,
                    content: None,
                    children: Some(Vec::new()),
                };
                files.insert(current.clone(), entry);
            }
        }
    }
}

// Browser file system implementation (limited)
#[cfg(feature = "wasm-runtime")]
impl BrowserFileSystem {
    pub fn new() -> Self {
        BrowserFileSystem {
            current_dir: Arc::new(RwLock::new(PathBuf::from("/"))),
        }
    }
}

#[cfg(feature = "wasm-runtime")]
#[async_trait::async_trait]
impl FileSystem for BrowserFileSystem {
    async fn read_file(&self, _path: &Path) -> Result<String, FileSystemError> {
        Err(FileSystemError::NotSupported("File reading not supported in browser".to_string()))
    }
    
    async fn write_file(&self, _path: &Path, _content: &str) -> Result<(), FileSystemError> {
        Err(FileSystemError::NotSupported("File writing not supported in browser".to_string()))
    }
    
    async fn create_dir(&self, _path: &Path) -> Result<(), FileSystemError> {
        Err(FileSystemError::NotSupported("Directory creation not supported in browser".to_string()))
    }
    
    async fn remove(&self, _path: &Path) -> Result<(), FileSystemError> {
        Err(FileSystemError::NotSupported("File removal not supported in browser".to_string()))
    }
    
    async fn list_dir(&self, _path: &Path) -> Result<Vec<FileMetadata>, FileSystemError> {
        Err(FileSystemError::NotSupported("Directory listing not supported in browser".to_string()))
    }
    
    async fn metadata(&self, _path: &Path) -> Result<FileMetadata, FileSystemError> {
        Err(FileSystemError::NotSupported("File metadata not supported in browser".to_string()))
    }
    
    async fn exists(&self, _path: &Path) -> Result<bool, FileSystemError> {
        Ok(false)
    }
    
    async fn copy(&self, _from: &Path, _to: &Path) -> Result<(), FileSystemError> {
        Err(FileSystemError::NotSupported("File copying not supported in browser".to_string()))
    }
    
    async fn move_file(&self, _from: &Path, _to: &Path) -> Result<(), FileSystemError> {
        Err(FileSystemError::NotSupported("File moving not supported in browser".to_string()))
    }
    
    async fn watch(&self, _path: &Path) -> Result<Box<dyn futures::Stream<Item = FileSystemEvent> + Send + Unpin>, FileSystemError> {
        Err(FileSystemError::NotSupported("File watching not supported in browser".to_string()))
    }
    
    async fn current_dir(&self) -> Result<PathBuf, FileSystemError> {
        let current_dir = self.current_dir.read().await;
        Ok(current_dir.clone())
    }
    
    async fn set_current_dir(&self, _path: &Path) -> Result<(), FileSystemError> {
        Err(FileSystemError::NotSupported("Directory change not supported in browser".to_string()))
    }
}

// File system manager implementation
impl FileSystemManager {
    /// Create a new file system manager
    pub fn new() -> Self {
        let fs = create_default_filesystem();
        
        FileSystemManager {
            fs,
            watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create file system manager with custom file system
    pub fn with_filesystem(fs: Arc<dyn FileSystem>) -> Self {
        FileSystemManager {
            fs,
            watchers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get the underlying file system
    pub fn filesystem(&self) -> Arc<dyn FileSystem> {
        self.fs.clone()
    }
    
    /// Read file content
    pub async fn read_file(&self, path: &Path) -> Result<String, FileSystemError> {
        self.fs.read_file(path).await
    }
    
    /// Write file content
    pub async fn write_file(&self, path: &Path, content: &str) -> Result<(), FileSystemError> {
        self.fs.write_file(path, content).await
    }
    
    /// Create directory
    pub async fn create_dir(&self, path: &Path) -> Result<(), FileSystemError> {
        self.fs.create_dir(path).await
    }
    
    /// Remove file or directory
    pub async fn remove(&self, path: &Path) -> Result<(), FileSystemError> {
        self.fs.remove(path).await
    }
    
    /// List directory contents
    pub async fn list_dir(&self, path: &Path) -> Result<Vec<FileMetadata>, FileSystemError> {
        self.fs.list_dir(path).await
    }
    
    /// Check if path exists
    pub async fn exists(&self, path: &Path) -> Result<bool, FileSystemError> {
        self.fs.exists(path).await
    }
    
    /// Start watching a path for changes
    pub async fn start_watch(&self, path: &Path) -> Result<(), FileSystemError> {
        let stream = self.fs.watch(path).await?;
        let mut watchers = self.watchers.write().await;
        watchers.insert(path.to_path_buf(), stream);
        Ok(())
    }
    
    /// Stop watching a path
    pub async fn stop_watch(&self, path: &Path) {
        let mut watchers = self.watchers.write().await;
        watchers.remove(path);
    }
}

// Create default file system based on feature flags
fn create_default_filesystem() -> Arc<dyn FileSystem> {
    #[cfg(feature = "native-runtime")]
    {
        Arc::new(NativeFileSystem::new())
    }
    
    #[cfg(feature = "wasm-runtime")]
    {
        Arc::new(BrowserFileSystem::new())
    }
    
    #[cfg(not(any(feature = "native-runtime", feature = "wasm-runtime")))]
    {
        Arc::new(VirtualFileSystem::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_file_metadata_creation() {
        let metadata = FileMetadata::new(PathBuf::from("/test/file.txt"), FileType::File);
        assert_eq!(metadata.path, PathBuf::from("/test/file.txt"));
        assert_eq!(metadata.file_type, FileType::File);
        assert!(metadata.is_file());
        assert!(!metadata.is_dir());
    }
    
    #[tokio::test]
    async fn test_virtual_file_system() {
        let fs = VirtualFileSystem::new();
        
        // Test file operations
        let content = "Hello, world!";
        let path = Path::new("/test/file.txt");
        
        // Write file
        fs.write_file(path, content).await.unwrap();
        
        // Check if file exists
        assert!(fs.exists(path).await.unwrap());
        
        // Read file
        let read_content = fs.read_file(path).await.unwrap();
        assert_eq!(read_content, content);
        
        // Get metadata
        let metadata = fs.metadata(path).await.unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.path, path);
        
        // Test directory operations
        let dir_path = Path::new("/test/dir");
        fs.create_dir(dir_path).await.unwrap();
        
        assert!(fs.exists(dir_path).await.unwrap());
        let dir_metadata = fs.metadata(dir_path).await.unwrap();
        assert!(dir_metadata.is_dir());
        
        // List directory
        let entries = fs.list_dir(Path::new("/test")).await.unwrap();
        assert_eq!(entries.len(), 2); // file.txt and dir
        
        // Remove file
        fs.remove(path).await.unwrap();
        assert!(!fs.exists(path).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_file_system_manager() {
        let fs = Arc::new(VirtualFileSystem::new());
        let manager = FileSystemManager::with_filesystem(fs);
        
        let content = "Test content";
        let path = Path::new("/manager/test.txt");
        
        // Write and read through manager
        manager.write_file(path, content).await.unwrap();
        let read_content = manager.read_file(path).await.unwrap();
        assert_eq!(read_content, content);
        
        // Check existence
        assert!(manager.exists(path).await.unwrap());
        
        // List directory
        let entries = manager.list_dir(Path::new("/manager")).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, path);
    }
    
    #[test]
    fn test_file_type_conversion() {
        assert_eq!(FileType::File, FileType::File);
        assert_ne!(FileType::File, FileType::Directory);
        
        let metadata = FileMetadata::new(PathBuf::from("/test"), FileType::Directory);
        assert!(metadata.is_dir());
        assert!(!metadata.is_file());
    }
}