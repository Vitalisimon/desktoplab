use desktoplab_runtime::{
    MlxLmEndpointProbe, MlxLmRuntime, RuntimeHealth, RuntimeManagementMode, VerificationResult,
};
use xtask::check_logical_line_limit;

#[test]
fn mlx_lm_is_exposed_as_macos_native_openai_compatible_runtime() {
    let runtime = MlxLmRuntime::new();

    assert_eq!(runtime.runtime_id().as_str(), "runtime.mlx-lm");
    assert_eq!(runtime.display_name(), "MLX-LM Server");
    assert!(
        runtime
            .capabilities()
            .contains(&"runtime.local".to_string())
    );
    assert!(
        runtime
            .capabilities()
            .contains(&"runtime.apple-silicon".to_string())
    );
    assert!(
        runtime
            .capabilities()
            .contains(&"api.openai-compatible.local".to_string())
    );
}

#[test]
fn mlx_lm_install_plan_is_macos_aarch64_only_and_not_silent_system_mutation() {
    let runtime = MlxLmRuntime::new();

    let plan = runtime
        .try_install_plan("macos-aarch64")
        .expect("Apple Silicon Macs should get an MLX-LM plan");

    assert_eq!(plan.runtime_id().as_str(), "runtime.mlx-lm");
    assert!(plan.explanation().contains("pip install mlx-lm"));
    assert!(plan.explanation().contains("mlx_lm.server"));
    assert!(plan.explanation().contains("OpenAI-compatible endpoint"));
    assert!(
        plan.explanation()
            .contains("execution_strategy: python_environment")
    );

    let unsupported = runtime
        .try_install_plan("linux-x64")
        .expect_err("MLX-LM is not a Linux runtime path");
    assert!(unsupported.reason().contains("Apple Silicon"));
}

#[test]
fn mlx_lm_endpoint_detection_and_process_spec_are_local_only() {
    let runtime = MlxLmRuntime::new();
    let detection = runtime.detect_endpoint(
        MlxLmEndpointProbe::new("http://127.0.0.1:8080/v1")
            .with_model("mlx-community/Qwen-3.5-4B-8bit"),
    );

    assert!(detection.is_available());
    assert_eq!(detection.endpoint(), "http://127.0.0.1:8080/v1");
    assert_eq!(
        detection.models(),
        &["mlx-community/Qwen-3.5-4B-8bit".to_string()]
    );

    let metadata = runtime.local_endpoint_metadata(detection.endpoint());
    assert!(metadata.is_openai_compatible());
    assert!(!metadata.requires_provider_credential());

    let command = runtime.start_command("mlx-community/Qwen-3.5-4B-8bit");
    assert_eq!(command.program(), "mlx_lm.server");
    assert_eq!(
        command.args(),
        &[
            "--model".to_string(),
            "mlx-community/Qwen-3.5-4B-8bit".to_string()
        ]
    );

    let spec = runtime.process_spec();
    assert_eq!(
        spec.management_mode(),
        RuntimeManagementMode::DesktopLabManaged
    );
}

#[test]
fn mlx_lm_verification_blocks_unhealthy_endpoint() {
    let runtime = MlxLmRuntime::new();

    assert_eq!(
        runtime.verify(RuntimeHealth::healthy()),
        VerificationResult::passed()
    );
    assert_eq!(
        runtime.verify(RuntimeHealth::unhealthy(
            "endpoint did not answer /v1/models"
        )),
        VerificationResult::failed("endpoint did not answer /v1/models")
    );
}

#[test]
fn mlx_lm_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/mlx_lm.rs",
        include_str!("../src/mlx_lm.rs"),
        260,
    )
    .expect("mlx-lm runtime source should stay focused");
}
