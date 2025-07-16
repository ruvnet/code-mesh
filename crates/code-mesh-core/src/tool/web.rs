//! Web tools implementation

use super::http::{HttpClient, HttpClientBuilder, HttpRequest, sanitize_url};
use super::{Tool, ToolContext, ToolError, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

/// Maximum response size (5MB)
const MAX_RESPONSE_SIZE: usize = 5 * 1024 * 1024;

/// Web fetch tool for retrieving content from URLs
pub struct WebFetchTool {
    client: Box<dyn HttpClient>,
}

impl WebFetchTool {
    pub fn new() -> Result<Self, ToolError> {
        let client = HttpClientBuilder::new()
            .rate_limit(2.0) // 2 requests per second
            .timeout(Duration::from_secs(30))
            .verify_ssl(true)
            .build()
            .map_err(|e| ToolError::Other(e.into()))?;
        
        Ok(Self { client })
    }
}

#[derive(Debug, Deserialize)]
struct WebFetchParams {
    url: String,
    format: Option<String>, // "text", "markdown", "html"
    timeout: Option<u64>,
}

#[async_trait]
impl Tool for WebFetchTool {
    fn id(&self) -> &str {
        "webfetch"
    }
    
    fn description(&self) -> &str {
        "Fetches content from a specified URL and processes it according to the specified format. Supports HTML text extraction and markdown conversion."
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "url": {
                    "type": "string",
                    "description": "The URL to fetch content from (HTTP/HTTPS only)"
                },
                "format": {
                    "type": "string",
                    "enum": ["text", "markdown", "html"],
                    "description": "The format to return the content in",
                    "default": "text"
                },
                "timeout": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 120,
                    "description": "Optional timeout in seconds (max 120)"
                }
            },
            "required": ["url"]
        })
    }
    
    async fn execute(&self, args: Value, _ctx: ToolContext) -> Result<ToolResult, ToolError> {
        let params: WebFetchParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        // Sanitize and validate URL for security
        let url = sanitize_url(&params.url)
            .map_err(|e| ToolError::PermissionDenied(e.to_string()))?;
        
        // Build request with timeout
        let timeout = Duration::from_secs(params.timeout.unwrap_or(30).min(120));
        let request = HttpRequest::get(url.clone())
            .timeout(timeout)
            .header("Accept".to_string(), "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8".to_string())
            .header("Accept-Language".to_string(), "en-US,en;q=0.9".to_string());
        
        let response = self.client.execute(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Request failed: {}", e)))?;
        
        if !response.is_success() {
            return Err(ToolError::ExecutionFailed(format!("Request failed with status: {}", response.status())));
        }
        
        // Check content length
        if response.body().len() > MAX_RESPONSE_SIZE {
            return Err(ToolError::ExecutionFailed("Response too large (exceeds 5MB limit)".to_string()));
        }
        
        let content_type = response.content_type()
            .cloned()
            .unwrap_or_else(|| "text/plain".to_string());
        
        let text = response.text()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to decode response: {}", e)))?;
        
        let format = params.format.as_deref().unwrap_or("text");
        
        let output = match format {
            "text" => {
                if content_type.contains("text/html") {
                    extract_text_from_html(&text)?
                } else {
                    text
                }
            },
            "markdown" => {
                if content_type.contains("text/html") {
                    convert_html_to_markdown(&text)?
                } else {
                    format!("```\n{}\n```", text)
                }
            },
            "html" => text,
            _ => return Err(ToolError::InvalidParameters("Invalid format specified".to_string())),
        };
        
        Ok(ToolResult {
            title: format!("{} ({})", params.url, content_type),
            output,
            metadata: json!({
                "url": params.url,
                "content_type": content_type,
                "size": response.body().len(),
                "format": format,
                "status": response.status()
            }),
        })
    }
}

/// Web search tool for searching the internet
pub struct WebSearchTool {
    client: Box<dyn HttpClient>,
}

impl WebSearchTool {
    pub fn new() -> Result<Self, ToolError> {
        let client = HttpClientBuilder::new()
            .rate_limit(1.0) // 1 request per second for search
            .timeout(Duration::from_secs(30))
            .verify_ssl(true)
            .build()
            .map_err(|e| ToolError::Other(e.into()))?;
        
        Ok(Self { client })
    }
    
    /// Search using DuckDuckGo Instant Answer API
    async fn search_duckduckgo(&self, query: &str, max_results: u32) -> Result<Vec<SearchResult>, ToolError> {
        let search_url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_html=1&skip_disambig=1",
            urlencoding::encode(query)
        );
        
        let url = Url::parse(&search_url)
            .map_err(|e| ToolError::ExecutionFailed(format!("Invalid search URL: {}", e)))?;
        
        let request = HttpRequest::get(url)
            .header("Accept".to_string(), "application/json".to_string());
        
        let response = self.client.execute(request).await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search request failed: {}", e)))?;
        
        if !response.is_success() {
            return Err(ToolError::ExecutionFailed(format!("Search failed with status: {}", response.status())));
        }
        
        let search_response: DuckDuckGoResponse = response.json()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to parse search response: {}", e)))?;
        
        let mut results = Vec::new();
        
        // Add instant answer if available
        if !search_response.answer.is_empty() {
            results.push(SearchResult {
                title: "Instant Answer".to_string(),
                url: search_response.answer_url.unwrap_or_else(|| "https://duckduckgo.com".to_string()),
                description: search_response.answer,
                rank: 1,
                source: "DuckDuckGo".to_string(),
            });
        }
        
        // Add abstract if available
        if !search_response.abstract_text.is_empty() {
            results.push(SearchResult {
                title: search_response.heading.unwrap_or_else(|| "Summary".to_string()),
                url: search_response.abstract_url.unwrap_or_else(|| "https://duckduckgo.com".to_string()),
                description: search_response.abstract_text,
                rank: results.len() as u32 + 1,
                source: "DuckDuckGo".to_string(),
            });
        }
        
        // Add related topics
        for (i, topic) in search_response.related_topics.iter().take(max_results as usize).enumerate() {
            if !topic.text.is_empty() {
                results.push(SearchResult {
                    title: format!("Related: {}", topic.first_url.split('/').last().unwrap_or("Topic")),
                    url: topic.first_url.clone(),
                    description: topic.text.clone(),
                    rank: results.len() as u32 + 1,
                    source: "DuckDuckGo".to_string(),
                });
            }
        }
        
        Ok(results.into_iter().take(max_results as usize).collect())
    }
}

#[derive(Debug, Deserialize)]
struct WebSearchParams {
    query: String,
    max_results: Option<u32>,
    language: Option<String>,
    provider: Option<String>, // "duckduckgo", "bing", "google"
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SearchResult {
    title: String,
    url: String,
    description: String,
    rank: u32,
    source: String,
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "Answer")]
    answer: String,
    #[serde(rename = "AnswerURL")]
    answer_url: Option<String>,
    #[serde(rename = "Abstract")]
    abstract_text: String,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "Heading")]
    heading: Option<String>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Vec<RelatedTopic>,
}

#[derive(Debug, Deserialize)]
struct RelatedTopic {
    #[serde(rename = "Text")]
    text: String,
    #[serde(rename = "FirstURL")]
    first_url: String,
}

#[async_trait]
impl Tool for WebSearchTool {
    fn id(&self) -> &str {
        "websearch"
    }
    
    fn description(&self) -> &str {
        "Searches the web using various search providers and returns formatted search results"
    }
    
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The search query"
                },
                "max_results": {
                    "type": "number",
                    "minimum": 1,
                    "maximum": 20,
                    "default": 10,
                    "description": "Maximum number of results to return"
                },
                "language": {
                    "type": "string",
                    "default": "en",
                    "description": "Language for search results"
                },
                "provider": {
                    "type": "string",
                    "enum": ["duckduckgo", "auto"],
                    "default": "duckduckgo",
                    "description": "Search provider to use"
                }
            },
            "required": ["query"]
        })
    }
    
    async fn execute(&self, args: Value, _ctx: ToolContext) -> Result<ToolResult, ToolError> {
        let params: WebSearchParams = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidParameters(e.to_string()))?;
        
        let max_results = params.max_results.unwrap_or(10).min(20);
        let provider = params.provider.as_deref().unwrap_or("duckduckgo");
        
        let results = match provider {
            "duckduckgo" | "auto" => {
                self.search_duckduckgo(&params.query, max_results).await?
            },
            _ => {
                return Err(ToolError::InvalidParameters(format!("Unsupported search provider: {}", provider)));
            }
        };
        
        let output = if results.is_empty() {
            format!("No search results found for query: {}", params.query)
        } else {
            let mut output = format!("Search results for: {}\n\n", params.query);
            for result in &results {
                output.push_str(&format!(
                    "{}. **{}**\n   URL: {}\n   {}\n   Source: {}\n\n",
                    result.rank,
                    result.title,
                    result.url,
                    result.description,
                    result.source
                ));
            }
            output
        };
        
        Ok(ToolResult {
            title: format!("Search results for: {}", params.query),
            output,
            metadata: json!({
                "query": params.query,
                "results_count": results.len(),
                "max_results": max_results,
                "language": params.language.unwrap_or_else(|| "en".to_string()),
                "provider": provider,
                "results": results
            }),
        })
    }
}

/// Extract text content from HTML using scraper
fn extract_text_from_html(html: &str) -> Result<String, ToolError> {
    use scraper::{Html, Selector};
    
    let document = Html::parse_document(html);
    
    // Remove script and style content
    let script_selector = Selector::parse("script, style, noscript").unwrap();
    let mut clean_html = html.to_string();
    
    for element in document.select(&script_selector) {
        if let Some(html_content) = element.html().get(0..element.html().len()) {
            clean_html = clean_html.replace(html_content, "");
        }
    }
    
    let clean_document = Html::parse_document(&clean_html);
    let body_selector = Selector::parse("body").unwrap();
    
    let text = if let Some(body) = clean_document.select(&body_selector).next() {
        body.text().collect::<Vec<_>>().join(" ")
    } else {
        // Fallback to whole document
        clean_document.root_element().text().collect::<Vec<_>>().join(" ")
    };
    
    // Clean up whitespace
    let re = regex::Regex::new(r"\s+").unwrap();
    let cleaned = re.replace_all(&text, " ");
    
    Ok(cleaned.trim().to_string())
}

/// Convert HTML to Markdown using html2md
fn convert_html_to_markdown(html: &str) -> Result<String, ToolError> {
    // Clean the HTML first
    let clean_html = clean_html_for_markdown(html);
    
    // Convert to markdown
    let markdown = html2md::parse_html(&clean_html);
    
    // Clean up the markdown
    let re = regex::Regex::new(r"\n\s*\n\s*\n").unwrap();
    let cleaned = re.replace_all(&markdown, "\n\n");
    
    Ok(cleaned.trim().to_string())
}

/// Clean HTML before markdown conversion
fn clean_html_for_markdown(html: &str) -> String {
    let mut cleaned = html.to_string();
    
    // Remove script and style tags
    let re = regex::Regex::new(r"(?s)<(script|style|noscript)[^>]*>.*?</\1>").unwrap();
    cleaned = re.replace_all(&cleaned, "").to_string();
    
    // Remove comments
    let re = regex::Regex::new(r"(?s)<!--.*?-->").unwrap();
    cleaned = re.replace_all(&cleaned, "").to_string();
    
    // Clean up attributes we don't need
    let re = regex::Regex::new(r#"\s+(class|id|style|onclick|onload)="[^"]*""#).unwrap();
    cleaned = re.replace_all(&cleaned, "").to_string();
    
    cleaned
}