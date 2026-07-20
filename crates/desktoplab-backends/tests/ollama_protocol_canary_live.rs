use desktoplab_backends::{
    BackendModelInventory, BackendPrompt, BackendToolSchema, ModelCapabilityState,
    ModelProtocolCertificationState, OllamaExecutionBackend, OllamaModelCapabilityResolver,
    OllamaToolProtocolCanary,
};
use serde_json::json;

#[test]
#[ignore = "requires an explicitly selected live Ollama model"]
fn selected_live_ollama_digest_passes_tool_protocol_canary() {
    let endpoint = std::env::var("DESKTOPLAB_LIVE_OLLAMA_ENDPOINT")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string());
    let model = std::env::var("DESKTOPLAB_LIVE_OLLAMA_MODEL")
        .expect("DESKTOPLAB_LIVE_OLLAMA_MODEL must name an installed model");
    let capabilities = OllamaModelCapabilityResolver::default()
        .resolve(&endpoint, &model)
        .expect("live Ollama capability discovery should pass");

    assert_eq!(
        capabilities.capability_state("tools"),
        ModelCapabilityState::Confirmed
    );
    let certification =
        OllamaToolProtocolCanary::default().certify_fresh(&endpoint, &capabilities, 300);
    eprintln!(
        "model={} fingerprint={} protocol={:?}",
        capabilities.model_id(),
        capabilities.fingerprint(),
        certification.protocol()
    );
    assert_eq!(
        certification.state(),
        ModelProtocolCertificationState::Certified,
        "{}",
        certification.failure_reason().unwrap_or("unknown failure")
    );

    let profile = capabilities
        .clone()
        .with_tool_protocol_certification(certification);
    let backend = OllamaExecutionBackend::new(BackendModelInventory::available(&[&model]))
        .with_model_capabilities([profile]);
    let prompt =
        BackendPrompt::new(&model, "Call desktoplab.read_file for README.md.").with_tools(vec![
            BackendToolSchema::new(
                "desktoplab.read_file",
                "Read one workspace file.",
                json!({
                    "type":"object",
                    "properties":{"path":{"type":"string"}},
                    "required":["path"]
                }),
            ),
        ]);
    let output = backend
        .execute_chat(&endpoint, &prompt)
        .expect("certified adapter should normalize the live tool response");
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["tool"], "desktoplab.read_file");
    assert_eq!(output["arguments"]["path"], "README.md");
}
