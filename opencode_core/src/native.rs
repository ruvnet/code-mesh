//! Native-specific functionality for OpenCode
//!
//! This module provides native-only features like command execution,
//! file watching, and OS integration.

#[cfg(feature = "native-runtime")]
use crate::OpenCodeResult;
#[cfg(feature = "native-runtime")]
use std::path::Path;
#[cfg(feature = "native-runtime")]
use std::process::Command;
#[cfg(feature = "native-runtime")]
use tokio::process::Command as TokioCommand;

/// Command execution result
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// Exit status code
    pub status: i32,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
    
    /// Command that was executed
    pub command: String,
    
    /// Execution time
    pub duration: std::time::Duration,
}

/// Command execution errors
#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("Command not found: {0}")]
    NotFound(String),
    
    #[error("Command failed with exit code {code}: {stderr}")]
    Failed { code: i32, stderr: String },
    
    #[error("Command timeout after {seconds} seconds")]
    Timeout { seconds: u64 },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
    
    #[error("Generic error: {0}")]
    Generic(String),
}

/// Command executor for native environments
#[cfg(feature = "native-runtime")]
pub struct CommandExecutor {
    /// Working directory
    working_dir: std::path::PathBuf,
    
    /// Environment variables
    env_vars: std::collections::HashMap<String, String>,
    
    /// Default timeout in seconds
    default_timeout: u64,
}

#[cfg(feature = "native-runtime")]
impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Self {
        let working_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        
        CommandExecutor {
            working_dir,
            env_vars: std::collections::HashMap::new(),
            default_timeout: 30,
        }
    }
    
    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = dir.into();
        self
    }
    
    /// Set environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
    
    /// Set default timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.default_timeout = seconds;
        self
    }
    
    /// Execute a command
    pub async fn execute(&self, command: &str) -> Result<CommandResult, CommandError> {
        self.execute_with_timeout(command, self.default_timeout).await
    }
    
    /// Execute a command with custom timeout
    pub async fn execute_with_timeout(&self, command: &str, timeout_seconds: u64) -> Result<CommandResult, CommandError> {
        let start_time = std::time::Instant::now();
        
        // Parse command and arguments
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(CommandError::Generic("Empty command".to_string()));
        }
        
        let program = parts[0];
        let args = &parts[1..];
        
        // Check if command exists
        if which::which(program).is_err() {
            return Err(CommandError::NotFound(program.to_string()));
        }
        
        // Create command
        let mut cmd = TokioCommand::new(program);
        cmd.args(args);
        cmd.current_dir(&self.working_dir);
        
        // Set environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }
        
        // Execute with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_seconds),
            cmd.output(),
        ).await;
        
        let duration = start_time.elapsed();
        
        match output {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8(output.stdout)?;
                let stderr = String::from_utf8(output.stderr)?;
                let status = output.status.code().unwrap_or(-1);
                
                let result = CommandResult {
                    status,
                    stdout,
                    stderr: stderr.clone(),
                    command: command.to_string(),
                    duration,
                };
                
                if status != 0 {
                    Err(CommandError::Failed {
                        code: status,
                        stderr,
                    })
                } else {
                    Ok(result)
                }
            }
            Ok(Err(e)) => Err(CommandError::Io(e)),
            Err(_) => Err(CommandError::Timeout { seconds: timeout_seconds }),
        }
    }
    
    /// Execute a command and return only stdout
    pub async fn execute_output(&self, command: &str) -> Result<String, CommandError> {
        let result = self.execute(command).await?;
        Ok(result.stdout)
    }
    
    /// Check if a command exists
    pub fn command_exists(&self, command: &str) -> bool {
        which::which(command).is_ok()
    }
    
    /// Get available commands in PATH
    pub fn get_available_commands(&self) -> Vec<String> {
        let mut commands = Vec::new();
        
        if let Ok(path) = std::env::var("PATH") {
            for dir in path.split(':') {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if entry.metadata().map_or(false, |m| m.is_file()) {
                                commands.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }
        
        commands.sort();
        commands.dedup();
        commands
    }
}

/// System information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    /// Operating system name
    pub os: String,
    
    /// OS version
    pub version: String,
    
    /// Architecture
    pub arch: String,
    
    /// Hostname
    pub hostname: String,
    
    /// Number of CPU cores
    pub cpu_cores: usize,
    
    /// Total memory in bytes
    pub total_memory: u64,
    
    /// Available memory in bytes
    pub available_memory: u64,
    
    /// Current working directory
    pub current_dir: std::path::PathBuf,
    
    /// Environment variables
    pub env_vars: std::collections::HashMap<String, String>,
}

/// Get system information
#[cfg(feature = "native-runtime")]
pub fn get_system_info() -> OpenCodeResult<SystemInfo> {
    let os = std::env::consts::OS.to_string();
    let arch = std::env::consts::ARCH.to_string();
    let current_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    
    // Get hostname
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    
    // Get CPU cores
    let cpu_cores = num_cpus::get();
    
    // Get memory info (simplified)
    let total_memory = 0; // Would need platform-specific code
    let available_memory = 0; // Would need platform-specific code
    
    // Get environment variables
    let env_vars: std::collections::HashMap<String, String> = std::env::vars().collect();
    
    Ok(SystemInfo {
        os,
        version: "unknown".to_string(), // Would need platform-specific code
        arch,
        hostname,
        cpu_cores,
        total_memory,
        available_memory,
        current_dir,
        env_vars,
    })
}

/// File watcher for native environments
#[cfg(all(feature = "native-runtime", feature = "file-watching"))]
pub struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    receiver: tokio::sync::mpsc::UnboundedReceiver<notify::Event>,
}

#[cfg(all(feature = "native-runtime", feature = "file-watching"))]
impl FileWatcher {
    /// Create a new file watcher
    pub fn new() -> Result<Self, CommandError> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        
        let watcher = notify::Watcher::new(move |res: Result<notify::Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        }, notify::Config::default())
        .map_err(|e| CommandError::Generic(e.to_string()))?;
        
        Ok(FileWatcher {
            watcher,
            receiver: rx,
        })
    }
    
    /// Watch a path
    pub fn watch(&mut self, path: &Path) -> Result<(), CommandError> {
        self.watcher
            .watch(path, notify::RecursiveMode::Recursive)
            .map_err(|e| CommandError::Generic(e.to_string()))?;
        Ok(())
    }
    
    /// Stop watching a path
    pub fn unwatch(&mut self, path: &Path) -> Result<(), CommandError> {
        self.watcher
            .unwatch(path)
            .map_err(|e| CommandError::Generic(e.to_string()))?;
        Ok(())
    }
    
    /// Get the next event
    pub async fn next_event(&mut self) -> Option<notify::Event> {
        self.receiver.recv().await
    }
}

/// Network utilities for native environments
#[cfg(feature = "native-runtime")]
pub mod network {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::time::Duration;
    
    /// Check if a port is available
    pub fn is_port_available(port: u16) -> bool {
        std::net::TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)).is_ok()
    }
    
    /// Find an available port in a range
    pub fn find_available_port(start: u16, end: u16) -> Option<u16> {
        for port in start..=end {
            if is_port_available(port) {
                return Some(port);
            }
        }
        None
    }
    
    /// Get local IP address
    pub fn get_local_ip() -> Option<IpAddr> {
        // Simple implementation - would need more sophisticated logic
        Some(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }
    
    /// Check if a host is reachable
    pub async fn is_host_reachable(host: &str, port: u16, timeout: Duration) -> bool {
        let addr = format!("{}:{}", host, port);
        tokio::time::timeout(timeout, tokio::net::TcpStream::connect(addr))
            .await
            .is_ok()
    }
}

/// Process management utilities
#[cfg(feature = "native-runtime")]
pub mod process {
    use std::process::Child;
    use std::collections::HashMap;
    
    /// Process manager for long-running processes
    pub struct ProcessManager {
        processes: HashMap<String, Child>,
    }
    
    impl ProcessManager {
        /// Create a new process manager
        pub fn new() -> Self {
            ProcessManager {
                processes: HashMap::new(),
            }
        }
        
        /// Start a new process
        pub fn start_process(&mut self, name: String, command: &str) -> Result<(), super::CommandError> {
            let parts: Vec<&str> = command.split_whitespace().collect();
            if parts.is_empty() {
                return Err(super::CommandError::Generic("Empty command".to_string()));
            }
            
            let mut cmd = std::process::Command::new(parts[0]);
            cmd.args(&parts[1..]);
            
            let child = cmd.spawn()
                .map_err(super::CommandError::Io)?;
            
            self.processes.insert(name, child);
            Ok(())
        }
        
        /// Stop a process
        pub fn stop_process(&mut self, name: &str) -> Result<(), super::CommandError> {
            if let Some(mut child) = self.processes.remove(name) {
                child.kill().map_err(super::CommandError::Io)?;
                let _ = child.wait();
            }
            Ok(())
        }
        
        /// Check if a process is running
        pub fn is_running(&mut self, name: &str) -> bool {
            if let Some(child) = self.processes.get_mut(name) {
                match child.try_wait() {
                    Ok(Some(_)) => {
                        // Process has exited
                        self.processes.remove(name);
                        false
                    }
                    Ok(None) => true, // Still running
                    Err(_) => false,  // Error checking status
                }
            } else {
                false
            }
        }
        
        /// List running processes
        pub fn list_processes(&mut self) -> Vec<String> {
            // Clean up finished processes
            let mut finished = Vec::new();
            for (name, child) in &mut self.processes {
                if let Ok(Some(_)) = child.try_wait() {
                    finished.push(name.clone());
                }
            }
            
            for name in finished {
                self.processes.remove(&name);
            }
            
            self.processes.keys().cloned().collect()
        }
        
        /// Stop all processes
        pub fn stop_all(&mut self) {
            let names: Vec<String> = self.processes.keys().cloned().collect();
            for name in names {
                let _ = self.stop_process(&name);
            }
        }
    }
    
    impl Drop for ProcessManager {
        fn drop(&mut self) {
            self.stop_all();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[cfg(feature = "native-runtime")]
    #[tokio::test]
    async fn test_command_executor() {
        let executor = CommandExecutor::new();
        
        // Test simple command
        let result = executor.execute("echo hello").await;
        assert!(result.is_ok());
        
        if let Ok(result) = result {
            assert_eq!(result.status, 0);
            assert_eq!(result.stdout.trim(), "hello");
        }
    }
    
    #[cfg(feature = "native-runtime")]
    #[tokio::test]
    async fn test_command_not_found() {
        let executor = CommandExecutor::new();
        
        let result = executor.execute("nonexistent_command").await;
        assert!(result.is_err());
        
        if let Err(CommandError::NotFound(_)) = result {
            // Expected
        } else {
            panic!("Expected NotFound error");
        }
    }
    
    #[cfg(feature = "native-runtime")]
    #[test]
    fn test_command_exists() {
        let executor = CommandExecutor::new();
        
        // These commands should exist on most systems
        assert!(executor.command_exists("echo"));
        assert!(!executor.command_exists("nonexistent_command_12345"));
    }
    
    #[cfg(feature = "native-runtime")]
    #[test]
    fn test_system_info() {
        let info = get_system_info().unwrap();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.cpu_cores > 0);
    }
    
    #[cfg(feature = "native-runtime")]
    #[test]
    fn test_network_utils() {
        use network::*;
        
        // Test port availability
        assert!(is_port_available(0)); // Port 0 should always be available for binding
        
        // Test finding available port
        let port = find_available_port(8000, 8100);
        assert!(port.is_some());
    }
    
    #[cfg(feature = "native-runtime")]
    #[test]
    fn test_process_manager() {
        use process::*;
        
        let mut manager = ProcessManager::new();
        
        // Start a simple process
        let result = manager.start_process("test".to_string(), "sleep 1");
        assert!(result.is_ok());
        
        // Check if it's running
        assert!(manager.is_running("test"));
        
        // Stop it
        let result = manager.stop_process("test");
        assert!(result.is_ok());
        
        // Should not be running anymore
        assert!(!manager.is_running("test"));
    }
}