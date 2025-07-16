//! End-to-end integration tests

use code_mesh_core::{
    auth::*,
    llm::*,
    session::*,
    storage::*,
    tool::*,
    CodeMeshResult,
};
use std::sync::Arc;
use tempfile::TempDir;

use super::common::{mocks::*, fixtures::*, *};

/// Complete workflow test: auth -> session -> tool execution -> storage
#[tokio::test]
async fn test_complete_workflow() {
    let temp_dir = temp_dir();
    
    // Setup components
    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());
    let storage = InMemoryStorage::new();
    let llm_provider = MockLLMProvider::with_responses(vec![
        "I'll help you with that task.".to_string(),
        "Here's the solution...".to_string(),
    ]);
    let session_storage = InMemorySessionStorage::new();
    
    // 1. Authentication workflow
    let api_key = "sk-test-key-12345";
    auth_storage.save_token(api_key.to_string()).await.unwrap();
    let saved_token = auth_storage.load_token().await.unwrap();
    assert_eq!(saved_token, Some(api_key.to_string()));
    
    // 2. Create and manage session
    let mut session = Session::new("e2e-test-session".to_string(), "test-user".to_string());
    
    // Add system message
    session.add_message(ChatMessage {
        role: ChatRole::System,
        content: "You are a helpful coding assistant.".to_string(),
    });
    
    // User request
    session.add_message(ChatMessage {
        role: ChatRole::User,
        content: "Help me write a function to read a file.".to_string(),
    });
    
    // Save session
    session_storage.save_session(session.clone()).await.unwrap();
    
    // 3. LLM interaction
    let messages = session.get_conversation_context();
    let completion = llm_provider.chat_completion(messages).await.unwrap();
    
    // Add assistant response
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: completion.content.clone(),
    });
    
    // 4. Tool execution simulation
    let read_tool = MockReadTool::new();
    read_tool.add_file("/test/example.txt".to_string(), "Hello, World!".to_string());
    
    let tool_result = read_tool.execute(serde_json::json!({
        "file_path": "/test/example.txt"
    })).await.unwrap();
    
    assert_eq!(tool_result.output, "Hello, World!");
    
    // 5. Store conversation and results
    storage.save("session_result".to_string(), serde_json::json!({
        "session_id": session.id,
        "completion": completion,
        "tool_result": tool_result,
        "timestamp": chrono::Utc::now()
    })).await.unwrap();
    
    // Update session with tool result
    session.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: format!("I used the read tool and got: {}", tool_result.output),
    });
    
    // Save updated session
    session_storage.save_session(session.clone()).await.unwrap();
    
    // 6. Verify complete workflow
    let final_session = session_storage
        .load_session(session.id.clone())
        .await
        .unwrap()
        .unwrap();
    
    assert_eq!(final_session.message_count(), 4); // system + user + assistant + tool result
    assert_eq!(final_session.id, "e2e-test-session");
    
    let stored_result = storage.load("session_result".to_string()).await.unwrap().unwrap();
    assert_eq!(stored_result["session_id"], session.id);
    assert_eq!(stored_result["tool_result"]["output"], "Hello, World!");
}

/// Multi-session conversation workflow
#[tokio::test]
async fn test_multi_session_workflow() {
    let storage = InMemoryStorage::new();
    let session_storage = InMemorySessionStorage::new();
    let llm_provider = MockLLMProvider::with_responses(vec![
        "Hello! How can I help?".to_string(),
        "Sure, I can help with that.".to_string(),
        "Let me continue from where we left off.".to_string(),
    ]);
    
    // Create first session
    let mut session1 = Session::new("session-1".to_string(), "user-123".to_string());
    session1.add_message(ChatMessage {
        role: ChatRole::User,
        content: "Hello, I need help with coding.".to_string(),
    });
    
    let response1 = llm_provider.chat_completion(session1.get_conversation_context()).await.unwrap();
    session1.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: response1.content,
    });
    
    session_storage.save_session(session1.clone()).await.unwrap();
    
    // Create second session for same user
    let mut session2 = Session::new("session-2".to_string(), "user-123".to_string());
    session2.add_message(ChatMessage {
        role: ChatRole::User,
        content: "I have a follow-up question.".to_string(),
    });
    
    let response2 = llm_provider.chat_completion(session2.get_conversation_context()).await.unwrap();
    session2.add_message(ChatMessage {
        role: ChatRole::Assistant,
        content: response2.content,
    });
    
    session_storage.save_session(session2.clone()).await.unwrap();
    
    // Verify both sessions exist
    let sessions = session_storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(sessions.contains(&"session-1".to_string()));
    assert!(sessions.contains(&"session-2".to_string()));
    
    // Store cross-session context
    storage.save("user_context".to_string(), serde_json::json!({
        "user_id": "user-123",
        "sessions": ["session-1", "session-2"],
        "total_messages": session1.message_count() + session2.message_count()
    })).await.unwrap();
    
    let context = storage.load("user_context".to_string()).await.unwrap().unwrap();
    assert_eq!(context["user_id"], "user-123");
    assert_eq!(context["total_messages"], 4);
}

/// Error handling and recovery workflow
#[tokio::test]
async fn test_error_handling_workflow() {
    let temp_dir = temp_dir();
    let auth_storage = AuthStorage::new(temp_dir.path().to_path_buf());
    let storage = InMemoryStorage::new();
    let session_storage = InMemorySessionStorage::new();
    
    // Test auth failure recovery
    let invalid_token_result = auth_storage.load_token().await;
    assert!(invalid_token_result.is_ok()); // Should return None, not error
    assert_eq!(invalid_token_result.unwrap(), None);
    
    // Create session and try to save it
    let session = Session::new("error-test".to_string(), "user".to_string());
    session_storage.save_session(session.clone()).await.unwrap();
    
    // Test graceful handling of missing sessions
    let missing_session = session_storage.load_session("nonexistent".to_string()).await.unwrap();
    assert!(missing_session.is_none());
    
    // Test storage error handling
    let missing_data = storage.load("nonexistent_key".to_string()).await.unwrap();
    assert!(missing_data.is_none());
    
    // Test tool error handling
    let read_tool = MockReadTool::new();
    let error_result = read_tool.execute(serde_json::json!({
        "file_path": "/nonexistent/file.txt"
    })).await;
    
    assert!(error_result.is_err());
}

/// Performance and concurrency workflow
#[tokio::test]
async fn test_concurrent_workflow() {
    let storage = Arc::new(InMemoryStorage::new());
    let session_storage = Arc::new(InMemorySessionStorage::new());
    let llm_provider = Arc::new(MockLLMProvider::with_responses(vec![
        "Response 1".to_string(),
        "Response 2".to_string(),
        "Response 3".to_string(),
    ]));
    
    // Spawn multiple concurrent workflows
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let storage = Arc::clone(&storage);
            let session_storage = Arc::clone(&session_storage);
            let llm_provider = Arc::clone(&llm_provider);
            
            tokio::spawn(async move {
                // Create session
                let mut session = Session::new(
                    format!("concurrent-session-{}", i),
                    format!("user-{}", i)
                );
                
                // Add message
                session.add_message(ChatMessage {
                    role: ChatRole::User,
                    content: format!("Message from user {}", i),
                });
                
                // Get LLM response
                let response = llm_provider
                    .chat_completion(session.get_conversation_context())
                    .await
                    .unwrap();
                
                session.add_message(ChatMessage {
                    role: ChatRole::Assistant,
                    content: response.content.clone(),
                });
                
                // Save session and data
                session_storage.save_session(session.clone()).await.unwrap();
                storage.save(
                    format!("workflow-{}", i),
                    serde_json::json!({
                        "session_id": session.id,
                        "response": response.content,
                        "worker": i
                    })
                ).await.unwrap();
                
                i
            })
        })
        .collect();
    
    // Wait for all workflows to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // Verify all workflows succeeded
    for (i, result) in results.into_iter().enumerate() {
        assert_eq!(result.unwrap(), i);
    }
    
    // Verify all sessions were created
    let sessions = session_storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 10);
    
    // Verify all data was stored
    for i in 0..10 {
        let data = storage.load(format!("workflow-{}", i)).await.unwrap().unwrap();
        assert_eq!(data["worker"], i);
    }
}

/// Tool chain execution workflow
#[tokio::test]
async fn test_tool_chain_workflow() {
    let storage = InMemoryStorage::new();
    let read_tool = MockReadTool::new();
    let write_tool = MockWriteTool::new();
    
    // Setup initial file
    read_tool.add_file("/input.txt".to_string(), "Initial content".to_string());
    
    // Chain 1: Read file
    let read_result = read_tool.execute(serde_json::json!({
        "file_path": "/input.txt"
    })).await.unwrap();
    
    assert_eq!(read_result.output, "Initial content");
    
    // Store intermediate result
    storage.save("step1_result".to_string(), serde_json::json!({
        "output": read_result.output,
        "metadata": read_result.metadata
    })).await.unwrap();
    
    // Chain 2: Process content (simulate transformation)
    let processed_content = format!("Processed: {}", read_result.output);
    
    // Chain 3: Write processed content
    let write_result = write_tool.execute(serde_json::json!({
        "file_path": "/output.txt",
        "content": processed_content
    })).await.unwrap();
    
    assert!(write_result.output.contains("Successfully wrote"));
    
    // Verify content was written
    let written_content = write_tool.get_written_content("/output.txt").unwrap();
    assert_eq!(written_content, "Processed: Initial content");
    
    // Store final result
    storage.save("final_result".to_string(), serde_json::json!({
        "input_file": "/input.txt",
        "output_file": "/output.txt",
        "transformation": "prefix_processing",
        "success": true
    })).await.unwrap();
    
    let final_result = storage.load("final_result".to_string()).await.unwrap().unwrap();
    assert_eq!(final_result["success"], true);
    assert_eq!(final_result["transformation"], "prefix_processing");
}

/// Long conversation workflow with context management
#[tokio::test]
async fn test_long_conversation_workflow() {
    let session_storage = InMemorySessionStorage::new();
    let llm_provider = MockLLMProvider::with_responses(vec![
        "Let me help you with that.".to_string(),
        "Here's the next step.".to_string(),
        "Building on our previous discussion.".to_string(),
        "To summarize what we've covered.".to_string(),
    ]);
    
    let mut session = Session::new("long-conversation".to_string(), "user".to_string());
    
    // System message
    session.add_message(ChatMessage {
        role: ChatRole::System,
        content: "You are a helpful assistant that maintains context across a long conversation.".to_string(),
    });
    
    // Simulate a long conversation
    for i in 1..=10 {
        // User message
        session.add_message(ChatMessage {
            role: ChatRole::User,
            content: format!("This is question {}. Please help me understand the concept.", i),
        });
        
        // Get LLM response
        let response = llm_provider
            .chat_completion(session.get_conversation_context())
            .await
            .unwrap();
        
        session.add_message(ChatMessage {
            role: ChatRole::Assistant,
            content: response.content,
        });
        
        // Save session after each exchange
        session_storage.save_session(session.clone()).await.unwrap();
    }
    
    // Verify conversation length
    assert_eq!(session.message_count(), 21); // 1 system + 10 user + 10 assistant
    
    // Verify context is maintained
    let context = session.get_conversation_context();
    assert_eq!(context[0].role, ChatRole::System);
    assert_eq!(context[1].role, ChatRole::User);
    assert_eq!(context[2].role, ChatRole::Assistant);
    
    // Last message should be from assistant
    assert_eq!(session.last_message().unwrap().role, ChatRole::Assistant);
}

// Helper implementations for integration tests
#[derive(Clone)]
struct InMemorySessionStorage {
    sessions: Arc<parking_lot::RwLock<std::collections::HashMap<String, Session>>>,
}

impl InMemorySessionStorage {
    fn new() -> Self {
        Self {
            sessions: Arc::new(parking_lot::RwLock::new(std::collections::HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl SessionStorage for InMemorySessionStorage {
    async fn save_session(&self, session: Session) -> CodeMeshResult<()> {
        self.sessions.write().insert(session.id.clone(), session);
        Ok(())
    }

    async fn load_session(&self, session_id: String) -> CodeMeshResult<Option<Session>> {
        Ok(self.sessions.read().get(&session_id).cloned())
    }

    async fn list_sessions(&self) -> CodeMeshResult<Vec<String>> {
        Ok(self.sessions.read().keys().cloned().collect())
    }

    async fn delete_session(&self, session_id: String) -> CodeMeshResult<()> {
        self.sessions.write().remove(&session_id);
        Ok(())
    }
}

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

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"}
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, parameters: serde_json::Value) -> CodeMeshResult<ToolResult> {
        let file_path = parameters["file_path"]
            .as_str()
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::InvalidInput("Missing file_path".to_string()))?;

        let contents = self.file_contents.read();
        let content = contents
            .get(file_path)
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::FileNotFound(file_path.to_string()))?;

        Ok(ToolResult {
            output: content.clone(),
            metadata: serde_json::json!({
                "file_path": file_path,
                "size": content.len()
            }),
        })
    }
}

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

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string"},
                "content": {"type": "string"}
            },
            "required": ["file_path", "content"]
        })
    }

    async fn execute(&self, parameters: serde_json::Value) -> CodeMeshResult<ToolResult> {
        let file_path = parameters["file_path"]
            .as_str()
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::InvalidInput("Missing file_path".to_string()))?;
        
        let content = parameters["content"]
            .as_str()
            .ok_or_else(|| code_mesh_core::error::CodeMeshError::InvalidInput("Missing content".to_string()))?;

        self.written_files.write().insert(file_path.to_string(), content.to_string());

        Ok(ToolResult {
            output: format!("Successfully wrote {} bytes to {}", content.len(), file_path),
            metadata: serde_json::json!({
                "file_path": file_path,
                "bytes_written": content.len()
            }),
        })
    }
}