//! HTTP client abstraction for unified web requests across native and WASM

use crate::error::Error;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

/// HTTP method enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Patch,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Head => write!(f, "HEAD"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Patch => write!(f, "PATCH"),
        }
    }
}

/// HTTP request builder
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: Url,
    pub headers: HashMap<String, String>,
    pub body: Option<Bytes>,
    pub timeout: Option<Duration>,
    pub follow_redirects: bool,
    pub max_redirects: u32,
    pub user_agent: Option<String>,
}

impl HttpRequest {
    pub fn new(method: HttpMethod, url: Url) -> Self {
        Self {
            method,
            url,
            headers: HashMap::new(),
            body: None,
            timeout: Some(Duration::from_secs(30)),
            follow_redirects: true,
            max_redirects: 10,
            user_agent: Some(default_user_agent()),
        }
    }

    pub fn get(url: Url) -> Self {
        Self::new(HttpMethod::Get, url)
    }

    pub fn post(url: Url) -> Self {
        Self::new(HttpMethod::Post, url)
    }

    pub fn header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    pub fn body(mut self, body: impl Into<Bytes>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn json<T: Serialize>(mut self, data: &T) -> Result<Self, Error> {
        let json = serde_json::to_vec(data)
            .map_err(|e| Error::Other(anyhow::anyhow!("JSON serialization failed: {}", e)))?;
        self.body = Some(json.into());
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    pub fn form(mut self, data: &HashMap<String, String>) -> Self {
        let form_data = data
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        self.body = Some(form_data.into_bytes().into());
        self.headers.insert("Content-Type".to_string(), "application/x-www-form-urlencoded".to_string());
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.timeout = Some(duration);
        self
    }

    pub fn user_agent(mut self, ua: String) -> Self {
        self.user_agent = Some(ua);
        self
    }

    pub fn no_redirects(mut self) -> Self {
        self.follow_redirects = false;
        self
    }
}

/// HTTP response
#[derive(Debug)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
    pub url: Url,
}

impl HttpResponse {
    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn is_redirect(&self) -> bool {
        self.status >= 300 && self.status < 400
    }

    pub fn header(&self, name: &str) -> Option<&String> {
        self.headers.get(name)
    }

    pub fn content_type(&self) -> Option<&String> {
        self.header("content-type").or_else(|| self.header("Content-Type"))
    }

    pub fn content_length(&self) -> Option<usize> {
        self.header("content-length")
            .or_else(|| self.header("Content-Length"))
            .and_then(|s| s.parse().ok())
    }

    pub fn body(&self) -> &Bytes {
        &self.body
    }

    pub fn text(&self) -> Result<String, Error> {
        String::from_utf8(self.body.to_vec())
            .map_err(|e| Error::Other(anyhow::anyhow!("Invalid UTF-8: {}", e)))
    }

    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, Error> {
        serde_json::from_slice(&self.body)
            .map_err(|e| Error::Other(anyhow::anyhow!("JSON deserialization failed: {}", e)))
    }
}

/// Request/Response interceptor trait
#[async_trait]
pub trait HttpInterceptor: Send + Sync {
    /// Called before sending a request
    async fn before_request(&self, request: &mut HttpRequest) -> Result<(), Error>;
    
    /// Called after receiving a response
    async fn after_response(&self, response: &mut HttpResponse) -> Result<(), Error>;
}

/// Rate limiting interceptor
pub struct RateLimiter {
    requests_per_second: f64,
    last_request: std::sync::Arc<parking_lot::Mutex<Option<std::time::Instant>>>,
}

impl RateLimiter {
    pub fn new(requests_per_second: f64) -> Self {
        Self {
            requests_per_second,
            last_request: std::sync::Arc::new(parking_lot::Mutex::new(None)),
        }
    }
}

#[async_trait]
impl HttpInterceptor for RateLimiter {
    async fn before_request(&self, _request: &mut HttpRequest) -> Result<(), Error> {
        let sleep_duration = {
            let mut last = self.last_request.lock();
            if let Some(last_time) = *last {
                let min_interval = Duration::from_secs_f64(1.0 / self.requests_per_second);
                let elapsed = last_time.elapsed();
                if elapsed < min_interval {
                    Some(min_interval - elapsed)
                } else {
                    None
                }
            } else {
                None
            }
        };
        
        if let Some(duration) = sleep_duration {
            tokio::time::sleep(duration).await;
        }
        
        {
            let mut last = self.last_request.lock();
            *last = Some(std::time::Instant::now());
        }
        
        Ok(())
    }

    async fn after_response(&self, _response: &mut HttpResponse) -> Result<(), Error> {
        Ok(())
    }
}

/// User-Agent interceptor
pub struct UserAgentInterceptor {
    user_agent: String,
}

impl UserAgentInterceptor {
    pub fn new(user_agent: String) -> Self {
        Self { user_agent }
    }
}

#[async_trait]
impl HttpInterceptor for UserAgentInterceptor {
    async fn before_request(&self, request: &mut HttpRequest) -> Result<(), Error> {
        if request.user_agent.is_none() {
            request.user_agent = Some(self.user_agent.clone());
        }
        Ok(())
    }

    async fn after_response(&self, _response: &mut HttpResponse) -> Result<(), Error> {
        Ok(())
    }
}

/// Cookie jar for session management
#[derive(Debug, Clone)]
pub struct CookieJar {
    cookies: std::sync::Arc<parking_lot::RwLock<HashMap<String, cookie::Cookie<'static>>>>,
}

impl CookieJar {
    pub fn new() -> Self {
        Self {
            cookies: std::sync::Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn add_cookie(&self, cookie: cookie::Cookie<'static>) {
        let mut cookies = self.cookies.write();
        cookies.insert(cookie.name().to_string(), cookie);
    }

    pub fn get_cookies_for_url(&self, url: &Url) -> Vec<cookie::Cookie<'static>> {
        let cookies = self.cookies.read();
        cookies
            .values()
            .filter(|cookie| {
                // Basic domain/path matching
                if let Some(domain) = cookie.domain() {
                    if let Some(host) = url.host_str() {
                        if !host.ends_with(domain) && host != domain {
                            return false;
                        }
                    }
                }
                if let Some(path) = cookie.path() {
                    if !url.path().starts_with(path) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn cookie_header_for_url(&self, url: &Url) -> Option<String> {
        let cookies = self.get_cookies_for_url(url);
        if cookies.is_empty() {
            None
        } else {
            Some(
                cookies
                    .iter()
                    .map(|c| format!("{}={}", c.name(), c.value()))
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        }
    }
}

/// Cookie interceptor
pub struct CookieInterceptor {
    jar: CookieJar,
}

impl CookieInterceptor {
    pub fn new(jar: CookieJar) -> Self {
        Self { jar }
    }
}

#[async_trait]
impl HttpInterceptor for CookieInterceptor {
    async fn before_request(&self, request: &mut HttpRequest) -> Result<(), Error> {
        if let Some(cookie_header) = self.jar.cookie_header_for_url(&request.url) {
            request.headers.insert("Cookie".to_string(), cookie_header);
        }
        Ok(())
    }

    async fn after_response(&self, response: &mut HttpResponse) -> Result<(), Error> {
        // Parse Set-Cookie headers
        for (name, value) in &response.headers {
            if name.to_lowercase() == "set-cookie" {
                if let Ok(cookie) = cookie::Cookie::parse(value.clone()) {
                    self.jar.add_cookie(cookie.into_owned());
                }
            }
        }
        Ok(())
    }
}

/// HTTP client trait
#[async_trait]
pub trait HttpClient: Send + Sync {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, Error>;
}

/// HTTP client builder
pub struct HttpClientBuilder {
    interceptors: Vec<Box<dyn HttpInterceptor>>,
    cookie_jar: Option<CookieJar>,
    rate_limit: Option<f64>,
    default_user_agent: Option<String>,
    default_timeout: Option<Duration>,
    verify_ssl: bool,
    proxy: Option<String>,
}

impl HttpClientBuilder {
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
            cookie_jar: None,
            rate_limit: None,
            default_user_agent: None,
            default_timeout: Some(Duration::from_secs(30)),
            verify_ssl: true,
            proxy: None,
        }
    }

    pub fn interceptor(mut self, interceptor: Box<dyn HttpInterceptor>) -> Self {
        self.interceptors.push(interceptor);
        self
    }

    pub fn cookie_jar(mut self, jar: CookieJar) -> Self {
        self.cookie_jar = Some(jar);
        self
    }

    pub fn rate_limit(mut self, requests_per_second: f64) -> Self {
        self.rate_limit = Some(requests_per_second);
        self
    }

    pub fn user_agent(mut self, ua: String) -> Self {
        self.default_user_agent = Some(ua);
        self
    }

    pub fn timeout(mut self, duration: Duration) -> Self {
        self.default_timeout = Some(duration);
        self
    }

    pub fn verify_ssl(mut self, verify: bool) -> Self {
        self.verify_ssl = verify;
        self
    }

    pub fn proxy(mut self, proxy_url: String) -> Self {
        self.proxy = Some(proxy_url);
        self
    }

    pub fn build(mut self) -> Result<Box<dyn HttpClient>, Error> {
        // Add default interceptors
        if let Some(rate) = self.rate_limit {
            self.interceptors.push(Box::new(RateLimiter::new(rate)));
        }

        if let Some(ua) = self.default_user_agent {
            self.interceptors.push(Box::new(UserAgentInterceptor::new(ua)));
        }

        if let Some(jar) = self.cookie_jar {
            self.interceptors.push(Box::new(CookieInterceptor::new(jar)));
        }

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "wasm32")] {
                Ok(Box::new(WasmHttpClient::new(self.interceptors, self.default_timeout)?))
            } else {
                Ok(Box::new(NativeHttpClient::new(
                    self.interceptors,
                    self.default_timeout,
                    self.verify_ssl,
                    self.proxy,
                )?))
            }
        }
    }
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Native HTTP client implementation using reqwest
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeHttpClient {
    client: reqwest::Client,
    interceptors: Vec<Box<dyn HttpInterceptor>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl NativeHttpClient {
    pub fn new(
        interceptors: Vec<Box<dyn HttpInterceptor>>,
        default_timeout: Option<Duration>,
        verify_ssl: bool,
        proxy: Option<String>,
    ) -> Result<Self, Error> {
        let mut builder = reqwest::Client::builder()
            .danger_accept_invalid_certs(!verify_ssl)
            .redirect(reqwest::redirect::Policy::none());

        if let Some(timeout) = default_timeout {
            builder = builder.timeout(timeout);
        }

        if let Some(proxy_url) = proxy {
            let proxy = reqwest::Proxy::all(&proxy_url)
                .map_err(|e| Error::Other(anyhow::anyhow!("Invalid proxy URL: {}", e)))?;
            builder = builder.proxy(proxy);
        }

        let client = builder
            .build()
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to create HTTP client: {}", e)))?;

        Ok(Self { client, interceptors })
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait]
impl HttpClient for NativeHttpClient {
    async fn execute(&self, mut request: HttpRequest) -> Result<HttpResponse, Error> {
        // Apply request interceptors
        for interceptor in &self.interceptors {
            interceptor.before_request(&mut request).await?;
        }

        let method = match request.method {
            HttpMethod::Get => reqwest::Method::GET,
            HttpMethod::Post => reqwest::Method::POST,
            HttpMethod::Put => reqwest::Method::PUT,
            HttpMethod::Delete => reqwest::Method::DELETE,
            HttpMethod::Head => reqwest::Method::HEAD,
            HttpMethod::Options => reqwest::Method::OPTIONS,
            HttpMethod::Patch => reqwest::Method::PATCH,
        };

        let mut req_builder = self.client.request(method, request.url.clone());

        // Add headers
        for (key, value) in &request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add user agent
        if let Some(ua) = &request.user_agent {
            req_builder = req_builder.header("User-Agent", ua);
        }

        // Add body
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }

        // Set timeout
        if let Some(timeout) = request.timeout {
            req_builder = req_builder.timeout(timeout);
        }

        let response = req_builder
            .send()
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("HTTP request failed: {}", e)))?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        let body = response
            .bytes()
            .await
            .map_err(|e| Error::Other(anyhow::anyhow!("Failed to read response body: {}", e)))?;

        let mut http_response = HttpResponse {
            status,
            headers,
            body,
            url: request.url,
        };

        // Apply response interceptors
        for interceptor in &self.interceptors {
            interceptor.after_response(&mut http_response).await?;
        }

        Ok(http_response)
    }
}

/// WASM HTTP client implementation using web-sys fetch
#[cfg(target_arch = "wasm32")]
pub struct WasmHttpClient {
    interceptors: Vec<Box<dyn HttpInterceptor>>,
    default_timeout: Option<Duration>,
}

#[cfg(target_arch = "wasm32")]
impl WasmHttpClient {
    pub fn new(
        interceptors: Vec<Box<dyn HttpInterceptor>>,
        default_timeout: Option<Duration>,
    ) -> Result<Self, Error> {
        Ok(Self {
            interceptors,
            default_timeout,
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait]
impl HttpClient for WasmHttpClient {
    async fn execute(&self, mut request: HttpRequest) -> Result<HttpResponse, Error> {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, Response};

        // Apply request interceptors
        for interceptor in &self.interceptors {
            interceptor.before_request(&mut request).await?;
        }

        let mut opts = RequestInit::new();
        opts.method(&request.method.to_string());

        // Add body
        if let Some(body) = request.body {
            let uint8_array = js_sys::Uint8Array::new_with_length(body.len() as u32);
            uint8_array.copy_from(&body);
            opts.body(Some(&uint8_array));
        }

        // Create headers
        let headers = web_sys::Headers::new()
            .map_err(|_| Error::Other("Failed to create headers".to_string()))?;

        for (key, value) in &request.headers {
            headers
                .set(key, value)
                .map_err(|_| Error::Other(format!("Failed to set header: {}", key)))?;
        }

        if let Some(ua) = &request.user_agent {
            headers
                .set("User-Agent", ua)
                .map_err(|_| Error::Other("Failed to set User-Agent".to_string()))?;
        }

        opts.headers(&headers);

        let req = Request::new_with_str_and_init(&request.url.to_string(), &opts)
            .map_err(|_| Error::Other("Failed to create request".to_string()))?;

        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&req))
            .await
            .map_err(|_| Error::Other("Fetch failed".to_string()))?;

        let resp: Response = resp_value
            .dyn_into()
            .map_err(|_| Error::Other("Invalid response".to_string()))?;

        let status = resp.status() as u16;

        // Extract headers
        let mut response_headers = HashMap::new();
        let headers_iter = js_sys::try_iter(&resp.headers())
            .map_err(|_| Error::Other("Failed to iterate headers".to_string()))?
            .ok_or_else(|| Error::Other("Headers not iterable".to_string()))?;

        for item in headers_iter {
            let item = item.map_err(|_| Error::Other("Header iteration error".to_string()))?;
            let entry = js_sys::Array::from(&item);
            let key = entry.get(0).as_string().unwrap_or_default();
            let value = entry.get(1).as_string().unwrap_or_default();
            response_headers.insert(key, value);
        }

        // Read body
        let array_buffer = JsFuture::from(resp.array_buffer())
            .await
            .map_err(|_| Error::Other("Failed to read response body".to_string()))?;

        let uint8_array = js_sys::Uint8Array::new(&array_buffer);
        let body = uint8_array.to_vec().into();

        let mut http_response = HttpResponse {
            status,
            headers: response_headers,
            body,
            url: request.url,
        };

        // Apply response interceptors
        for interceptor in &self.interceptors {
            interceptor.after_response(&mut http_response).await?;
        }

        Ok(http_response)
    }
}

/// Default User-Agent string
pub fn default_user_agent() -> String {
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 CodeMesh/1.0".to_string()
}

/// Security helpers for SSRF protection
pub fn is_safe_url(url: &Url) -> bool {
    // Check scheme
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }

    // Check for private/internal IP ranges
    if let Some(host) = url.host() {
        match host {
            url::Host::Ipv4(ip) => {
                if ip.is_private() || ip.is_loopback() || ip.is_link_local() {
                    return false;
                }
            }
            url::Host::Ipv6(ip) => {
                if ip.is_loopback() || ip.is_unspecified() {
                    return false;
                }
            }
            url::Host::Domain(domain) => {
                // Block localhost and internal domains
                if domain == "localhost" || domain.ends_with(".local") || domain.ends_with(".internal") {
                    return false;
                }
            }
        }
    }

    true
}

/// Sanitize URL to prevent SSRF attacks
pub fn sanitize_url(url_str: &str) -> Result<Url, Error> {
    let url = Url::parse(url_str)
        .map_err(|e| Error::Other(anyhow::anyhow!("Invalid URL: {}", e)))?;

    if !is_safe_url(&url) {
        return Err(Error::Other(anyhow::anyhow!("URL not allowed for security reasons")));
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_safety() {
        // Safe URLs
        assert!(is_safe_url(&Url::parse("https://example.com").unwrap()));
        assert!(is_safe_url(&Url::parse("http://google.com").unwrap()));

        // Unsafe URLs
        assert!(!is_safe_url(&Url::parse("http://127.0.0.1").unwrap()));
        assert!(!is_safe_url(&Url::parse("http://localhost").unwrap()));
        assert!(!is_safe_url(&Url::parse("http://192.168.1.1").unwrap()));
        assert!(!is_safe_url(&Url::parse("file:///etc/passwd").unwrap()));
    }

    #[test]
    fn test_cookie_jar() {
        let jar = CookieJar::new();
        let cookie = cookie::Cookie::build(("session", "abc123"))
            .domain("example.com")
            .path("/")
            .finish();
        
        jar.add_cookie(cookie.into_owned());

        let url = Url::parse("https://example.com/test").unwrap();
        let header = jar.cookie_header_for_url(&url);
        assert_eq!(header, Some("session=abc123".to_string()));
    }
}