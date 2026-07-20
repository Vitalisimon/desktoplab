use std::cell::RefCell;

use desktoplab_agent_engine::{
    LlmExecutionAdapter, LlmExecutionError, LocalInferenceAdapter, LocalInferenceError,
    LocalInferenceRequest, LocalInferenceTransport,
};
use xtask::check_logical_line_limit;

#[test]
fn local_inference_requires_configured_endpoint_before_claiming_completion() {
    let adapter =
        LocalInferenceAdapter::ollama("backend.ollama", "runtime.ollama", "model.qwen-coder-7b-q4");

    let error = adapter
        .complete("Inspect repository", &RecordingTransport::default())
        .expect_err("missing endpoint should block honestly");

    assert_eq!(error, LocalInferenceError::NotConfigured);
}

#[test]
fn openai_compatible_local_endpoint_records_runtime_model_and_backend_evidence() {
    let transport = RecordingTransport::default();
    let adapter =
        LocalInferenceAdapter::ollama("backend.ollama", "runtime.ollama", "model.qwen-coder-7b-q4")
            .with_openai_compatible_endpoint("http://127.0.0.1:11434/v1/chat/completions");

    let result = adapter
        .complete("Summarize repository", &transport)
        .expect("configured local endpoint should call transport");

    assert_eq!(result.output(), "local endpoint response");
    assert_eq!(result.evidence().backend_id(), "backend.ollama");
    assert_eq!(result.evidence().runtime_id(), "runtime.ollama");
    assert_eq!(result.evidence().model_id(), "model.qwen-coder-7b-q4");
    assert!(result.evidence().endpoint().starts_with("http://127.0.0.1"));
    assert_eq!(
        transport.last_prompt(),
        Some("Summarize repository".to_string())
    );
}

#[test]
fn llm_local_adapter_blocks_when_no_inference_adapter_is_configured() {
    let error = LlmExecutionAdapter::local("backend.ollama")
        .complete("Inspect repository")
        .expect_err("local inference should not fake completion");

    assert_eq!(error, LlmExecutionError::LocalInferenceNotConfigured);
}

#[test]
fn local_inference_boundary_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/inference.rs",
        include_str!("../src/inference.rs"),
        190,
    )
    .expect("local inference boundary should stay focused");
}

#[derive(Default)]
struct RecordingTransport {
    last_prompt: RefCell<Option<String>>,
}

impl RecordingTransport {
    fn last_prompt(&self) -> Option<String> {
        self.last_prompt.borrow().clone()
    }
}

impl LocalInferenceTransport for RecordingTransport {
    fn post_chat(&self, request: &LocalInferenceRequest) -> Result<String, LocalInferenceError> {
        assert!(request.endpoint().ends_with("/chat/completions"));
        assert_eq!(request.model_id(), "model.qwen-coder-7b-q4");
        self.last_prompt
            .borrow_mut()
            .replace(request.prompt().to_string());
        Ok("local endpoint response".to_string())
    }
}
