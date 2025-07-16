//! Utility functions and helpers for Code Mesh Core

use crate::{Error, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ::url::Url as UrlType;

/// File system utilities
pub mod fs {
    use super::*;
    use std::fs;
    
    /// Safely canonicalize a path, handling cases where the path doesn't exist
    pub fn safe_canonicalize<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
        let path = path.as_ref();
        
        // Try direct canonicalization first
        if let Ok(canonical) = fs::canonicalize(path) {
            return Ok(canonical);
        }
        
        // If that fails, try to canonicalize the parent and append the filename
        if let Some(parent) = path.parent() {
            if let Some(filename) = path.file_name() {
                if let Ok(parent_canonical) = fs::canonicalize(parent) {
                    return Ok(parent_canonical.join(filename));
                }
            }
        }
        
        // Fall back to absolute path
        let current_dir = std::env::current_dir()?;
        Ok(current_dir.join(path))
    }
    
    /// Get file size safely
    pub fn file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
        let metadata = fs::metadata(path)?;
        Ok(metadata.len())
    }
    
    /// Check if a path is safe to access (no directory traversal)
    pub fn is_safe_path<P: AsRef<Path>>(base: P, target: P) -> Result<bool> {
        let base = safe_canonicalize(base)?;
        let target = safe_canonicalize(target)?;
        
        Ok(target.starts_with(base))
    }
    
    /// Create directories recursively with proper error handling
    pub fn ensure_dir<P: AsRef<Path>>(path: P) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }
    
    /// Get file extension as lowercase string
    pub fn file_extension<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
    }
    
    /// Check if file is binary based on content
    pub fn is_binary_file<P: AsRef<Path>>(path: P) -> Result<bool> {
        let mut buffer = [0; 1024];
        let file = std::fs::File::open(path)?;
        let bytes_read = std::io::Read::read(&mut std::io::BufReader::new(file), &mut buffer)?;
        
        // Check for null bytes, which typically indicate binary content
        Ok(buffer[..bytes_read].contains(&0))
    }
}

/// String utilities
pub mod string {
    use super::*;
    
    /// Truncate string to specified length with ellipsis
    pub fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else if max_len <= 3 {
            "...".to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }
    
    /// Escape string for safe inclusion in JSON
    pub fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }
    
    /// Clean string for use as filename
    pub fn sanitize_filename(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '/' | '\\' | '?' | '%' | '*' | ':' | '|' | '"' | '<' | '>' => '_',
                c if c.is_control() => '_',
                c => c,
            })
            .collect()
    }
    
    /// Convert camelCase to snake_case
    pub fn camel_to_snake(s: &str) -> String {
        let mut result = String::new();
        let mut prev_lowercase = false;
        
        for c in s.chars() {
            if c.is_uppercase() && prev_lowercase {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
            prev_lowercase = c.is_lowercase();
        }
        
        result
    }
    
    /// Extract lines around a specific line number
    pub fn extract_context(content: &str, line_number: usize, context: usize) -> Vec<(usize, &str)> {
        let lines: Vec<&str> = content.lines().collect();
        let start = line_number.saturating_sub(context);
        let end = (line_number + context + 1).min(lines.len());
        
        lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| (start + i + 1, *line))
            .collect()
    }
}

/// Time utilities
pub mod time {
    use super::*;
    
    /// Get current timestamp as seconds since Unix epoch
    pub fn now_timestamp() -> Result<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .map_err(|e| Error::Other(anyhow::anyhow!("Time error: {}", e)))
    }
    
    /// Format duration in human-readable form
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        
        if total_seconds < 60 {
            format!("{}s", total_seconds)
        } else if total_seconds < 3600 {
            format!("{}m {}s", total_seconds / 60, total_seconds % 60)
        } else if total_seconds < 86400 {
            format!(
                "{}h {}m", 
                total_seconds / 3600, 
                (total_seconds % 3600) / 60
            )
        } else {
            format!(
                "{}d {}h", 
                total_seconds / 86400, 
                (total_seconds % 86400) / 3600
            )
        }
    }
    
    /// Parse duration from human-readable string
    pub fn parse_duration(s: &str) -> Result<Duration> {
        let s = s.trim().to_lowercase();
        
        if let Ok(seconds) = s.parse::<u64>() {
            return Ok(Duration::from_secs(seconds));
        }
        
        if s.ends_with("ms") {
            let ms = s[..s.len() - 2].parse::<u64>()?;
            return Ok(Duration::from_millis(ms));
        }
        
        if s.ends_with('s') {
            let secs = s[..s.len() - 1].parse::<u64>()?;
            return Ok(Duration::from_secs(secs));
        }
        
        if s.ends_with('m') {
            let mins = s[..s.len() - 1].parse::<u64>()?;
            return Ok(Duration::from_secs(mins * 60));
        }
        
        if s.ends_with('h') {
            let hours = s[..s.len() - 1].parse::<u64>()?;
            return Ok(Duration::from_secs(hours * 3600));
        }
        
        if s.ends_with('d') {
            let days = s[..s.len() - 1].parse::<u64>()?;
            return Ok(Duration::from_secs(days * 86400));
        }
        
        Err(Error::Other(anyhow::anyhow!("Invalid duration format: {}", s)))
    }
}

/// Hash utilities
pub mod hash {
    use super::*;
    use sha2::{Sha256, Digest};
    
    /// Generate SHA-256 hash of content
    pub fn sha256(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        hex::encode(hasher.finalize())
    }
    
    /// Generate SHA-256 hash of string
    pub fn sha256_string(content: &str) -> String {
        sha256(content.as_bytes())
    }
    
    /// Generate content hash for caching
    pub fn content_hash(content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// URL utilities
pub mod url {
    use super::*;
    
    /// Validate URL format
    pub fn is_valid_url(s: &str) -> bool {
        UrlType::parse(s).is_ok()
    }
    
    /// Extract domain from URL
    pub fn extract_domain(url_str: &str) -> Result<String> {
        let url = UrlType::parse(url_str)
            .map_err(|e| Error::Other(anyhow::anyhow!("Invalid URL: {}", e)))?;
        
        url.host_str()
            .map(|host| host.to_string())
            .ok_or_else(|| Error::Other(anyhow::anyhow!("No host in URL")))
    }
    
    /// Join URL paths safely
    pub fn join_path(base: &str, path: &str) -> Result<String> {
        let mut url = UrlType::parse(base)
            .map_err(|e| Error::Other(anyhow::anyhow!("Invalid base URL: {}", e)))?;
        
        url = url.join(path)
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to join path: {}", e)))?;
        
        Ok(url.to_string())
    }
}

/// Process utilities
pub mod process {
    use super::*;
    
    /// Check if a command exists in PATH
    pub fn command_exists(command: &str) -> bool {
        which::which(command).is_ok()
    }
    
    /// Get available shell command
    pub fn get_shell() -> String {
        if cfg!(windows) {
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd".to_string())
        } else {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        }
    }
    
    /// Escape shell argument
    pub fn escape_shell_arg(arg: &str) -> String {
        if cfg!(windows) {
            // Windows shell escaping
            if arg.contains(' ') || arg.contains('"') {
                format!("\"{}\"", arg.replace('"', "\\\""))
            } else {
                arg.to_string()
            }
        } else {
            // Unix shell escaping
            if arg.chars().any(|c| " \t\n\r\"'\\|&;<>()$`".contains(c)) {
                format!("'{}'", arg.replace('\'', "'\"'\"'"))
            } else {
                arg.to_string()
            }
        }
    }
}

/// Memory utilities
pub mod memory {
    use super::*;
    
    /// Format bytes in human-readable form
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        const THRESHOLD: u64 = 1024;
        
        if bytes < THRESHOLD {
            return format!("{} B", bytes);
        }
        
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
            size /= THRESHOLD as f64;
            unit_index += 1;
        }
        
        format!("{:.1} {}", size, UNITS[unit_index])
    }
    
    /// Parse bytes from human-readable string
    pub fn parse_bytes(s: &str) -> Result<u64> {
        let s = s.trim().to_uppercase();
        
        if let Ok(bytes) = s.parse::<u64>() {
            return Ok(bytes);
        }
        
        let (number_part, unit_part) = if s.ends_with('B') {
            let unit_start = s.len() - if s.ends_with("KB") || s.ends_with("MB") || s.ends_with("GB") || s.ends_with("TB") { 2 } else { 1 };
            (s[..unit_start].trim(), &s[unit_start..])
        } else {
            (s.as_str(), "B")
        };
        
        let number: f64 = number_part.parse()
            .map_err(|_| Error::Other(anyhow::anyhow!("Invalid number: {}", number_part)))?;
        
        let multiplier = match unit_part {
            "B" => 1,
            "KB" => 1024,
            "MB" => 1024 * 1024,
            "GB" => 1024 * 1024 * 1024,
            "TB" => 1024_u64.pow(4),
            _ => return Err(Error::Other(anyhow::anyhow!("Invalid unit: {}", unit_part))),
        };
        
        Ok((number * multiplier as f64) as u64)
    }
}

/// Validation utilities
pub mod validation {
    use super::*;
    
    /// Validate email format
    pub fn is_valid_email(email: &str) -> bool {
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
        email_regex.is_match(email)
    }
    
    /// Validate API key format (basic check)
    pub fn is_valid_api_key(key: &str) -> bool {
        !key.is_empty() && key.len() >= 10 && key.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }
    
    /// Validate session ID format
    pub fn is_valid_session_id(id: &str) -> bool {
        !id.is_empty() && id.len() <= 256 && id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }
    
    /// Validate model name format
    pub fn is_valid_model_name(name: &str) -> bool {
        !name.is_empty() && name.len() <= 100 && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '/' || c == '.')
    }
}

/// Configuration utilities
pub mod config {
    use super::*;
    
    /// Get configuration directory for the application
    pub fn config_dir() -> Result<PathBuf> {
        dirs::config_dir()
            .map(|dir| dir.join("code-mesh"))
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Could not find config directory")))
    }
    
    /// Get data directory for the application
    pub fn data_dir() -> Result<PathBuf> {
        dirs::data_dir()
            .map(|dir| dir.join("code-mesh"))
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Could not find data directory")))
    }
    
    /// Get cache directory for the application
    pub fn cache_dir() -> Result<PathBuf> {
        dirs::cache_dir()
            .map(|dir| dir.join("code-mesh"))
            .ok_or_else(|| Error::Other(anyhow::anyhow!("Could not find cache directory")))
    }
    
    /// Ensure all application directories exist
    pub fn ensure_app_dirs() -> Result<()> {
        fs::ensure_dir(config_dir()?)?;
        fs::ensure_dir(data_dir()?)?;
        fs::ensure_dir(cache_dir()?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_truncate() {
        assert_eq!(string::truncate("hello", 10), "hello");
        assert_eq!(string::truncate("hello world", 8), "hello...");
        assert_eq!(string::truncate("hi", 1), "...");
    }

    #[test]
    fn test_string_sanitize_filename() {
        assert_eq!(string::sanitize_filename("hello/world"), "hello_world");
        assert_eq!(string::sanitize_filename("file?.txt"), "file_.txt");
    }

    #[test]
    fn test_camel_to_snake() {
        assert_eq!(string::camel_to_snake("camelCase"), "camel_case");
        assert_eq!(string::camel_to_snake("HTTPSConnection"), "h_t_t_p_s_connection");
        assert_eq!(string::camel_to_snake("simple"), "simple");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(memory::format_bytes(512), "512 B");
        assert_eq!(memory::format_bytes(1024), "1.0 KB");
        assert_eq!(memory::format_bytes(1536), "1.5 KB");
        assert_eq!(memory::format_bytes(1024 * 1024), "1.0 MB");
    }

    #[test]
    fn test_parse_duration() -> Result<()> {
        assert_eq!(time::parse_duration("30s")?, Duration::from_secs(30));
        assert_eq!(time::parse_duration("5m")?, Duration::from_secs(300));
        assert_eq!(time::parse_duration("2h")?, Duration::from_secs(7200));
        assert_eq!(time::parse_duration("1d")?, Duration::from_secs(86400));
        Ok(())
    }

    #[test]
    fn test_validation() {
        assert!(validation::is_valid_email("test@example.com"));
        assert!(!validation::is_valid_email("invalid-email"));
        
        assert!(validation::is_valid_api_key("sk-1234567890abcdef"));
        assert!(!validation::is_valid_api_key("short"));
        
        assert!(validation::is_valid_model_name("anthropic/claude-3-opus"));
        assert!(!validation::is_valid_model_name(""));
    }
}