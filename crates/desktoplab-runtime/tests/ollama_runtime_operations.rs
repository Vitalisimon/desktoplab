use desktoplab_runtime::{
    InstallHostCapacity, OllamaBinaryVerification, OllamaHostAdapter, OllamaInstallPlanError,
    OllamaRuntime, RuntimeHealth, RuntimeInstallPlanner, RuntimeInstallRequest, RuntimeState,
};
use xtask::check_logical_line_limit;

#[test]
fn ollama_installer_is_not_bundled_and_has_platform_metadata() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should be supported");

    assert!(!plan.is_bundled());
    assert_eq!(plan.target_platform(), Some("darwin-arm64"));
    assert!(
        plan.explanation()
            .contains("installer_source: https://ollama.com/download")
    );
    assert!(
        plan.explanation()
            .contains("verify: signed Ollama installer")
    );
}

#[test]
fn ollama_install_plans_are_platform_specific_without_downloads() {
    let runtime = OllamaRuntime::new();

    let macos = runtime
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 plan should be available")
        .explanation();
    let windows = runtime
        .try_platform_install_plan("windows-x64")
        .expect("Windows x64 plan should be available")
        .explanation();
    let linux = runtime
        .try_platform_install_plan("linux-x64")
        .expect("Linux x64 plan should be available")
        .explanation();

    assert!(macos.contains("open Ollama .dmg installer"));
    assert!(macos.contains("https://ollama.com/download/Ollama.dmg"));
    assert!(windows.contains("run OllamaSetup.exe"));
    assert!(windows.contains("https://ollama.com/download/OllamaSetup.exe"));
    assert!(linux.contains("run verified install.sh"));
    assert!(linux.contains("https://ollama.com/install.sh"));
    assert!(!linux.contains("Ollama.dmg"));
}

#[test]
fn unsupported_ollama_install_platform_returns_blocked_reason() {
    let error = OllamaRuntime::new()
        .try_platform_install_plan("solaris-sparc")
        .expect_err("unsupported platform should be blocked");

    assert_eq!(
        error,
        OllamaInstallPlanError::UnsupportedPlatform {
            target: "solaris-sparc".to_string(),
        }
    );
    assert_eq!(
        error.reason(),
        "Ollama installer is not supported on solaris-sparc."
    );
}

#[test]
fn linux_ollama_install_plan_blocks_automatic_execution_without_trusted_digest() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("linux-x64")
        .expect("Linux plan should be available");
    let planner = RuntimeInstallPlanner::new(InstallHostCapacity::new(64, true));

    let error = planner
        .plan_job(RuntimeInstallRequest::new(plan).with_setup_plan_accepted(true))
        .expect_err("Linux install should not be automatic without trusted digest metadata");

    assert_eq!(
        error,
        desktoplab_runtime::RuntimeInstallError::MissingVerificationMetadata
    );
}

#[test]
fn ollama_model_pull_ref_validation_blocks_shell_injection_characters() {
    let runtime = OllamaRuntime::new();

    assert!(runtime.validate_model_pull_ref("qwen2.5-coder:7b").is_ok());
    assert!(
        runtime
            .validate_model_pull_ref("deepseek-coder:6.7b")
            .is_ok()
    );
    assert!(runtime.validate_model_pull_ref("nemotron:70b").is_ok());
    assert!(
        runtime
            .validate_model_pull_ref("qwen:7b; rm -rf /")
            .is_err()
    );
    assert!(runtime.validate_model_pull_ref("../secret").is_err());
    assert!(runtime.validate_model_pull_ref("bad token").is_err());
}

#[test]
fn failed_binary_verification_blocks_readiness() {
    let status = OllamaRuntime::new().status_from_binary_verification(
        OllamaBinaryVerification::failed("downloaded binary checksum mismatch"),
    );

    assert_eq!(status.state(), RuntimeState::VerificationFailed);
    assert_eq!(
        status.failure_reason(),
        Some("downloaded binary checksum mismatch")
    );
}

#[test]
fn health_endpoint_failure_becomes_degraded_state() {
    let status =
        OllamaRuntime::new().status_from_health(RuntimeHealth::unhealthy("ollama API offline"));

    assert_eq!(status.state(), RuntimeState::Degraded);
    assert_eq!(status.failure_reason(), Some("ollama API offline"));
}

#[test]
fn model_inventory_is_read_through_ollama_adapter() {
    let inventory = OllamaRuntime::new().model_inventory(&FixtureOllamaHost);

    assert_eq!(
        inventory,
        vec!["qwen3:8b".to_string(), "devstral:24b".to_string()]
    );
}

#[test]
fn ollama_operations_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/ollama.rs",
        include_str!("../src/ollama.rs"),
        280,
    )
    .expect("ollama operations source should stay below the initial line-count guard");
}

struct FixtureOllamaHost;

impl OllamaHostAdapter for FixtureOllamaHost {
    fn list_models(&self) -> Vec<String> {
        vec!["qwen3:8b".to_string(), "devstral:24b".to_string()]
    }
}
