use desktoplab_runtime::{
    LmStudioEndpointProbe, LmStudioHostAdapter, LmStudioRuntime, RuntimeState,
};
use xtask::check_logical_line_limit;

struct FakeLmStudioAdapter {
    models: Vec<String>,
}

impl LmStudioHostAdapter for FakeLmStudioAdapter {
    fn list_openai_compatible_models(&self, _endpoint: &str) -> Vec<String> {
        self.models.clone()
    }
}

#[test]
fn unavailable_endpoint_becomes_degraded_runtime_status() {
    let runtime = LmStudioRuntime::new();
    let detection = runtime.detect_endpoint(
        LmStudioEndpointProbe::new("http://127.0.0.1:1234").mark_unavailable("connection refused"),
    );

    let status = runtime.status_from_endpoint(detection);

    assert_eq!(status.state(), RuntimeState::Degraded);
    assert_eq!(status.failure_reason(), Some("connection refused"));
}

#[test]
fn guided_setup_is_explicit_when_automatic_install_is_unsupported() {
    let setup = LmStudioRuntime::new().guided_setup_plan("macos-arm64");

    assert!(!setup.can_install_automatically());
    assert_eq!(setup.mode(), "guided");
    assert!(setup.explanation().contains("open LM Studio manually"));
    assert!(setup.explanation().contains("start local server"));
}

#[test]
fn local_lm_studio_usage_requires_no_provider_credentials() {
    let endpoint = LmStudioRuntime::new().local_endpoint_metadata("http://127.0.0.1:1234");

    assert_eq!(endpoint.url(), "http://127.0.0.1:1234");
    assert!(endpoint.is_openai_compatible());
    assert!(!endpoint.requires_provider_credential());
}

#[test]
fn model_inventory_uses_openai_compatible_endpoint_adapter() {
    let adapter = FakeLmStudioAdapter {
        models: vec!["local-deepseek".to_string(), "qwen3".to_string()],
    };

    let models = LmStudioRuntime::new().model_inventory("http://127.0.0.1:1234", &adapter);

    assert_eq!(
        models,
        vec!["local-deepseek".to_string(), "qwen3".to_string()]
    );
}

#[test]
fn lm_studio_operations_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/lm_studio.rs",
        include_str!("../src/lm_studio.rs"),
        280,
    )
    .expect("lm studio runtime source should stay below the operations line-count guard");
}
