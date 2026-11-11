//! Tests for the exported `code_mesh_wasm` API. Runs under `wasm-pack test --node`.

use code_mesh_wasm::{benchmark_performance, process_data, CodeMesh};
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn agents_are_created_with_sequential_ids() {
    let mut mesh = CodeMesh::new();
    mesh.init().unwrap();
    assert_eq!(mesh.get_agent_count(), 0);

    assert_eq!(mesh.create_agent("coder").unwrap(), "agent_0");
    assert_eq!(mesh.create_agent("tester").unwrap(), "agent_1");
    assert_eq!(mesh.get_agent_count(), 2);
}

#[wasm_bindgen_test]
fn execute_task_requires_a_known_agent() {
    let mut mesh = CodeMesh::new();
    let id = mesh.create_agent("coder").unwrap();

    let done = mesh.execute_task(&id, "write tests").unwrap();
    assert_eq!(done, "Task 'write tests' completed by agent agent_0");

    let err = mesh.execute_task("agent_99", "anything").unwrap_err();
    assert_eq!(err.as_string().as_deref(), Some("Agent agent_99 not found"));
}

#[wasm_bindgen_test]
fn agent_info_and_listing_round_trip() {
    let mut mesh = CodeMesh::new();
    let id = mesh.create_agent("reviewer").unwrap();
    mesh.execute_task(&id, "review").unwrap();

    let info = mesh.get_agent_info(&id).unwrap();
    let agent_type = js_sys::Reflect::get(&info, &"agent_type".into()).unwrap();
    let tasks = js_sys::Reflect::get(&info, &"tasks_completed".into()).unwrap();
    assert_eq!(agent_type.as_string().as_deref(), Some("reviewer"));
    assert_eq!(tasks.as_f64(), Some(1.0));

    let list = js_sys::Array::from(&mesh.list_agents().unwrap());
    assert_eq!(list.length(), 1);
    assert!(mesh.get_agent_info("missing").is_err());
}

#[wasm_bindgen_test]
fn performance_metrics_count_agents_and_tasks() {
    let mut mesh = CodeMesh::new();
    let a = mesh.create_agent("coder").unwrap();
    mesh.create_agent("tester").unwrap();
    mesh.execute_task(&a, "one").unwrap();
    mesh.execute_task(&a, "two").unwrap();

    let metrics = mesh.get_performance_metrics().unwrap();
    let get = |k: &str| js_sys::Reflect::get(&metrics, &k.into()).unwrap().as_f64();
    assert_eq!(get("total_agents"), Some(2.0));
    assert_eq!(get("active_agents"), Some(2.0));
    assert_eq!(get("total_tasks"), Some(2.0));
}

#[wasm_bindgen_test]
fn process_data_uppercases_input() {
    assert_eq!(process_data("mesh").unwrap(), "Processed: MESH");
}

#[wasm_bindgen_test]
fn benchmark_reports_iterations() {
    let result = benchmark_performance(100).unwrap();
    let iterations = js_sys::Reflect::get(&result, &"iterations".into()).unwrap();
    assert_eq!(iterations.as_f64(), Some(100.0));
}
