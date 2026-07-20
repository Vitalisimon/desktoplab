use desktoplab_runtime::{
    InstallPlan, OllamaRuntime, RuntimeHealth, RuntimeId, RuntimeProbe, RuntimeState,
    VerificationResult,
};
use xtask::check_logical_line_limit;

#[test]
fn ollama_is_exposed_behind_the_runtime_abstraction() {
    let ollama = OllamaRuntime::new();

    assert_eq!(ollama.runtime_id(), &RuntimeId::new("runtime.ollama"));
    assert_eq!(ollama.display_name(), "Ollama");
    assert!(ollama.capabilities().contains(&"runtime.local".to_string()));
    assert!(
        ollama
            .capabilities()
            .contains(&"models.inventory".to_string())
    );
}

#[test]
fn ollama_install_plan_downloads_on_demand_and_is_not_bundled() {
    let plan: InstallPlan = OllamaRuntime::new().install_plan("macos-aarch64");

    assert_eq!(plan.runtime_name(), "Ollama");
    assert!(!plan.is_bundled());
    assert!(
        plan.explanation()
            .contains("download Ollama installer on demand")
    );
    assert!(plan.explanation().contains("verify downloaded binary"));
}

#[test]
fn failed_ollama_verification_blocks_readiness() {
    let health = RuntimeHealth::unhealthy("ollama API did not respond");
    let verification = OllamaRuntime::new().verify(health);

    assert_eq!(
        verification,
        VerificationResult::failed("ollama API did not respond")
    );
    let mut status = desktoplab_runtime::RuntimeStatus::installed(
        RuntimeId::new("runtime.ollama"),
        "Ollama",
        "0.9.0",
    );
    status.apply_verification(verification);
    assert_eq!(status.state(), RuntimeState::VerificationFailed);
    assert!(!status.is_ready());
}

#[test]
fn ollama_detection_and_model_inventory_are_probe_driven() {
    let probe = RuntimeProbe::new()
        .with_binary_path("/usr/local/bin/ollama")
        .with_version("0.9.0")
        .with_model("qwen3:8b")
        .with_model("devstral:24b");

    let detection = OllamaRuntime::new().detect(probe);

    assert!(detection.is_installed());
    assert_eq!(detection.version(), Some("0.9.0"));
    assert_eq!(
        detection.models(),
        &["qwen3:8b".to_string(), "devstral:24b".to_string()]
    );
}

#[test]
fn ollama_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/ollama.rs",
        include_str!("../src/ollama.rs"),
        250,
    )
    .expect("ollama runtime source should stay below the initial line-count guard");
}
