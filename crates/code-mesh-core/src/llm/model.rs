//! Model trait and common implementations

use super::provider::{Cost, Limits, ModelCapabilities};
use serde_json::Value;
use std::collections::HashMap;

/// Model trait representing an AI model
pub trait Model: Send + Sync {
    /// Unique identifier for the model
    fn id(&self) -> &str;
    
    /// Display name for the model
    fn name(&self) -> &str;
    
    /// Release date of the model
    fn release_date(&self) -> &str;
    
    /// Get model capabilities
    fn capabilities(&self) -> &ModelCapabilities;
    
    /// Get cost information
    fn cost(&self) -> &Cost;
    
    /// Get model limits
    fn limits(&self) -> &Limits;
    
    /// Get model-specific options
    fn options(&self) -> &HashMap<String, Value>;
    
    // Convenience methods
    fn supports_attachments(&self) -> bool {
        self.capabilities().attachment
    }
    
    fn supports_reasoning(&self) -> bool {
        self.capabilities().reasoning
    }
    
    fn supports_temperature(&self) -> bool {
        self.capabilities().temperature
    }
    
    fn supports_tool_calls(&self) -> bool {
        self.capabilities().tool_call
    }
    
    fn supports_vision(&self) -> bool {
        self.capabilities().vision
    }
    
    fn supports_caching(&self) -> bool {
        self.capabilities().caching
    }
    
    fn context_limit(&self) -> u32 {
        self.limits().context
    }
    
    fn output_limit(&self) -> u32 {
        self.limits().output
    }
}