//! Utility functions for the CLI

use crate::cmd::{CliError, Result, UI};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Utility functions
pub struct Utils;

impl Utils {
    /// Get the current timestamp as a string
    pub fn timestamp() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string()
    }

    /// Generate a random ID
    pub fn generate_id() -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        SystemTime::now().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if a command exists in PATH
    pub fn command_exists(command: &str) -> bool {
        which::which(command).is_ok()
    }

    /// Run a shell command and return output
    pub fn run_command(command: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(command)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| CliError::CommandFailed(format!("Failed to execute {}: {}", command, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CliError::CommandFailed(format!(
                "Command {} failed: {}",
                command, stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Check if we're running in a git repository
    pub fn is_git_repo(path: &Path) -> bool {
        path.join(".git").exists() || 
        path.ancestors().any(|p| p.join(".git").exists())
    }

    /// Get git root directory
    pub fn git_root(path: &Path) -> Option<PathBuf> {
        for ancestor in path.ancestors() {
            if ancestor.join(".git").exists() {
                return Some(ancestor.to_path_buf());
            }
        }
        None
    }

    /// Check if a file has a specific extension
    pub fn has_extension(path: &Path, ext: &str) -> bool {
        path.extension()
            .and_then(|s| s.to_str())
            .map(|s| s.eq_ignore_ascii_case(ext))
            .unwrap_or(false)
    }

    /// Get file size in bytes
    pub fn file_size(path: &Path) -> Result<u64> {
        fs::metadata(path)
            .map(|m| m.len())
            .map_err(|e| CliError::FileSystem(format!("Failed to get file size: {}", e)))
    }

    /// Format file size as human readable
    pub fn format_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    /// Format duration as human readable
    pub fn format_duration(duration: Duration) -> String {
        let secs = duration.as_secs();
        
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Truncate string to max length with ellipsis
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    /// Get terminal width
    pub fn terminal_width() -> usize {
        terminal_size::terminal_size()
            .map(|(terminal_size::Width(w), _)| w as usize)
            .unwrap_or(80)
    }

    /// Check if terminal supports colors
    pub fn supports_color() -> bool {
        atty::is(atty::Stream::Stderr) && 
        std::env::var("NO_COLOR").is_err() &&
        std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true)
    }

    /// Open URL in default browser
    pub fn open_url(url: &str) -> Result<()> {
        open::that(url)
            .map_err(|e| CliError::CommandFailed(format!("Failed to open URL: {}", e)))?;
        Ok(())
    }

    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        url.starts_with("http://") || url.starts_with("https://")
    }

    /// Create a backup of a file
    pub fn backup_file(path: &Path) -> Result<PathBuf> {
        if !path.exists() {
            return Err(CliError::FileSystem("File does not exist".to_string()));
        }

        let backup_path = path.with_extension(format!(
            "{}.backup.{}",
            path.extension().and_then(|s| s.to_str()).unwrap_or(""),
            Self::timestamp()
        ));

        fs::copy(path, &backup_path)
            .map_err(|e| CliError::FileSystem(format!("Failed to create backup: {}", e)))?;

        Ok(backup_path)
    }

    /// Clean up old backup files
    pub fn cleanup_backups(dir: &Path, keep_count: usize) -> Result<usize> {
        let mut backups = Vec::new();
        
        for entry in fs::read_dir(dir)
            .map_err(|e| CliError::FileSystem(format!("Failed to read directory: {}", e)))?
        {
            let entry = entry
                .map_err(|e| CliError::FileSystem(format!("Failed to read entry: {}", e)))?;
            
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.contains(".backup.") {
                    backups.push(path);
                }
            }
        }

        // Sort by modification time (newest first)
        backups.sort_by_key(|path| {
            fs::metadata(path)
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        backups.reverse();

        let mut removed = 0;
        for backup in backups.iter().skip(keep_count) {
            if fs::remove_file(backup).is_ok() {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

/// Project detection utilities
pub struct ProjectDetector;

impl ProjectDetector {
    /// Detect project type based on files in directory
    pub fn detect_project_type(path: &Path) -> Option<String> {
        let files = fs::read_dir(path).ok()?;
        let mut file_names = Vec::new();

        for entry in files.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                file_names.push(name.to_string());
            }
        }

        // Check for specific project files
        if file_names.contains(&"Cargo.toml".to_string()) {
            return Some("rust".to_string());
        }
        
        if file_names.contains(&"package.json".to_string()) {
            return Some("javascript".to_string());
        }
        
        if file_names.contains(&"pyproject.toml".to_string()) || 
           file_names.contains(&"setup.py".to_string()) ||
           file_names.contains(&"requirements.txt".to_string()) {
            return Some("python".to_string());
        }
        
        if file_names.contains(&"go.mod".to_string()) {
            return Some("go".to_string());
        }
        
        if file_names.contains(&"pom.xml".to_string()) ||
           file_names.contains(&"build.gradle".to_string()) {
            return Some("java".to_string());
        }

        None
    }

    /// Get project name from current directory
    pub fn get_project_name(path: &Path) -> String {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Check if directory is a valid project root
    pub fn is_project_root(path: &Path) -> bool {
        Self::detect_project_type(path).is_some() || Utils::is_git_repo(path)
    }
}

/// Input validation utilities
pub struct Validator;

impl Validator {
    /// Validate model name format (provider/model)
    pub fn validate_model_name(model: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = model.split('/').collect();
        if parts.len() != 2 {
            return Err(CliError::InvalidInput(
                "Model must be in format 'provider/model'".to_string(),
            ));
        }

        let provider = parts[0].trim();
        let model = parts[1].trim();

        if provider.is_empty() || model.is_empty() {
            return Err(CliError::InvalidInput(
                "Provider and model names cannot be empty".to_string(),
            ));
        }

        Ok((provider.to_string(), model.to_string()))
    }

    /// Validate session ID format
    pub fn validate_session_id(session_id: &str) -> Result<()> {
        if session_id.is_empty() {
            return Err(CliError::InvalidInput("Session ID cannot be empty".to_string()));
        }

        if !session_id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
            return Err(CliError::InvalidInput(
                "Session ID can only contain alphanumeric characters, hyphens, and underscores".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate port number
    pub fn validate_port(port: u16) -> Result<()> {
        if port < 1024 {
            return Err(CliError::InvalidInput(
                "Port number must be 1024 or higher".to_string(),
            ));
        }
        Ok(())
    }

    /// Validate timeout value
    pub fn validate_timeout(timeout: u64) -> Result<()> {
        if timeout == 0 {
            return Err(CliError::InvalidInput(
                "Timeout must be greater than 0".to_string(),
            ));
        }
        if timeout > 3600 {
            return Err(CliError::InvalidInput(
                "Timeout cannot be more than 1 hour (3600 seconds)".to_string(),
            ));
        }
        Ok(())
    }
}