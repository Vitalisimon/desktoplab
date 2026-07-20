use desktoplab_backends::{
    BackendToolCallEvidence, ProviderCompatibilityProfile, backend_response_to_agent_text,
    parse_ollama_tool_response, parse_openai_compatible_tool_response,
};
use serde_json::json;

#[test]
fn provider_profiles_preserve_protocol_specific_reasoning_fields() {
    let ollama = parse_ollama_tool_response(
        &json!({"message":{"content":"done","thinking":"private reasoning"}}),
        evidence("backend.ollama"),
    );
    let openai = parse_openai_compatible_tool_response(
        &json!({"choices":[{"message":{
            "content":"done",
            "reasoning_content":"provider reasoning"
        }}]}),
        evidence("backend.openai-compatible"),
    );

    assert_eq!(ollama.reasoning_text(), Some("private reasoning"));
    assert_eq!(openai.reasoning_text(), Some("provider reasoning"));
    assert_eq!(backend_response_to_agent_text(ollama).unwrap(), "done");
}

#[test]
fn active_profiles_shape_tool_choice_by_wire_protocol() {
    let mut ollama = json!({});
    let mut openai = json!({});

    ProviderCompatibilityProfile::ollama_chat().apply_tool_choice(&mut ollama);
    ProviderCompatibilityProfile::openai_chat_completions().apply_tool_choice(&mut openai);

    assert!(ollama.get("tool_choice").is_none());
    assert_eq!(openai["tool_choice"], "auto");
}

#[test]
fn parallel_and_malformed_tool_calls_fail_closed() {
    let parallel = parse_ollama_tool_response(
        &json!({"message":{"content":"", "tool_calls":[
            tool_call("desktoplab.read_file", json!({"path":"a.md"})),
            tool_call("desktoplab.read_file", json!({"path":"b.md"}))
        ]}}),
        evidence("backend.ollama"),
    );
    let malformed = parse_openai_compatible_tool_response(
        &json!({"choices":[{"message":{"content":"", "tool_calls":[{
            "function":{"arguments":"{}"}
        }]}}]}),
        evidence("backend.openai-compatible"),
    );

    assert_eq!(
        backend_response_to_agent_text(parallel).unwrap_err(),
        "parallel_tool_calls_unsupported"
    );
    assert_eq!(
        backend_response_to_agent_text(malformed).unwrap_err(),
        "provider_tool_call_missing_name"
    );
}

#[test]
fn compatibility_profile_sources_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-backends/src/provider_compatibility.rs",
            include_str!("../src/provider_compatibility.rs"),
            100,
        ),
        (
            "crates/desktoplab-backends/src/tool_response_bridge.rs",
            include_str!("../src/tool_response_bridge.rs"),
            60,
        ),
        (
            "crates/desktoplab-backends/tests/provider_compatibility_profiles.rs",
            include_str!("provider_compatibility_profiles.rs"),
            120,
        ),
    ] {
        xtask::check_logical_line_limit(path, source, max_lines).unwrap();
    }
}

fn evidence(backend: &str) -> BackendToolCallEvidence {
    BackendToolCallEvidence::native(backend, "model", "http://localhost", false)
}

fn tool_call(name: &str, arguments: serde_json::Value) -> serde_json::Value {
    json!({"function":{"name":name,"arguments":arguments}})
}
