use desktoplab_backends::BackendMessage;
use desktoplab_runtime::{
    DeterministicProcessRunner, LmStudioModelProbe, RuntimeEndpointError,
    discover_existing_lm_studio,
};

use super::{AgentExecutionBinding, LocalApiRouter, PreparedAgentModelExecution};

struct Models;

impl LmStudioModelProbe for Models {
    fn models(&self, _endpoint: &str) -> Result<Vec<String>, RuntimeEndpointError> {
        Ok(vec!["gemma4:12b".to_string()])
    }
}

fn discovery() -> desktoplab_runtime::LmStudioExistingDiscovery {
    discover_existing_lm_studio(
        &DeterministicProcessRunner::sequence(vec![
            (Some(0), r#"{"running":true,"port":12345}"#, ""),
            (Some(0), r#"{"status":"running","isDaemon":false}"#, ""),
        ]),
        &Models,
    )
}

#[test]
fn lm_studio_execution_uses_session_model_and_discovered_port() {
    let mut router = LocalApiRouter::default();
    router.selected_route_id = crate::execution_routes::local_route_id("model.gemma4-12b-q4");
    let binding = AgentExecutionBinding::capture(&router, "backend.lm-studio");
    router.selected_route_id = crate::execution_routes::local_route_id("model.qwen3.5-9b-q4");

    let execution = router.prepare_lm_studio_model_execution_with_discovery(
        &binding,
        vec![BackendMessage::user("test")],
        &discovery(),
    );

    let PreparedAgentModelExecution::LmStudio { backend, prompt } = execution else {
        panic!("session-bound LM Studio execution should be prepared");
    };
    assert_eq!(prompt.model(), "gemma4:12b");
    assert_eq!(
        backend.chat_completions_url(),
        "http://127.0.0.1:12345/v1/chat/completions"
    );
}

#[test]
fn lm_studio_execution_fails_when_bound_model_is_not_loaded() {
    let mut router = LocalApiRouter::default();
    router.selected_route_id = crate::execution_routes::local_route_id("model.qwen3.5-9b-q4");
    let binding = AgentExecutionBinding::capture(&router, "backend.lm-studio");

    let execution = router.prepare_lm_studio_model_execution_with_discovery(
        &binding,
        vec![BackendMessage::user("test")],
        &discovery(),
    );

    let PreparedAgentModelExecution::Failed(reason) = execution else {
        panic!("an unloaded LM Studio model must fail closed");
    };
    assert_eq!(reason, "lm_studio_model_unavailable");
}

#[test]
fn managed_lm_studio_execution_uses_the_pinned_api_model_identifier() {
    let mut router = LocalApiRouter::default();
    router.selected_route_id =
        crate::execution_routes::local_route_id("model.gpt-oss-20b-lm-studio");
    let binding = AgentExecutionBinding::capture(&router, "backend.lm-studio");

    let execution = router.prepare_lm_studio_model_execution_with_inventory(
        &binding,
        vec![BackendMessage::user("test")],
        "http://127.0.0.1:12345",
        &["desktoplab-gpt-oss-20b".to_string()],
    );

    let PreparedAgentModelExecution::LmStudio { backend, prompt } = execution else {
        panic!("managed LM Studio execution should be prepared");
    };
    assert_eq!(prompt.model(), "desktoplab-gpt-oss-20b");
    assert_eq!(
        backend.chat_completions_url(),
        "http://127.0.0.1:12345/v1/chat/completions"
    );
}

#[test]
fn ollama_execution_fails_if_readiness_moved_to_another_model() {
    let mut router = LocalApiRouter::default();
    router.selected_route_id = crate::execution_routes::local_route_id("model.gemma4-12b-q4");
    let binding = AgentExecutionBinding::capture(&router, "backend.ollama");
    router.readiness = router
        .readiness
        .clone()
        .select("runtime.ollama", "model.qwen3.5-9b-q4");
    router.readiness.mark_model_capabilities(
        desktoplab_backends::BackendModelCapabilities::reported(
            "backend.ollama",
            "qwen3.5:9b",
            None,
            Some(32_768),
            ["tools"],
        ),
    );

    let execution =
        router.prepare_ollama_model_execution(&binding, vec![BackendMessage::user("test")]);

    let PreparedAgentModelExecution::Failed(reason) = execution else {
        panic!("changed model readiness must fail closed");
    };
    assert_eq!(reason, "session_model_configuration_changed");
}

#[test]
fn ollama_execution_uses_wizard_memory_budget_for_bound_model() {
    let mut router = LocalApiRouter::default();
    router.set_host_memory_gb_for_test(36);
    router.selected_route_id = crate::execution_routes::local_route_id("model.gemma4-12b-q4");
    router.readiness = router
        .readiness
        .clone()
        .select("runtime.ollama", "model.gemma4-12b-q4");
    router.readiness.mark_model_capabilities(
        desktoplab_backends::BackendModelCapabilities::reported(
            "backend.ollama",
            "gemma4:12b",
            None,
            Some(256_000),
            ["tools"],
        ),
    );
    let binding = AgentExecutionBinding::capture(&router, "backend.ollama");

    let execution =
        router.prepare_ollama_model_execution(&binding, vec![BackendMessage::user("test")]);

    let PreparedAgentModelExecution::Ollama { prompt, .. } = execution else {
        panic!("configured Ollama execution should be prepared");
    };
    assert_eq!(prompt.context_window_tokens(), Some(65_536));
    assert_eq!(prompt.request_timeout_seconds(), Some(240));
}
