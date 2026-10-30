use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, LmStudioExecutionBackend,
    LocalEndpoint,
};
use serde_json::json;
use std::sync::atomic::AtomicBool;

#[test]
#[ignore = "requires an explicitly configured local MLX diagnostic server"]
fn mlx_live_respects_available_tool_scope_for_read_and_unavailable_write() {
    let endpoint = std::env::var("DESKTOPLAB_MLX_TEST_ENDPOINT").expect("local endpoint required");
    assert!(endpoint.starts_with("http://127.0.0.1:"));
    let model = "mlx-community/SmolLM3-3B-4bit";
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&[model]),
    );
    let schemas = inspection_schemas();
    for (request, allowed) in [
        (
            "Read README.md and briefly describe its purpose.",
            "desktoplab.read_file",
        ),
        (
            "Create a text file in this workspace containing the word verified.",
            "desktoplab.clarify",
        ),
    ] {
        let prompt = BackendPrompt::new(model, request)
            .with_tools(schemas.clone())
            .with_request_timeout_seconds(180);
        let output = backend
            .execute_constrained_chat(&prompt)
            .expect("valid constrained response");
        let value: serde_json::Value = serde_json::from_str(&output).expect("canonical response");
        assert_eq!(
            value["tool"], allowed,
            "runtime selected an unavailable tool"
        );
    }
}

#[test]
#[ignore = "requires an explicitly configured local MLX diagnostic server"]
fn mlx_live_stream_reaches_a_valid_read_only_tool() {
    let endpoint = std::env::var("DESKTOPLAB_MLX_TEST_ENDPOINT").expect("local endpoint required");
    assert!(endpoint.starts_with("http://127.0.0.1:"));
    let model = "mlx-community/SmolLM3-3B-4bit";
    let backend = LmStudioExecutionBackend::new(
        LocalEndpoint::available(endpoint),
        BackendModelInventory::available(&[model]),
    );
    let prompt = BackendPrompt::new(model, "Read README.md and briefly describe its purpose.")
        .with_tools(inspection_schemas())
        .with_request_timeout_seconds(180);
    let output = backend
        .execute_constrained_chat_stream(&prompt, &AtomicBool::new(false), |_| {})
        .expect("valid streamed response");
    let value: serde_json::Value = serde_json::from_str(&output).expect("canonical response");
    assert_eq!(value["tool"], "desktoplab.read_file");
}

fn inspection_schemas() -> Vec<BackendToolSchema> {
    vec![
        BackendToolSchema::new(
            "desktoplab.read_file",
            "Read a workspace file.",
            json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}),
        ),
        BackendToolSchema::new(
            "desktoplab.clarify",
            "Ask for clarification.",
            json!({"type":"object","properties":{"message":{"type":"string"}},"required":["message"]}),
        ),
        BackendToolSchema::new(
            "desktoplab.complete",
            "Complete a task with evidence.",
            json!({"type":"object","properties":{"message":{"type":"string"},"outcome":{"type":"string"},"evidenceCallIds":{"type":"array","items":{"type":"string"}}},"required":["message","outcome","evidenceCallIds"]}),
        ),
    ]
}
