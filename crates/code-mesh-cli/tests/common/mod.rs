//! Common test utilities for CLI testing

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

pub mod fixtures;
pub mod helpers;

pub use fixtures::*;
pub use helpers::*;

/// Helper to create a test command
pub fn test_command() -> Command {
    Command::cargo_bin("code-mesh").unwrap()
}

/// Helper to create a temporary directory
pub fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Helper to create test configuration
pub fn create_test_config(temp_dir: &TempDir) -> std::path::PathBuf {
    let config_path = temp_dir.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"
[auth]
provider = "mock"
api_key = "test-key"

[llm]
model = "mock-model"
max_tokens = 1000

[storage]
type = "file"
path = "test-data"
"#,
    )
    .expect("Failed to write test config");
    config_path
}

/// Test environment setup
pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub config_path: std::path::PathBuf,
}

impl TestEnvironment {
    pub fn new() -> Self {
        let temp_dir = temp_dir();
        let config_path = create_test_config(&temp_dir);
        Self { temp_dir, config_path }
    }

    pub fn command(&self) -> Command {
        let mut cmd = test_command();
        cmd.env("CODE_MESH_CONFIG", &self.config_path);
        cmd.env("CODE_MESH_DATA_DIR", self.temp_dir.path());
        cmd
    }
}

/// Assertion helpers
pub mod assertions {
    use super::*;

    pub fn assert_success_output(output: &std::process::Output, expected: &str) {
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains(expected));
    }

    pub fn assert_error_output(output: &std::process::Output, expected: &str) {
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected));
    }

    pub fn assert_json_output(output: &std::process::Output) -> serde_json::Value {
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str(&stdout).expect("Output should be valid JSON")
    }
}