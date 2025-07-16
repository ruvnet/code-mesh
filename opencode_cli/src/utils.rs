//! Utility functions for the CLI

use anyhow::Result;
use std::path::Path;
use std::fs;

/// Check if a file exists
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// Check if a directory exists
pub fn dir_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// Create a directory if it doesn't exist
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
    }
    Ok(())
}

/// Get the config directory for OpenCode
pub fn get_config_dir() -> Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("opencode");
    
    ensure_dir(&config_dir)?;
    Ok(config_dir)
}

/// Get the data directory for OpenCode
pub fn get_data_dir() -> Result<std::path::PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine data directory"))?
        .join("opencode");
    
    ensure_dir(&data_dir)?;
    Ok(data_dir)
}

/// Get the cache directory for OpenCode
pub fn get_cache_dir() -> Result<std::path::PathBuf> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine cache directory"))?
        .join("opencode");
    
    ensure_dir(&cache_dir)?;
    Ok(cache_dir)
}

/// Format a timestamp for display
pub fn format_timestamp(timestamp: chrono::DateTime<chrono::Utc>) -> String {
    timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format a duration for display
pub fn format_duration(duration: std::time::Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    
    if secs >= 60 {
        let mins = secs / 60;
        let secs = secs % 60;
        if mins >= 60 {
            let hours = mins / 60;
            let mins = mins % 60;
            format!("{}h {}m {}s", hours, mins, secs)
        } else {
            format!("{}m {}s", mins, secs)
        }
    } else if secs > 0 {
        format!("{}.{:03}s", secs, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// Format a file size for display
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{:.0} {}", size, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

/// Truncate a string to a maximum length
pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

/// Wrap text to a specific width
pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    
    for word in text.split_whitespace() {
        let word_width = word.chars().count();
        
        if current_width + word_width + 1 <= width {
            if !current_line.is_empty() {
                current_line.push(' ');
                current_width += 1;
            }
            current_line.push_str(word);
            current_width += word_width;
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            current_line = word.to_string();
            current_width = word_width;
        }
    }
    
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    
    if lines.is_empty() {
        lines.push(String::new());
    }
    
    lines
}

/// Extract the first line from a multi-line string
pub fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

/// Count the number of lines in a string
pub fn count_lines(text: &str) -> usize {
    text.lines().count()
}

/// Check if a string is valid JSON
pub fn is_valid_json(text: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(text).is_ok()
}

/// Pretty print JSON
pub fn pretty_json(value: &serde_json::Value) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

/// Parse a JSON string
pub fn parse_json(text: &str) -> Result<serde_json::Value> {
    Ok(serde_json::from_str(text)?)
}

/// Get the terminal size
pub fn get_terminal_size() -> Result<(u16, u16)> {
    let (width, height) = crossterm::terminal::size()?;
    Ok((width, height))
}

/// Check if we're running in a TTY
pub fn is_tty() -> bool {
    atty::is(atty::Stream::Stdout)
}

/// Get environment variable with default
pub fn env_var_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Expand tilde in path
pub fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

/// Check if a command is available in PATH
pub fn command_exists(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Get the current working directory
pub fn get_current_dir() -> Result<std::path::PathBuf> {
    Ok(std::env::current_dir()?)
}

/// Generate a random string
pub fn random_string(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Convert a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_uppercase() {
                format!("-{}", c.to_lowercase())
            } else {
                c.to_string()
            }
        })
        .collect::<String>()
        .trim_start_matches('-')
        .to_string()
}

/// Convert a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_uppercase() {
                format!("_{}", c.to_lowercase())
            } else {
                c.to_string()
            }
        })
        .collect::<String>()
        .trim_start_matches('_')
        .to_string()
}

/// Create a spinner character sequence
pub fn spinner_chars() -> &'static [char] {
    &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
}

/// Get the next spinner character
pub fn next_spinner_char(index: usize) -> char {
    let chars = spinner_chars();
    chars[index % chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(std::time::Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration(std::time::Duration::from_secs(65)), "1m 5s");
        assert_eq!(format_duration(std::time::Duration::from_secs(3661)), "1h 1m 1s");
    }
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    }
    
    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
        assert_eq!(truncate_string("hi", 2), "hi");
    }
    
    #[test]
    fn test_wrap_text() {
        let text = "This is a long line that should be wrapped";
        let wrapped = wrap_text(text, 10);
        assert!(wrapped.len() > 1);
        assert!(wrapped[0].len() <= 10);
    }
    
    #[test]
    fn test_first_line() {
        assert_eq!(first_line("hello\nworld"), "hello");
        assert_eq!(first_line("single line"), "single line");
        assert_eq!(first_line(""), "");
    }
    
    #[test]
    fn test_count_lines() {
        assert_eq!(count_lines("hello\nworld"), 2);
        assert_eq!(count_lines("single line"), 1);
        assert_eq!(count_lines(""), 1);
    }
    
    #[test]
    fn test_is_valid_json() {
        assert!(is_valid_json(r#"{"key": "value"}"#));
        assert!(is_valid_json(r#"[]"#));
        assert!(is_valid_json(r#""string""#));
        assert!(!is_valid_json(r#"invalid json"#));
    }
    
    #[test]
    fn test_case_conversion() {
        assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
        assert_eq!(to_kebab_case("hello"), "hello");
        assert_eq!(to_kebab_case("HTML"), "h-t-m-l");
        
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("hello"), "hello");
        assert_eq!(to_snake_case("HTML"), "h_t_m_l");
    }
    
    #[test]
    fn test_expand_tilde() {
        let expanded = expand_tilde("~/test");
        assert!(!expanded.starts_with("~"));
        
        let no_tilde = expand_tilde("/absolute/path");
        assert_eq!(no_tilde, "/absolute/path");
    }
    
    #[test]
    fn test_random_string() {
        let s1 = random_string(10);
        let s2 = random_string(10);
        assert_eq!(s1.len(), 10);
        assert_eq!(s2.len(), 10);
        assert_ne!(s1, s2); // Very unlikely to be equal
    }
    
    #[test]
    fn test_spinner_chars() {
        let chars = spinner_chars();
        assert!(!chars.is_empty());
        
        let char1 = next_spinner_char(0);
        let char2 = next_spinner_char(1);
        assert_ne!(char1, char2);
        
        let char_wrapped = next_spinner_char(chars.len());
        assert_eq!(char_wrapped, chars[0]);
    }
}