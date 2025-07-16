//! Model trait and common implementations

use super::provider::{Cost, Limits, ModelCapabilities};
use serde_json::Value;
use std::collections::HashMap;

/// Model trait representing an AI model (re-exported from provider)
pub use super::provider::Model;