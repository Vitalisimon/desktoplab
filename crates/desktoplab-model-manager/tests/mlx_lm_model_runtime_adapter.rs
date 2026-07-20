use desktoplab_model_manager::{
    MlxLmModelRuntimeAdapter, ModelRuntimeAdapter, ModelRuntimeReadiness, RuntimeModelPullRequest,
};
use desktoplab_runtime::{DeterministicProcessRunner, RuntimeId};
use xtask::check_logical_line_limit;

#[test]
fn mlx_lm_adapter_downloads_models_through_generate_entrypoint() {
    let adapter = MlxLmModelRuntimeAdapter::new(DeterministicProcessRunner::succeeds("ready", ""));

    let result = adapter.pull(RuntimeModelPullRequest::new(
        RuntimeId::new("runtime.mlx-lm"),
        "mlx-community/Qwen-3.5-4B-8bit",
    ));

    assert_eq!(result.state(), "completed");
    assert_eq!(
        result.command_evidence(),
        "mlx_lm.generate --model mlx-community/Qwen-3.5-4B-8bit --prompt DesktopLab model readiness check. --max-tokens 1"
    );
}

#[test]
fn mlx_lm_adapter_verifies_readiness_by_loading_the_model() {
    let adapter = MlxLmModelRuntimeAdapter::new(DeterministicProcessRunner::succeeds("ok", ""));

    let ready = adapter.verify(ModelRuntimeReadiness::new(
        RuntimeId::new("runtime.mlx-lm"),
        "mlx-community/Qwen-3.5-4B-8bit",
    ));

    assert!(ready.is_ready());
}

#[test]
fn mlx_lm_adapter_rejects_unsafe_model_refs_before_process_execution() {
    let adapter =
        MlxLmModelRuntimeAdapter::new(DeterministicProcessRunner::succeeds("should not run", ""));

    let result = adapter.pull(RuntimeModelPullRequest::new(
        RuntimeId::new("runtime.mlx-lm"),
        "../secret",
    ));

    assert_eq!(result.state(), "blocked");
    assert_eq!(result.reason(), Some("unsafe model reference"));
}

#[test]
fn mlx_lm_adapter_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-model-manager/src/runtime_adapter.rs",
        include_str!("../src/runtime_adapter.rs"),
        320,
    )
    .expect("model runtime adapter should stay focused after MLX-LM support");
}
