# Web Tools Implementation

This module implements comprehensive web functionality for Code Mesh, including HTTP client abstraction, web content fetching, search capabilities, and task management tools.

## Architecture

### HTTP Client Abstraction (`http.rs`)

The HTTP client provides a unified interface that works across both native and WASM environments:

#### Key Features:
- **Cross-Platform**: Works on native (using reqwest) and WASM (using web-sys fetch)
- **Request/Response Interceptors**: Pluggable middleware system for modifying requests and responses
- **Rate Limiting**: Built-in rate limiting to respect server limits
- **Cookie Management**: Automatic cookie handling with domain/path matching
- **User-Agent Management**: Configurable user agent strings
- **Security**: SSRF protection and URL sanitization
- **SSL/TLS**: Configurable SSL verification
- **Proxy Support**: HTTP proxy support for corporate environments

#### Example Usage:
```rust
use code_mesh_core::tool::http::HttpClientBuilder;

let client = HttpClientBuilder::new()
    .rate_limit(2.0)  // 2 requests per second
    .timeout(Duration::from_secs(30))
    .verify_ssl(true)
    .build()?;

let request = HttpRequest::get(url)
    .header("Accept".to_string(), "application/json".to_string());

let response = client.execute(request).await?;
```

### Web Fetch Tool (`WebFetchTool`)

Retrieves and processes web content with multiple output formats:

#### Features:
- **Multiple Formats**: HTML, text, markdown output
- **HTML Processing**: Clean text extraction and markdown conversion using scraper and html2md
- **Security**: URL validation and SSRF protection
- **Size Limits**: 5MB response size limit
- **Rate Limiting**: Built-in request throttling

#### Tool Parameters:
- `url`: Target URL (HTTP/HTTPS only)
- `format`: Output format ("text", "markdown", "html")
- `timeout`: Request timeout in seconds (max 120)

### Web Search Tool (`WebSearchTool`)

Multi-provider web search with result processing:

#### Features:
- **DuckDuckGo Integration**: Uses DuckDuckGo Instant Answer API
- **Result Processing**: Structured search results with ranking
- **Extensible**: Framework for adding additional search providers
- **Rate Limiting**: Separate rate limits for search requests

#### Tool Parameters:
- `query`: Search query string
- `max_results`: Maximum results to return (1-20)
- `language`: Result language preference
- `provider`: Search provider ("duckduckgo", "auto")

### Todo Management Tool (`TodoTool`)

Comprehensive task management with dependency tracking:

#### Features:
- **Task Lifecycle**: Full status tracking (pending, in_progress, completed, cancelled, blocked)
- **Dependency Management**: Task dependencies with cycle detection
- **Progress Tracking**: Percentage completion and duration tracking
- **Analytics**: Completion statistics and performance metrics
- **Multiple Export Formats**: JSON, Markdown, CSV export
- **Session Persistence**: Per-session task storage

#### Task Actions:
- `list`: Display all tasks with status grouping
- `add`: Create new task with priority
- `update`: Modify task status, priority, content, or progress
- `remove`: Delete task
- `add_dependency`: Add task dependency with cycle detection
- `add_note`: Add notes to tasks
- `stats`: View completion statistics
- `export`: Export tasks in various formats

## Security Considerations

### SSRF Protection
- URL validation to prevent access to internal networks
- Blocked schemes (file://, ftp://, etc.)
- Private IP range blocking (127.0.0.1, 192.168.x.x, 10.x.x.x, etc.)
- Localhost and .local domain blocking

### Request Sanitization
- User-Agent header validation
- Content-Type verification
- Response size limits
- Timeout enforcement

### Cookie Security
- Domain and path matching
- Secure cookie handling
- Session isolation

## WASM Compatibility

All web tools are designed to work in WASM environments with appropriate fallbacks:

### Native Implementation
- Uses `reqwest` for HTTP requests
- Full feature set including connection pooling
- Proxy support for corporate environments

### WASM Implementation
- Uses `web-sys` fetch API
- CORS handling for browser security
- Limited to browser capabilities

## Performance Optimizations

### Connection Management
- HTTP keep-alive support
- Connection pooling (native)
- Request pipelining where supported

### Caching
- Response caching with TTL
- Search result caching
- Memory-efficient storage

### Rate Limiting
- Per-host rate limiting
- Configurable limits per tool
- Burst capacity handling

## Error Handling

### Network Errors
- Timeout handling
- Connection failure recovery
- DNS resolution errors
- SSL/TLS errors

### HTTP Errors
- Status code validation
- Content-Type checking
- Response size validation
- Redirect handling

### Parsing Errors
- HTML parsing errors
- JSON deserialization errors
- URL parsing errors
- Character encoding issues

## Testing

The module includes comprehensive tests for:
- HTTP client functionality
- Web content fetching
- Search result processing
- Task management operations
- Dependency cycle detection
- Security validations

## Dependencies

### Core Dependencies
- `reqwest`: HTTP client for native environments
- `tokio`: Async runtime
- `serde`: Serialization framework
- `chrono`: Date/time handling
- `uuid`: Unique identifier generation

### Web-Specific Dependencies
- `scraper`: HTML parsing and text extraction
- `html2md`: HTML to Markdown conversion
- `cookie`: Cookie parsing and management
- `url`: URL parsing and validation
- `urlencoding`: URL encoding utilities

### WASM Dependencies
- `web-sys`: Web API bindings
- `wasm-bindgen`: JavaScript interop
- `js-sys`: JavaScript standard library bindings

## Future Enhancements

### Planned Features
- Additional search providers (Google, Bing)
- Semantic search capabilities
- Advanced result filtering and ranking
- Persistent task storage
- Real-time collaboration features
- WebSocket support for live updates

### Performance Improvements
- HTTP/2 support
- Request batching
- Advanced caching strategies
- Memory optimization
- Connection multiplexing

### Security Enhancements
- Certificate pinning
- Request signing
- Enhanced SSRF protection
- Content security policies
- Rate limiting improvements