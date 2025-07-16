//! Tool system unit tests

use code_mesh_core::tool::*;
use proptest::prelude::*;
use rstest::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tempfile::TempDir;

mod common;
use common::{mocks::*, fixtures::*, *};

#[tokio::test]
async fn test_counting_tool() {
    let tool = CountingTool::new("test-tool");
    
    assert_eq!(tool.name(), "test-tool");
    assert_eq!(tool.description(), "Mock tool for testing");
    assert_eq!(tool.call_count(), 0);

    // Execute tool multiple times
    for i in 1..=5 {
        let result = tool.execute(json!({})).await.unwrap();
        assert_eq!(result.output, "Mock tool output");
        assert_eq!(tool.call_count(), i);
    }
}

#[tokio::test]
async fn test_counting_tool_with_custom_response() {
    let custom_response = ToolResult {
        output: "Custom output".to_string(),
        metadata: json!({
            "custom": true,
            "value": 42
        }),
    };

    let tool = CountingTool::new("custom-tool")
        .with_response(custom_response.clone());

    let result = tool.execute(json!({})).await.unwrap();
    assert_eq!(result.output, "Custom output");
    assert_eq!(result.metadata["custom"], true);
    assert_eq!(result.metadata["value"], 42);
}

#[tokio::test]
async fn test_tool_parameters_schema() {
    let tool = CountingTool::new("schema-tool");
    let parameters = tool.parameters();

    assert_eq!(parameters["type"], "object");
    assert!(parameters["properties"].is_object());
    assert!(parameters["required"].is_array());
}

#[test]
fn test_tool_result_creation() {
    let result = ToolResult {
        output: "Test output".to_string(),
        metadata: json!({
            "execution_time": 123,
            "status": "success"
        }),
    };

    assert_eq!(result.output, "Test output");
    assert_eq!(result.metadata["execution_time"], 123);
    assert_eq!(result.metadata["status"], "success");
}

#[test]
fn test_tool_result_serialization() {
    let result = ToolResult {
        output: "Serialization test".to_string(),
        metadata: json!({
            "version": "1.0",
            "tags": ["test", "serialization"]
        }),
    };

    let serialized = serde_json::to_string(&result).unwrap();
    let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();

    assert_eq!(deserialized.output, result.output);
    assert_eq!(deserialized.metadata, result.metadata);
}

#[tokio::test]
async fn test_concurrent_tool_execution() {
    let tool = Arc::new(CountingTool::new("concurrent-tool"));
    
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let tool = Arc::clone(&tool);
            tokio::spawn(async move {
                tool.execute(json!({"index": i})).await
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All executions should succeed
    for result in results {
        assert!(result.unwrap().is_ok());
    }

    // Should have been called 10 times
    assert_eq!(tool.call_count(), 10);
}

#[rstest]
#[case("read", json!({"file_path": "/test/file.txt"}))]
#[case("write", json!({"file_path": "/test/output.txt", "content": "Hello"}))]
#[case("edit", json!({"file_path": "/test/file.txt", "old_string": "old", "new_string": "new"}))]
#[case("bash", json!({"command": "ls -la"}))]
fn test_tool_parameter_validation(#[case] tool_name: &str, #[case] params: Value) {
    // Test that tool parameters are properly structured
    assert!(params.is_object());
    
    match tool_name {
        "read" => {
            assert!(params["file_path"].is_string());
        }
        "write" => {
            assert!(params["file_path"].is_string());
            assert!(params["content"].is_string());
        }
        "edit" => {
            assert!(params["file_path"].is_string());
            assert!(params["old_string"].is_string());
            assert!(params["new_string"].is_string());
        }
        "bash" => {
            assert!(params["command"].is_string());
        }
        _ => {}
    }
}

// Property-based tests
proptest! {
    #[test]
    fn test_tool_result_properties(
        output in ".*",
        metadata in prop::collection::btree_map(
            "[a-zA-Z_][a-zA-Z0-9_]*",
            prop_oneof![
                any::<bool>().prop_map(Value::Bool),
                any::<i64>().prop_map(|i| Value::Number(i.into())),
                ".*".prop_map(Value::String)
            ],
            0..10
        )
    ) {
        let metadata_value = Value::Object(metadata.into_iter().collect());
        let result = ToolResult {
            output: output.clone(),
            metadata: metadata_value.clone(),
        };

        prop_assert_eq!(result.output, output);
        prop_assert_eq!(result.metadata, metadata_value);

        // Test serialization roundtrip
        let serialized = serde_json::to_string(&result).unwrap();
        let deserialized: ToolResult = serde_json::from_str(&serialized).unwrap();
        prop_assert_eq!(deserialized.output, output);
        prop_assert_eq!(deserialized.metadata, metadata_value);
    }

    #[test]
    fn test_counting_tool_properties(
        name in "[a-zA-Z][a-zA-Z0-9_-]*",
        execution_count in 0usize..100
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tool = CountingTool::new(&name);
            
            prop_assert_eq!(tool.name(), name);
            prop_assert_eq!(tool.call_count(), 0);

            for _ in 0..execution_count {
                let _ = tool.execute(json!({})).await.unwrap();
            }

            prop_assert_eq!(tool.call_count(), execution_count);
        });
    }
}

#[test]
fn test_tool_fixtures() {
    let read_params = ToolFixtures::read_file_params();
    assert_eq!(read_params["file_path"], "/path/to/file.txt");

    let write_params = ToolFixtures::write_file_params();
    assert_eq!(write_params["file_path"], "/path/to/output.txt");
    assert_eq!(write_params["content"], "Hello, World!");

    let edit_params = ToolFixtures::edit_file_params();
    assert_eq!(edit_params["file_path"], "/path/to/file.txt");
    assert_eq!(edit_params["old_string"], "old content");
    assert_eq!(edit_params["new_string"], "new content");

    let bash_params = ToolFixtures::bash_command_params();
    assert_eq!(bash_params["command"], "ls -la");
    assert_eq!(bash_params["working_directory"], "/tmp");

    let search_params = ToolFixtures::web_search_params();
    assert_eq!(search_params["query"], "rust programming language");
    assert_eq!(search_params["num_results"], 10);

    let glob_params = ToolFixtures::glob_pattern_params();
    assert_eq!(glob_params["pattern"], "**/*.rs");
}

// Mock tool implementations for specific tools
#[derive(Clone)]
struct MockReadTool {
    file_contents: Arc<parking_lot::RwLock<std::collections::HashMap<String, String>>>,
}

impl MockReadTool {
    fn new() -> Self {
        Self {
            file_contents: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    fn add_file(&self, path: String, content: String) {
        self.file_contents.write().insert(path, content);
    }
}

#[async_trait::async_trait]
impl Tool for MockReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "Read file contents"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, parameters: Value) -> code_mesh_core::CodeMeshResult<ToolResult> {
        let file_path = parameters["file_path"]
            .as_str()
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::InvalidInput("Missing file_path".to_string()))?;

        let contents = self.file_contents.read();
        let content = contents
            .get(file_path)
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::FileNotFound(file_path.to_string()))?;

        Ok(ToolResult {
            output: content.clone(),
            metadata: json!({
                "file_path": file_path,
                "size": content.len(),
                "type": "file_read"
            }),
        })
    }
}

#[tokio::test]
async fn test_mock_read_tool() {
    let tool = MockReadTool::new();
    tool.add_file("/test/file.txt".to_string(), "Hello, World!".to_string());

    let result = tool.execute(json!({
        "file_path": "/test/file.txt"
    })).await.unwrap();

    assert_eq!(result.output, "Hello, World!");
    assert_eq!(result.metadata["file_path"], "/test/file.txt");
    assert_eq!(result.metadata["size"], 13);
    assert_eq!(result.metadata["type"], "file_read");
}

#[tokio::test]
async fn test_mock_read_tool_file_not_found() {
    let tool = MockReadTool::new();

    let result = tool.execute(json!({
        "file_path": "/nonexistent/file.txt"
    })).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        code_mesh_core::error::CodeMeshError::FileNotFound(path) => {
            assert_eq!(path, "/nonexistent/file.txt");
        }
        _ => panic!("Expected FileNotFound error"),
    }
}

#[tokio::test]
async fn test_mock_read_tool_invalid_parameters() {
    let tool = MockReadTool::new();

    let result = tool.execute(json!({
        "invalid_param": "value"
    })).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        code_mesh_core::error::CodeMeshError::InvalidInput(_) => {},
        _ => panic!("Expected InvalidInput error"),
    }
}

// Write tool mock
#[derive(Clone)]
struct MockWriteTool {
    written_files: Arc<parking_lot::RwLock<std::collections::HashMap<String, String>>>,
}

impl MockWriteTool {
    fn new() -> Self {
        Self {
            written_files: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    fn get_written_content(&self, path: &str) -> Option<String> {
        self.written_files.read().get(path).cloned()
    }
}

#[async_trait::async_trait]
impl Tool for MockWriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "Write content to file"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn execute(&self, parameters: Value) -> code_mesh_core::CodeMeshResult<ToolResult> {
        let file_path = parameters["file_path"]
            .as_str()
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::InvalidInput("Missing file_path".to_string()))?;
        
        let content = parameters["content"]
            .as_str()
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::InvalidInput("Missing content".to_string()))?;

        self.written_files.write().insert(file_path.to_string(), content.to_string());

        Ok(ToolResult {
            output: format!("Successfully wrote {} bytes to {}", content.len(), file_path),
            metadata: json!({
                "file_path": file_path,
                "bytes_written": content.len(),
                "type": "file_write"
            }),
        })
    }
}

#[tokio::test]
async fn test_mock_write_tool() {
    let tool = MockWriteTool::new();

    let result = tool.execute(json!({
        "file_path": "/test/output.txt",
        "content": "Hello, World!"
    })).await.unwrap();

    assert!(result.output.contains("Successfully wrote"));
    assert!(result.output.contains("13 bytes"));
    assert_eq!(result.metadata["file_path"], "/test/output.txt");
    assert_eq!(result.metadata["bytes_written"], 13);

    // Verify content was written
    let written_content = tool.get_written_content("/test/output.txt").unwrap();
    assert_eq!(written_content, "Hello, World!");
}

// Tool registry tests
#[derive(Clone)]
struct MockToolRegistry {
    tools: Arc<parking_lot::RwLock<std::collections::HashMap<String, Arc<dyn Tool + Send + Sync>>>>,
}

impl MockToolRegistry {
    fn new() -> Self {
        Self {
            tools: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }

    fn register_tool(&self, tool: Arc<dyn Tool + Send + Sync>) {
        self.tools.write().insert(tool.name().to_string(), tool);
    }

    fn get_tool(&self, name: &str) -> Option<Arc<dyn Tool + Send + Sync>> {
        self.tools.read().get(name).cloned()
    }

    fn list_tools(&self) -> Vec<String> {
        self.tools.read().keys().cloned().collect()
    }
}

#[tokio::test]
async fn test_tool_registry() {
    let registry = MockToolRegistry::new();
    
    let read_tool = Arc::new(MockReadTool::new());
    let write_tool = Arc::new(MockWriteTool::new());
    let counting_tool = Arc::new(CountingTool::new("counter"));

    registry.register_tool(read_tool.clone());
    registry.register_tool(write_tool.clone());
    registry.register_tool(counting_tool.clone());

    let tools = registry.list_tools();
    assert_eq!(tools.len(), 3);
    assert!(tools.contains(&"read".to_string()));
    assert!(tools.contains(&"write".to_string()));
    assert!(tools.contains(&"counter".to_string()));

    let retrieved_read = registry.get_tool("read").unwrap();
    assert_eq!(retrieved_read.name(), "read");

    let nonexistent = registry.get_tool("nonexistent");
    assert!(nonexistent.is_none());
}

#[test]
fn test_tool_metadata_validation() {
    let valid_metadata = json!({
        "execution_time_ms": 123,
        "status": "success",
        "version": "1.0.0",
        "cached": false
    });

    // Test that metadata is a proper JSON object
    assert!(valid_metadata.is_object());
    
    let metadata_obj = valid_metadata.as_object().unwrap();
    assert!(metadata_obj.contains_key("execution_time_ms"));
    assert!(metadata_obj.contains_key("status"));
    assert!(metadata_obj.contains_key("version"));
    assert!(metadata_obj.contains_key("cached"));
}

#[tokio::test]
async fn test_tool_error_handling() {
    let tool = CountingTool::new("error-tool");
    
    // Test with invalid JSON parameters
    let invalid_params = json!("not an object");
    let result = tool.execute(invalid_params).await;
    // CountingTool doesn't validate parameters, so this should still work
    assert!(result.is_ok());

    // Test with null parameters
    let null_params = Value::Null;
    let result = tool.execute(null_params).await;
    assert!(result.is_ok());
}

#[test]
fn test_tool_performance_metadata() {
    let start = std::time::Instant::now();
    
    // Simulate some work
    std::thread::sleep(std::time::Duration::from_millis(1));
    
    let duration = start.elapsed();
    let metadata = json!({
        "execution_time_ms": duration.as_millis(),
        "performance_tier": if duration.as_millis() < 100 { "fast" } else { "slow" }
    });

    assert!(metadata["execution_time_ms"].as_u64().unwrap() >= 1);
    assert!(["fast", "slow"].contains(&metadata["performance_tier"].as_str().unwrap()));
}