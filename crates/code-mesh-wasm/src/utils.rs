//! Utility functions for WASM environment detection and helpers

use wasm_bindgen::prelude::*;
use web_sys::{window, Performance};
use js_sys::{Object, Array, Date};
use serde::{Serialize, Deserialize};

/// Platform information
#[derive(Serialize, Deserialize)]
pub struct PlatformInfo {
    pub is_browser: bool,
    pub is_node: bool,
    pub is_webworker: bool,
    pub user_agent: Option<String>,
    pub platform: Option<String>,
    pub language: Option<String>,
    pub screen_resolution: Option<(u32, u32)>,
    pub timezone: Option<String>,
}

/// WASM features support
#[derive(Serialize, Deserialize)]
pub struct WasmFeatures {
    pub simd: bool,
    pub threads: bool,
    pub bulk_memory: bool,
    pub reference_types: bool,
    pub multi_value: bool,
    pub tail_call: bool,
}

/// Memory usage information
#[derive(Serialize, Deserialize)]
pub struct MemoryInfo {
    pub used: u32,
    pub total: u32,
    pub limit: u32,
}

/// Check if running in browser environment
pub fn is_browser() -> bool {
    window().is_some()
}

/// Check if running in Node.js environment
pub fn is_node() -> bool {
    js_sys::eval("typeof process !== 'undefined' && process.versions && process.versions.node")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Check if running in web worker environment
pub fn is_webworker() -> bool {
    js_sys::eval("typeof WorkerGlobalScope !== 'undefined' && typeof importScripts === 'function'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Check if running in service worker environment
pub fn is_service_worker() -> bool {
    js_sys::eval("typeof ServiceWorkerGlobalScope !== 'undefined'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Get comprehensive platform information
pub fn get_platform_info() -> Result<PlatformInfo, JsValue> {
    let is_browser = is_browser();
    let is_node = is_node();
    let is_webworker = is_webworker();
    
    let (user_agent, platform, language, screen_resolution) = if is_browser {
        let window = window().unwrap();
        let navigator = window.navigator();
        
        let user_agent = navigator.user_agent().ok();
        let platform = navigator.platform().ok();
        let language = navigator.language();
        
        let screen_resolution = window.screen().ok().map(|screen| {
            let width = screen.width().unwrap_or(0) as u32;
            let height = screen.height().unwrap_or(0) as u32;
            (width, height)
        });
        
        (user_agent, platform, language, screen_resolution)
    } else {
        (None, None, None, None)
    };
    
    let timezone = if is_browser {
        js_sys::eval("Intl.DateTimeFormat().resolvedOptions().timeZone")
            .ok()
            .and_then(|v| v.as_string())
    } else {
        None
    };
    
    Ok(PlatformInfo {
        is_browser,
        is_node,
        is_webworker,
        user_agent,
        platform,
        language,
        screen_resolution,
        timezone,
    })
}

/// Check WASM feature support
pub fn check_wasm_features() -> Result<WasmFeatures, JsValue> {
    let simd = js_sys::eval("typeof WebAssembly.SIMD !== 'undefined'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    let threads = js_sys::eval("typeof SharedArrayBuffer !== 'undefined' && typeof Atomics !== 'undefined'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false);
    
    let bulk_memory = check_wasm_proposal("bulk-memory")?;
    let reference_types = check_wasm_proposal("reference-types")?;
    let multi_value = check_wasm_proposal("multi-value")?;
    let tail_call = check_wasm_proposal("tail-call")?;
    
    Ok(WasmFeatures {
        simd,
        threads,
        bulk_memory,
        reference_types,
        multi_value,
        tail_call,
    })
}

/// Check support for specific WASM proposal
fn check_wasm_proposal(proposal: &str) -> Result<bool, JsValue> {
    // This is a simplified check - in practice, you'd test specific instructions
    match proposal {
        "bulk-memory" => {
            // Test if bulk memory operations are supported
            let test_module = js_sys::Uint8Array::from(&[
                0x00, 0x61, 0x73, 0x6d, // WASM magic
                0x01, 0x00, 0x00, 0x00, // version
            ]);
            
            js_sys::eval("WebAssembly.validate")
                .and_then(|validate_fn| {
                    let args = Array::new();
                    args.push(&test_module);
                    js_sys::Reflect::apply(&validate_fn.into(), &JsValue::null(), &args)
                })
                .map(|v| v.as_bool().unwrap_or(false))
                .unwrap_or(false)
        }
        _ => Ok(false), // Default to false for unknown proposals
    }
}

/// Get memory usage information
pub fn get_memory_usage() -> Result<JsValue, JsValue> {
    let mut info = Object::new();
    
    // Try to get JS heap info
    if is_browser() {
        let window = window().unwrap();
        if let Some(performance) = window.performance() {
            if let Ok(memory) = js_sys::Reflect::get(&performance, &"memory".into()) {
                if !memory.is_undefined() {
                    let used = js_sys::Reflect::get(&memory, &"usedJSHeapSize".into())
                        .unwrap_or_default()
                        .as_f64()
                        .unwrap_or(0.0) as u32;
                    
                    let total = js_sys::Reflect::get(&memory, &"totalJSHeapSize".into())
                        .unwrap_or_default()
                        .as_f64()
                        .unwrap_or(0.0) as u32;
                    
                    let limit = js_sys::Reflect::get(&memory, &"jsHeapSizeLimit".into())
                        .unwrap_or_default()
                        .as_f64()
                        .unwrap_or(0.0) as u32;
                    
                    js_sys::Reflect::set(&info, &"js_heap_used".into(), &used.into())?;
                    js_sys::Reflect::set(&info, &"js_heap_total".into(), &total.into())?;
                    js_sys::Reflect::set(&info, &"js_heap_limit".into(), &limit.into())?;
                }
            }
        }
    }
    
    // Add WASM memory info
    if let Ok(memory) = js_sys::eval("WebAssembly.Memory") {
        if !memory.is_undefined() {
            // This would require access to the actual memory instance
            js_sys::Reflect::set(&info, &"wasm_memory_available".into(), &true.into())?;
        }
    }
    
    Ok(info.into())
}

/// Format bytes into human-readable format
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    const THRESHOLD: u64 = 1024;
    
    if bytes == 0 {
        return "0 B".to_string();
    }
    
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= THRESHOLD as f64 && unit_index < UNITS.len() - 1 {
        size /= THRESHOLD as f64;
        unit_index += 1;
    }
    
    format!("{:.1} {}", size, UNITS[unit_index])
}

/// Get current timestamp in milliseconds
pub fn get_timestamp() -> f64 {
    Date::now()
}

/// Get high-resolution timestamp if available
pub fn get_high_res_timestamp() -> f64 {
    if let Some(window) = window() {
        if let Some(performance) = window.performance() {
            return performance.now();
        }
    }
    Date::now()
}

/// Sleep for specified milliseconds (async)
pub async fn sleep(ms: u32) -> Result<(), JsValue> {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let window = web_sys::window().unwrap();
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            &resolve,
            ms as i32,
        );
    });
    
    wasm_bindgen_futures::JsFuture::from(promise).await?;
    Ok(())
}

/// Create a debounced function
#[wasm_bindgen]
pub struct DebouncedFunction {
    timeout_id: Option<i32>,
    delay_ms: u32,
    callback: js_sys::Function,
}

#[wasm_bindgen]
impl DebouncedFunction {
    /// Create a new debounced function
    #[wasm_bindgen(constructor)]
    pub fn new(callback: js_sys::Function, delay_ms: u32) -> Self {
        Self {
            timeout_id: None,
            delay_ms,
            callback,
        }
    }
    
    /// Call the debounced function
    #[wasm_bindgen]
    pub fn call(&mut self, args: &JsValue) -> Result<(), JsValue> {
        let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
        
        // Clear existing timeout
        if let Some(id) = self.timeout_id {
            window.clear_timeout_with_handle(id);
        }
        
        // Set new timeout
        let callback = self.callback.clone();
        let args_clone = args.clone();
        
        let timeout_callback = Closure::wrap(Box::new(move || {
            let _ = callback.call1(&JsValue::null(), &args_clone);
        }) as Box<dyn FnMut()>);
        
        let timeout_id = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            timeout_callback.as_ref().unchecked_ref(),
            self.delay_ms as i32,
        )?;
        
        timeout_callback.forget();
        self.timeout_id = Some(timeout_id);
        
        Ok(())
    }
    
    /// Cancel the pending call
    #[wasm_bindgen]
    pub fn cancel(&mut self) -> Result<(), JsValue> {
        if let Some(id) = self.timeout_id.take() {
            let window = window().ok_or_else(|| JsValue::from_str("No window object"))?;
            window.clear_timeout_with_handle(id);
        }
        Ok(())
    }
}

/// Create a throttled function
#[wasm_bindgen]
pub struct ThrottledFunction {
    last_call: f64,
    delay_ms: u32,
    callback: js_sys::Function,
}

#[wasm_bindgen]
impl ThrottledFunction {
    /// Create a new throttled function
    #[wasm_bindgen(constructor)]
    pub fn new(callback: js_sys::Function, delay_ms: u32) -> Self {
        Self {
            last_call: 0.0,
            delay_ms,
            callback,
        }
    }
    
    /// Call the throttled function
    #[wasm_bindgen]
    pub fn call(&mut self, args: &JsValue) -> Result<(), JsValue> {
        let now = get_high_res_timestamp();
        
        if now - self.last_call >= self.delay_ms as f64 {
            self.last_call = now;
            self.callback.call1(&JsValue::null(), args)?;
        }
        
        Ok(())
    }
}

/// URL utilities
#[wasm_bindgen]
pub struct UrlUtils;

#[wasm_bindgen]
impl UrlUtils {
    /// Parse URL and return components
    #[wasm_bindgen]
    pub fn parse_url(url: String) -> Result<JsValue, JsValue> {
        let url_obj = web_sys::Url::new(&url)?;
        
        let result = Object::new();
        js_sys::Reflect::set(&result, &"protocol".into(), &url_obj.protocol().into())?;
        js_sys::Reflect::set(&result, &"hostname".into(), &url_obj.hostname().into())?;
        js_sys::Reflect::set(&result, &"port".into(), &url_obj.port().into())?;
        js_sys::Reflect::set(&result, &"pathname".into(), &url_obj.pathname().into())?;
        js_sys::Reflect::set(&result, &"search".into(), &url_obj.search().into())?;
        js_sys::Reflect::set(&result, &"hash".into(), &url_obj.hash().into())?;
        
        Ok(result.into())
    }
    
    /// Get current page URL
    #[wasm_bindgen]
    pub fn get_current_url() -> Option<String> {
        window()
            .and_then(|w| w.location().href().ok())
    }
    
    /// Build URL with query parameters
    #[wasm_bindgen]
    pub fn build_url(base: String, params: &JsValue) -> Result<String, JsValue> {
        let url = web_sys::Url::new(&base)?;
        let search_params = url.search_params();
        
        // Add parameters from object
        let object: Object = params.clone().dyn_into()?;
        let entries = Object::entries(&object);
        
        for i in 0..entries.length() {
            let entry = entries.get(i);
            let entry_array: Array = entry.dyn_into()?;
            
            let key = entry_array.get(0).as_string().unwrap_or_default();
            let value = entry_array.get(1).as_string().unwrap_or_default();
            
            search_params.set(&key, &value);
        }
        
        Ok(url.href())
    }
}

/// Environment detection functions for WASM exports
#[wasm_bindgen]
pub fn wasm_is_browser() -> bool {
    is_browser()
}

#[wasm_bindgen]
pub fn wasm_is_node() -> bool {
    is_node()
}

#[wasm_bindgen]
pub fn wasm_is_webworker() -> bool {
    is_webworker()
}

#[wasm_bindgen]
pub fn wasm_get_platform_info() -> Result<JsValue, JsValue> {
    let info = get_platform_info()?;
    serde_wasm_bindgen::to_value(&info)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_check_features() -> Result<JsValue, JsValue> {
    let features = check_wasm_features()?;
    serde_wasm_bindgen::to_value(&features)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn wasm_get_memory_usage() -> Result<JsValue, JsValue> {
    get_memory_usage()
}

#[wasm_bindgen]
pub fn wasm_format_bytes(bytes: f64) -> String {
    format_bytes(bytes as u64)
}

#[wasm_bindgen]
pub fn wasm_get_timestamp() -> f64 {
    get_timestamp()
}

#[wasm_bindgen]
pub fn wasm_get_high_res_timestamp() -> f64 {
    get_high_res_timestamp()
}