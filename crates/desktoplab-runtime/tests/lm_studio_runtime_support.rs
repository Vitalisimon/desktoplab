use desktoplab_runtime::{
    LmStudioEndpointProbe, LmStudioRuntime, RuntimeHealth, RuntimeId, RuntimeState,
};
use xtask::check_logical_line_limit;

#[test]
fn lm_studio_is_exposed_behind_the_runtime_abstraction() {
    let runtime = LmStudioRuntime::new();

    assert_eq!(runtime.runtime_id(), &RuntimeId::new("runtime.lm-studio"));
    assert_eq!(runtime.display_name(), "LM Studio");
    assert!(
        runtime
            .capabilities()
            .contains(&"runtime.local".to_string())
    );
    assert!(
        runtime
            .capabilities()
            .contains(&"api.openai-compatible.local".to_string())
    );
}

#[test]
fn unavailable_lm_studio_api_degrades_explicitly() {
    let probe =
        LmStudioEndpointProbe::new("http://127.0.0.1:1234").mark_unavailable("connection refused");
    let detection = LmStudioRuntime::new().detect_endpoint(probe);

    assert!(!detection.is_available());
    assert!(detection.is_degraded());
    assert_eq!(detection.reason(), Some("connection refused"));
}

#[test]
fn lm_studio_health_verification_blocks_readiness_when_endpoint_fails() {
    let verification =
        LmStudioRuntime::new().verify(RuntimeHealth::unhealthy("models endpoint missing"));
    let mut status = desktoplab_runtime::RuntimeStatus::installed(
        RuntimeId::new("runtime.lm-studio"),
        "LM Studio",
        "0.3",
    );

    status.apply_verification(verification);

    assert_eq!(status.state(), RuntimeState::VerificationFailed);
    assert!(!status.is_ready());
}

#[test]
fn routing_can_distinguish_ollama_and_lm_studio_capabilities() {
    let ollama = desktoplab_runtime::OllamaRuntime::new();
    let lm_studio = LmStudioRuntime::new();

    assert!(
        ollama
            .capabilities()
            .contains(&"models.download".to_string())
    );
    assert!(
        !ollama
            .capabilities()
            .contains(&"api.openai-compatible.local".to_string())
    );
    assert!(
        lm_studio
            .capabilities()
            .contains(&"api.openai-compatible.local".to_string())
    );
}

#[test]
fn lm_studio_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/lm_studio.rs",
        include_str!("../src/lm_studio.rs"),
        250,
    )
    .expect("lm studio runtime source should stay below the initial line-count guard");
}
