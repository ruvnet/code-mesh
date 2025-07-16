//! Serve command implementation - API server functionality

use crate::cmd::{CliError, UI, Config};
use crate::cmd::Result as CliResult;
use axum::{
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tokio::net::TcpListener;

/// Execute the serve command
pub async fn execute(host: &str, port: u16) -> CliResult<()> {
    let mut ui = UI::new();
    
    ui.print_logo()?;
    ui.info("Starting Code Mesh API server")?;
    
    // Validate inputs
    crate::cmd::utils::Validator::validate_port(port)?;
    
    // Load configuration
    let _config = Config::load()?;
    
    // Check authentication
    let auth_manager = code_mesh_core::auth::AuthManager::new().await?;
    let credentials = auth_manager.list_credentials().await?;
    
    if credentials.is_empty() {
        ui.warning("No authentication providers configured")?;
        ui.info("Some API endpoints may not work without authentication")?;
        ui.info("Run 'code-mesh auth login' to set up authentication")?;
        ui.println("")?;
    } else {
        ui.success(&format!("Found {} configured providers", credentials.len()))?;
    }
    
    // Create the application router
    let app = create_router().await?;
    
    // Create server address
    let addr = format!("{}:{}", host, port);
    
    ui.info(&format!("Server listening on http://{}", addr))?;
    ui.info("Press Ctrl+C to stop the server")?;
    ui.println("")?;
    
    // Start server
    let listener = TcpListener::bind(&addr).await
        .map_err(|e| CliError::Server(format!("Failed to bind to {}: {}", addr, e)))?;
    
    // Handle graceful shutdown
    let shutdown = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install CTRL+C signal handler");
        eprintln!("Received shutdown signal, stopping server...");
    };
    
    // Run server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .map_err(|e| CliError::Server(format!("Server error: {}", e)))?;
    
    ui.success("Server stopped gracefully")?;
    Ok(())
}

/// Create the API router
async fn create_router() -> CliResult<Router> {
    let api_router = Router::new()
        .route("/health", get(health_check))
        .route("/chat", post(chat_endpoint))
        .route("/sessions", get(list_sessions))
        .route("/sessions", post(create_session))
        .route("/models", get(list_models))
        .route("/providers", get(list_providers));
    
    let app = Router::new()
        .nest("/api/v1", api_router)
        .route("/", get(root))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );
    
    Ok(app)
}

/// Root endpoint
async fn root() -> Json<ApiResponse<ServerInfo>> {
    Json(ApiResponse::success(ServerInfo {
        name: "Code Mesh API".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "AI-powered coding assistant API".to_string(),
    }))
}

/// Health check endpoint
async fn health_check() -> Json<ApiResponse<HealthStatus>> {
    Json(ApiResponse::success(HealthStatus {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        uptime: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

/// Chat endpoint - placeholder implementation
async fn chat_endpoint(
    Json(request): Json<ChatRequest>,
) -> Result<Json<ApiResponse<ChatResponse>>, StatusCode> {
    // Validate request
    if request.message.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    
    // TODO: Implement actual chat logic using code-mesh-core
    let response = ChatResponse {
        id: crate::cmd::utils::Utils::generate_id(),
        message: format!("Echo: {}", request.message),
        model: request.model.unwrap_or_else(|| "claude-3-sonnet-20240229".to_string()),
        session_id: request.session_id.unwrap_or_else(|| crate::cmd::utils::Utils::generate_id()),
        created_at: chrono::Utc::now(),
        tokens_used: 100, // Placeholder
    };
    
    Ok(Json(ApiResponse::success(response)))
}

/// List sessions endpoint - placeholder implementation
async fn list_sessions() -> Result<Json<ApiResponse<Vec<SessionInfo>>>, StatusCode> {
    // TODO: Implement actual session listing using code-mesh-core
    let sessions = vec![
        SessionInfo {
            id: "session-1".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            message_count: 5,
            model: Some("claude-3-sonnet-20240229".to_string()),
        },
    ];
    
    Ok(Json(ApiResponse::success(sessions)))
}

/// Create session endpoint - placeholder implementation
async fn create_session(
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<ApiResponse<SessionInfo>>, StatusCode> {
    // TODO: Implement actual session creation using code-mesh-core
    let session = SessionInfo {
        id: crate::cmd::utils::Utils::generate_id(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        message_count: 0,
        model: request.model,
    };
    
    Ok(Json(ApiResponse::success(session)))
}

/// List models endpoint - placeholder implementation
async fn list_models() -> Result<Json<ApiResponse<Vec<ModelInfo>>>, StatusCode> {
    // TODO: Implement actual model listing using code-mesh-core
    let models = vec![
        ModelInfo {
            id: "claude-3-sonnet-20240229".to_string(),
            provider: "anthropic".to_string(),
            name: "Claude 3 Sonnet".to_string(),
            description: "Anthropic's Claude 3 Sonnet model".to_string(),
            context_length: 200000,
            supports_streaming: true,
        },
        ModelInfo {
            id: "gpt-4".to_string(),
            provider: "openai".to_string(),
            name: "GPT-4".to_string(),
            description: "OpenAI's GPT-4 model".to_string(),
            context_length: 8192,
            supports_streaming: true,
        },
    ];
    
    Ok(Json(ApiResponse::success(models)))
}

/// List providers endpoint - placeholder implementation
async fn list_providers() -> Result<Json<ApiResponse<Vec<ProviderInfo>>>, StatusCode> {
    // TODO: Implement actual provider listing using code-mesh-core
    let providers = vec![
        ProviderInfo {
            id: "anthropic".to_string(),
            name: "Anthropic".to_string(),
            description: "Anthropic's Claude models".to_string(),
            configured: true,
            models: vec!["claude-3-sonnet-20240229".to_string()],
        },
        ProviderInfo {
            id: "openai".to_string(),
            name: "OpenAI".to_string(),
            description: "OpenAI's GPT models".to_string(),
            configured: false,
            models: vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
        },
    ];
    
    Ok(Json(ApiResponse::success(providers)))
}

// API Types

#[derive(Debug, Serialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: chrono::Utc::now(),
        }
    }
    
    fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
            timestamp: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ServerInfo {
    name: String,
    version: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct HealthStatus {
    status: String,
    timestamp: chrono::DateTime<chrono::Utc>,
    uptime: u64,
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
    model: Option<String>,
    session_id: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ChatResponse {
    id: String,
    message: String,
    model: String,
    session_id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    tokens_used: u32,
}

#[derive(Debug, Deserialize)]
struct CreateSessionRequest {
    model: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Serialize)]
struct SessionInfo {
    id: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    message_count: u32,
    model: Option<String>,
}

#[derive(Debug, Serialize)]
struct ModelInfo {
    id: String,
    provider: String,
    name: String,
    description: String,
    context_length: u32,
    supports_streaming: bool,
}

#[derive(Debug, Serialize)]
struct ProviderInfo {
    id: String,
    name: String,
    description: String,
    configured: bool,
    models: Vec<String>,
}

