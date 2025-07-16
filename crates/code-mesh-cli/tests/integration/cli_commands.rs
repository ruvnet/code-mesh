//! CLI command integration tests

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::*;

#[test]
fn test_cli_help_command() {
    let mut cmd = test_command();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("code-mesh"))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_version_command() {
    let mut cmd = test_command();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("code-mesh"));
}

#[test]
fn test_auth_command() {
    let env = TestEnvironment::new();
    
    // Test auth status (should be unauthorized initially)
    env.command()
        .args(&["auth", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Not authenticated"));
}

#[test]
fn test_auth_login_command() {
    let env = TestEnvironment::new();
    
    // Test auth login with test credentials
    env.command()
        .args(&["auth", "login", "--provider", "mock", "--token", "test-token"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully authenticated"));
}

#[test]
fn test_auth_logout_command() {
    let env = TestEnvironment::new();
    
    // First login
    env.command()
        .args(&["auth", "login", "--provider", "mock", "--token", "test-token"])
        .assert()
        .success();
    
    // Then logout
    env.command()
        .args(&["auth", "logout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully logged out"));
}

#[test]
fn test_session_commands() {
    let env = TestEnvironment::new();
    
    // Test list sessions (should be empty initially)
    env.command()
        .args(&["session", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No sessions found").or(predicate::str::contains("[]")));
}

#[test]
fn test_session_create_command() {
    let env = TestEnvironment::new();
    
    // Create a new session
    env.command()
        .args(&["session", "create", "--name", "test-session"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created session"));
}

#[test]
fn test_chat_command_with_mock() {
    let env = TestEnvironment::new();
    
    // First authenticate
    env.command()
        .args(&["auth", "login", "--provider", "mock", "--token", "test-token"])
        .assert()
        .success();
    
    // Test chat command
    env.command()
        .args(&["chat", "--message", "Hello, world!", "--mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Mock response"));
}

#[test]
fn test_file_operations() {
    let env = TestEnvironment::new();
    
    // Create a test file
    let test_file = env.temp_dir.path().join("test.txt");
    std::fs::write(&test_file, "Hello, World!").unwrap();
    
    // Test read command
    env.command()
        .args(&["file", "read", test_file.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello, World!"));
}

#[test]
fn test_config_commands() {
    let env = TestEnvironment::new();
    
    // Test config show
    env.command()
        .args(&["config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("auth").or(predicate::str::contains("llm")));
}

#[test]
fn test_config_set_command() {
    let env = TestEnvironment::new();
    
    // Test config set
    env.command()
        .args(&["config", "set", "llm.max_tokens", "2000"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration updated"));
}

#[test]
fn test_tool_list_command() {
    let env = TestEnvironment::new();
    
    // Test list available tools
    env.command()
        .args(&["tool", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Available tools:"));
}

#[test]
fn test_tool_execute_command() {
    let env = TestEnvironment::new();
    
    // Test tool execution with mock tool
    env.command()
        .args(&[
            "tool", 
            "execute", 
            "echo",
            "--params", 
            r#"{"message": "Hello from tool"}"#
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello from tool"));
}

#[test]
fn test_server_start_command() {
    let env = TestEnvironment::new();
    
    // Test server start (should fail without proper setup, but command should be recognized)
    env.command()
        .args(&["server", "start", "--port", "3000", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Server would start"));
}

#[test]
fn test_invalid_command() {
    let env = TestEnvironment::new();
    
    // Test invalid command
    env.command()
        .args(&["invalid-command"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:").or(predicate::str::contains("command")));
}

#[test]
fn test_json_output_format() {
    let env = TestEnvironment::new();
    
    // Test JSON output format
    let output = env.command()
        .args(&["session", "list", "--format", "json"])
        .output()
        .unwrap();
    
    assertions::assert_json_output(&output);
}

#[test]
fn test_verbose_output() {
    let env = TestEnvironment::new();
    
    // Test verbose mode
    env.command()
        .args(&["--verbose", "auth", "status"])
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG").or(predicate::str::contains("TRACE")));
}

#[test]
fn test_quiet_mode() {
    let env = TestEnvironment::new();
    
    // Test quiet mode
    env.command()
        .args(&["--quiet", "auth", "status"])
        .assert()
        .success();
    // In quiet mode, there should be minimal output
}

#[test]
fn test_config_file_override() {
    let env = TestEnvironment::new();
    
    // Create alternative config
    let alt_config = env.temp_dir.path().join("alt_config.toml");
    std::fs::write(
        &alt_config,
        r#"
[auth]
provider = "alternative"
"#,
    ).unwrap();
    
    // Test with alternative config
    env.command()
        .args(&["--config", alt_config.to_str().unwrap(), "config", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("alternative"));
}

#[test]
fn test_workspace_detection() {
    let env = TestEnvironment::new();
    
    // Create a git repository in temp dir
    std::process::Command::new("git")
        .args(&["init"])
        .current_dir(env.temp_dir.path())
        .output()
        .unwrap();
    
    // Test workspace detection
    env.command()
        .args(&["workspace", "info"])
        .current_dir(env.temp_dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Workspace detected"));
}

#[test]
fn test_error_handling() {
    let env = TestEnvironment::new();
    
    // Test reading non-existent file
    env.command()
        .args(&["file", "read", "/nonexistent/file.txt"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("error")));
}

#[test]
fn test_pipeline_commands() {
    let env = TestEnvironment::new();
    
    // Create input file
    let input_file = env.temp_dir.path().join("input.txt");
    std::fs::write(&input_file, "Original content").unwrap();
    
    // Test pipeline execution
    env.command()
        .args(&[
            "pipeline",
            "run",
            "--input", input_file.to_str().unwrap(),
            "--steps", "read,transform,write",
            "--output", env.temp_dir.path().join("output.txt").to_str().unwrap()
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Pipeline completed"));
}

#[test]
fn test_interactive_mode_simulation() {
    let env = TestEnvironment::new();
    
    // Test non-interactive flag
    env.command()
        .args(&["chat", "--non-interactive", "--message", "Test message"])
        .assert()
        .success();
}

#[test]
fn test_memory_commands() {
    let env = TestEnvironment::new();
    
    // Test memory store
    env.command()
        .args(&["memory", "store", "test-key", "test-value"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Stored"));
    
    // Test memory retrieve
    env.command()
        .args(&["memory", "get", "test-key"])
        .assert()
        .success()
        .stdout(predicate::str::contains("test-value"));
}

#[test]
fn test_plugin_commands() {
    let env = TestEnvironment::new();
    
    // Test plugin list
    env.command()
        .args(&["plugin", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("plugins").or(predicate::str::contains("No plugins")));
}

#[test]
fn test_logging_levels() {
    let env = TestEnvironment::new();
    
    // Test different log levels
    for level in &["error", "warn", "info", "debug", "trace"] {
        env.command()
            .args(&["--log-level", level, "auth", "status"])
            .assert()
            .success();
    }
}

#[test]
fn test_completion_generation() {
    let env = TestEnvironment::new();
    
    // Test shell completion generation
    for shell in &["bash", "zsh", "fish", "powershell"] {
        env.command()
            .args(&["completion", shell])
            .assert()
            .success()
            .stdout(predicate::str::contains("complete").or(predicate::str::contains("_code_mesh")));
    }
}