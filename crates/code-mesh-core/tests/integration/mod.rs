//! Integration tests for code-mesh-core

use code_mesh_core::*;
use std::sync::Arc;
use tempfile::TempDir;

mod common;
use common::{mocks::*, fixtures::*, *};

pub mod end_to_end_tests;
pub mod api_integration_tests;
pub mod workflow_tests;
pub mod performance_tests;