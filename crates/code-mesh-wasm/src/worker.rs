//! Web Worker support for background processing

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Worker, MessageEvent, DedicatedWorkerGlobalScope};
use js_sys::{Object, JSON, Promise};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Web Worker wrapper for Code Mesh operations
#[wasm_bindgen]
pub struct CodeMeshWorker {
    worker: Worker,
    task_id_counter: u32,
    pending_tasks: HashMap<u32, js_sys::Function>,
}

#[derive(Serialize, Deserialize)]
struct WorkerMessage {
    task_id: u32,
    task_type: String,
    payload: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
struct WorkerResponse {
    task_id: u32,
    success: bool,
    result: Option<serde_json::Value>,
    error: Option<String>,
}

impl CodeMeshWorker {
    /// Create a new web worker
    pub fn new(script_url: String) -> Result<Self, JsValue> {
        let worker = Worker::new(&script_url)?;
        
        let mut instance = Self {
            worker,
            task_id_counter: 0,
            pending_tasks: HashMap::new(),
        };
        
        instance.setup_message_handler()?;
        Ok(instance)
    }
    
    /// Setup message handler for worker responses
    fn setup_message_handler(&mut self) -> Result<(), JsValue> {
        let pending_tasks_ptr = &mut self.pending_tasks as *mut HashMap<u32, js_sys::Function>;
        
        let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
            if let Ok(data) = event.data().dyn_into::<js_sys::Object>() {
                if let Ok(json_str) = JSON::stringify(&data) {
                    if let Ok(response) = serde_json::from_str::<WorkerResponse>(&json_str.as_string().unwrap_or_default()) {
                        unsafe {
                            if let Some(pending_tasks) = pending_tasks_ptr.as_mut() {
                                if let Some(callback) = pending_tasks.remove(&response.task_id) {
                                    let result = if response.success {
                                        response.result.map(|r| serde_wasm_bindgen::to_value(&r).unwrap_or(JsValue::null())).unwrap_or(JsValue::null())
                                    } else {
                                        JsValue::from_str(&response.error.unwrap_or_default())
                                    };
                                    
                                    let _ = callback.call1(&JsValue::null(), &result);
                                }
                            }
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        
        self.worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
        
        Ok(())
    }
    
    /// Post a task to the worker
    pub async fn post_task(&mut self, task_type: String, payload: serde_json::Value) -> Result<JsValue, JsValue> {
        let task_id = self.task_id_counter;
        self.task_id_counter += 1;
        
        let message = WorkerMessage {
            task_id,
            task_type,
            payload,
        };
        
        let message_js = serde_wasm_bindgen::to_value(&message)?;
        self.worker.post_message(&message_js)?;
        
        // Create a promise that resolves when the task completes
        let (promise, resolve, reject) = self.create_task_promise();
        
        // Store the resolve function for later use
        self.pending_tasks.insert(task_id, resolve);
        
        Ok(promise.into())
    }
    
    /// Create a promise for task completion
    fn create_task_promise(&self) -> (Promise, js_sys::Function, js_sys::Function) {
        let mut resolve_fn: Option<js_sys::Function> = None;
        let mut reject_fn: Option<js_sys::Function> = None;
        
        let promise = Promise::new(&mut |resolve, reject| {
            resolve_fn = Some(resolve);
            reject_fn = Some(reject);
        });
        
        (promise, resolve_fn.unwrap(), reject_fn.unwrap())
    }
    
    /// Process text with AI model in worker
    pub async fn process_text(&mut self, text: String, model: String) -> Result<String, JsValue> {
        let payload = serde_json::json!({
            "text": text,
            "model": model
        });
        
        let result = self.post_task("process_text".to_string(), payload).await?;
        let promise: Promise = result.dyn_into()?;
        let response = JsFuture::from(promise).await?;
        
        response.as_string().ok_or_else(|| JsValue::from_str("Invalid response"))
    }
    
    /// Perform code analysis in worker
    pub async fn analyze_code(&mut self, code: String, language: String) -> Result<JsValue, JsValue> {
        let payload = serde_json::json!({
            "code": code,
            "language": language
        });
        
        let result = self.post_task("analyze_code".to_string(), payload).await?;
        let promise: Promise = result.dyn_into()?;
        JsFuture::from(promise).await
    }
    
    /// Format code in worker
    pub async fn format_code(&mut self, code: String, language: String, options: JsValue) -> Result<String, JsValue> {
        let options_json: serde_json::Value = serde_wasm_bindgen::from_value(options)?;
        let payload = serde_json::json!({
            "code": code,
            "language": language,
            "options": options_json
        });
        
        let result = self.post_task("format_code".to_string(), payload).await?;
        let promise: Promise = result.dyn_into()?;
        let response = JsFuture::from(promise).await?;
        
        response.as_string().ok_or_else(|| JsValue::from_str("Invalid response"))
    }
    
    /// Terminate the worker
    pub fn terminate(&self) {
        self.worker.terminate();
    }
}

#[wasm_bindgen]
impl CodeMeshWorker {
    /// Create a new web worker (WASM constructor)
    #[wasm_bindgen(constructor)]
    pub fn new_wasm(script_url: String) -> Result<CodeMeshWorker, JsValue> {
        Self::new(script_url)
    }
    
    /// Process text (WASM method)
    #[wasm_bindgen]
    pub async fn process_text_wasm(&mut self, text: String, model: String) -> Result<String, JsValue> {
        self.process_text(text, model).await
    }
    
    /// Analyze code (WASM method)
    #[wasm_bindgen]
    pub async fn analyze_code_wasm(&mut self, code: String, language: String) -> Result<JsValue, JsValue> {
        self.analyze_code(code, language).await
    }
    
    /// Format code (WASM method)
    #[wasm_bindgen]
    pub async fn format_code_wasm(&mut self, code: String, language: String, options: JsValue) -> Result<String, JsValue> {
        self.format_code(code, language, options).await
    }
    
    /// Terminate worker (WASM method)
    #[wasm_bindgen]
    pub fn terminate_wasm(&self) {
        self.terminate();
    }
}

/// Service Worker utilities
#[wasm_bindgen]
pub struct ServiceWorkerManager {
    registration: Option<web_sys::ServiceWorkerRegistration>,
}

impl ServiceWorkerManager {
    /// Create a new service worker manager
    pub fn new() -> Self {
        Self {
            registration: None,
        }
    }
    
    /// Register a service worker
    pub async fn register(&mut self, script_url: String) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object"))?;
        let navigator = window.navigator();
        let service_worker = navigator.service_worker()
            .ok_or_else(|| JsValue::from_str("Service Worker not supported"))?;
        
        let registration_promise = service_worker.register(&script_url);
        let registration = JsFuture::from(registration_promise).await?;
        
        self.registration = Some(registration.dyn_into()?);
        Ok(())
    }
    
    /// Check if service worker is registered
    pub fn is_registered(&self) -> bool {
        self.registration.is_some()
    }
    
    /// Update service worker
    pub async fn update(&self) -> Result<(), JsValue> {
        if let Some(registration) = &self.registration {
            let update_promise = registration.update();
            JsFuture::from(update_promise).await?;
        }
        Ok(())
    }
    
    /// Unregister service worker
    pub async fn unregister(&mut self) -> Result<bool, JsValue> {
        if let Some(registration) = &self.registration {
            let unregister_promise = registration.unregister();
            let result = JsFuture::from(unregister_promise).await?;
            let success = result.as_bool().unwrap_or(false);
            
            if success {
                self.registration = None;
            }
            
            Ok(success)
        } else {
            Ok(false)
        }
    }
    
    /// Send message to service worker
    pub fn send_message(&self, message: JsValue) -> Result<(), JsValue> {
        if let Some(registration) = &self.registration {
            if let Some(active) = registration.active() {
                active.post_message(&message)?;
            }
        }
        Ok(())
    }
}

#[wasm_bindgen]
impl ServiceWorkerManager {
    /// Create a new service worker manager (WASM constructor)
    #[wasm_bindgen(constructor)]
    pub fn new_wasm() -> ServiceWorkerManager {
        Self::new()
    }
    
    /// Register service worker (WASM method)
    #[wasm_bindgen]
    pub async fn register_wasm(&mut self, script_url: String) -> Result<(), JsValue> {
        self.register(script_url).await
    }
    
    /// Check registration status (WASM method)
    #[wasm_bindgen]
    pub fn is_registered_wasm(&self) -> bool {
        self.is_registered()
    }
    
    /// Update service worker (WASM method)
    #[wasm_bindgen]
    pub async fn update_wasm(&self) -> Result<(), JsValue> {
        self.update().await
    }
    
    /// Unregister service worker (WASM method)
    #[wasm_bindgen]
    pub async fn unregister_wasm(&mut self) -> Result<bool, JsValue> {
        self.unregister().await
    }
    
    /// Send message (WASM method)
    #[wasm_bindgen]
    pub fn send_message_wasm(&self, message: JsValue) -> Result<(), JsValue> {
        self.send_message(message)
    }
}

/// Check if web workers are supported
#[wasm_bindgen]
pub fn supports_web_workers() -> bool {
    js_sys::eval("typeof Worker !== 'undefined'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Check if service workers are supported
#[wasm_bindgen]
pub fn supports_service_workers() -> bool {
    web_sys::window()
        .and_then(|w| w.navigator().service_worker())
        .is_some()
}

/// Check if shared array buffer is supported (for threads)
#[wasm_bindgen]
pub fn supports_shared_array_buffer() -> bool {
    js_sys::eval("typeof SharedArrayBuffer !== 'undefined'")
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

/// Get the number of logical CPU cores
#[wasm_bindgen]
pub fn get_cpu_core_count() -> u32 {
    web_sys::window()
        .and_then(|w| w.navigator().hardware_concurrency())
        .unwrap_or(1) as u32
}

/// Create a worker pool for parallel processing
#[wasm_bindgen]
pub struct WorkerPool {
    workers: Vec<CodeMeshWorker>,
    current_worker: usize,
}

#[wasm_bindgen]
impl WorkerPool {
    /// Create a new worker pool
    #[wasm_bindgen(constructor)]
    pub fn new_wasm(script_url: String, worker_count: usize) -> Result<WorkerPool, JsValue> {
        let mut workers = Vec::new();
        
        for _ in 0..worker_count {
            workers.push(CodeMeshWorker::new(script_url.clone())?);
        }
        
        Ok(Self {
            workers,
            current_worker: 0,
        })
    }
    
    /// Get the next available worker (round-robin)
    fn next_worker(&mut self) -> &mut CodeMeshWorker {
        let worker = &mut self.workers[self.current_worker];
        self.current_worker = (self.current_worker + 1) % self.workers.len();
        worker
    }
    
    /// Process multiple tasks in parallel
    #[wasm_bindgen]
    pub async fn process_parallel(&mut self, tasks: js_sys::Array) -> Result<js_sys::Array, JsValue> {
        let results = js_sys::Array::new();
        
        for i in 0..tasks.length() {
            let task = tasks.get(i);
            let worker = self.next_worker();
            
            // Extract task data
            let task_type = js_sys::Reflect::get(&task, &"type".into())?
                .as_string()
                .unwrap_or_default();
            let payload = js_sys::Reflect::get(&task, &"payload".into())?;
            let payload_json: serde_json::Value = serde_wasm_bindgen::from_value(payload)?;
            
            let result = worker.post_task(task_type, payload_json).await?;
            results.push(&result);
        }
        
        Ok(results)
    }
    
    /// Terminate all workers
    #[wasm_bindgen]
    pub fn terminate_all(&self) {
        for worker in &self.workers {
            worker.terminate();
        }
    }
}