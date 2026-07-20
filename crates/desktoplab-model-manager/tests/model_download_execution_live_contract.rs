use desktoplab_model_manager::{
    ModelDownloadPlan, ModelRuntimeAdapter, ModelRuntimeReadiness, OllamaModelRuntimeAdapter,
    RuntimeModelPullRequest,
};
use desktoplab_runtime::{DeterministicProcessRunner, RuntimeId};
use xtask::check_logical_line_limit;

#[test]
fn model_download_executes_runtime_adapter() {
    let adapter =
        OllamaModelRuntimeAdapter::new(DeterministicProcessRunner::succeeds("pull complete", ""));
    let result = adapter.pull(RuntimeModelPullRequest::new(
        RuntimeId::new("runtime.ollama"),
        "qwen2.5-coder:7b",
    ));

    assert_eq!(result.state(), "completed");
    assert_eq!(result.command_evidence(), "ollama pull qwen2.5-coder:7b");
    assert_eq!(result.stdout(), "pull complete");
}

#[test]
fn model_readiness_requires_runtime_inventory() {
    let adapter = OllamaModelRuntimeAdapter::new(DeterministicProcessRunner::succeeds(
        "qwen2.5-coder:7b\nllama3.1:8b",
        "",
    ));

    let ready = adapter.verify(ModelRuntimeReadiness::new(
        RuntimeId::new("runtime.ollama"),
        "qwen2.5-coder:7b",
    ));
    let missing = adapter.verify(ModelRuntimeReadiness::new(
        RuntimeId::new("runtime.ollama"),
        "deepseek-coder:6.7b",
    ));

    assert!(ready.is_ready());
    assert!(!missing.is_ready());
    assert_eq!(missing.reason(), Some("model_not_reported_by_runtime"));
}

#[test]
fn model_runtime_adapter_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-model-manager/src/runtime_adapter.rs",
        include_str!("../src/runtime_adapter.rs"),
        320,
    )
    .expect("model runtime adapter should stay focused");
}

#[test]
fn executable_download_keeps_plan_pull_ref() {
    let catalog = desktoplab_model_manager::ModelManager::new().default_family_catalog();
    let variant = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.gemma4-12b-q4")
        .expect("Gemma 4 12B should be available");
    let plan = ModelDownloadPlan::from_variant(variant, true);

    assert_eq!(plan.pull_ref(), "gemma4:12b");
}
