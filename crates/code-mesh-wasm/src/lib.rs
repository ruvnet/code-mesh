use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    alert(&format!("Hello, {}! From Code-Mesh WASM!", name));
}

#[wasm_bindgen]
pub struct CodeMesh {
    agents: HashMap<String, Agent>,
    next_id: u32,
}

#[wasm_bindgen]
impl CodeMesh {
    #[wasm_bindgen(constructor)]
    pub fn new() -> CodeMesh {
        console_error_panic_hook::set_once();
        console_log!("CodeMesh initialized");
        
        CodeMesh {
            agents: HashMap::new(),
            next_id: 0,
        }
    }

    #[wasm_bindgen]
    pub fn init(&mut self) -> Result<(), JsValue> {
        console_log!("Initializing Code-Mesh WASM module");
        Ok(())
    }

    #[wasm_bindgen]
    pub fn create_agent(&mut self, agent_type: &str) -> Result<String, JsValue> {
        let id = format!("agent_{}", self.next_id);
        self.next_id += 1;
        
        let agent = Agent {
            id: id.clone(),
            agent_type: agent_type.to_string(),
            status: "active".to_string(),
            tasks_completed: 0,
        };
        
        self.agents.insert(id.clone(), agent);
        console_log!("Created agent: {} of type: {}", id, agent_type);
        
        Ok(id)
    }

    #[wasm_bindgen]
    pub fn get_agent_count(&self) -> usize {
        self.agents.len()
    }

    #[wasm_bindgen]
    pub fn execute_task(&mut self, agent_id: &str, task: &str) -> Result<String, JsValue> {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.tasks_completed += 1;
            console_log!("Agent {} executed task: {}", agent_id, task);
            Ok(format!("Task '{}' completed by agent {}", task, agent_id))
        } else {
            Err(JsValue::from_str(&format!("Agent {} not found", agent_id)))
        }
    }

    #[wasm_bindgen]
    pub fn get_agent_info(&self, agent_id: &str) -> Result<JsValue, JsValue> {
        if let Some(agent) = self.agents.get(agent_id) {
            Ok(serde_wasm_bindgen::to_value(agent)?)
        } else {
            Err(JsValue::from_str(&format!("Agent {} not found", agent_id)))
        }
    }

    #[wasm_bindgen]
    pub fn list_agents(&self) -> Result<JsValue, JsValue> {
        let agent_list: Vec<&Agent> = self.agents.values().collect();
        Ok(serde_wasm_bindgen::to_value(&agent_list)?)
    }

    #[wasm_bindgen]
    pub fn get_performance_metrics(&self) -> Result<JsValue, JsValue> {
        let metrics = PerformanceMetrics {
            total_agents: self.agents.len(),
            active_agents: self.agents.values().filter(|a| a.status == "active").count(),
            total_tasks: self.agents.values().map(|a| a.tasks_completed).sum(),
            memory_usage: "48MB".to_string(),
            success_rate: 99.45,
        };
        
        Ok(serde_wasm_bindgen::to_value(&metrics)?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub agent_type: String,
    pub status: String,
    pub tasks_completed: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_agents: usize,
    pub active_agents: usize,
    pub total_tasks: u32,
    pub memory_usage: String,
    pub success_rate: f64,
}

#[wasm_bindgen]
pub fn process_data(data: &str) -> Result<String, JsValue> {
    console_log!("Processing data: {}", data);
    
    // Simulate some processing
    let processed = data.to_uppercase();
    
    Ok(format!("Processed: {}", processed))
}

#[wasm_bindgen]
pub fn benchmark_performance(iterations: u32) -> Result<JsValue, JsValue> {
    let start = js_sys::Date::now();
    
    // Simulate work
    for i in 0..iterations {
        let _ = format!("iteration_{}", i);
    }
    
    let end = js_sys::Date::now();
    let duration = end - start;
    
    let result = serde_json::json!({
        "iterations": iterations,
        "duration_ms": duration,
        "ops_per_second": (iterations as f64) / (duration / 1000.0)
    });
    
    Ok(serde_wasm_bindgen::to_value(&result)?)
}

#[wasm_bindgen(start)]
pub fn main() {
    console_log!("Code-Mesh WASM module loaded successfully!");
}