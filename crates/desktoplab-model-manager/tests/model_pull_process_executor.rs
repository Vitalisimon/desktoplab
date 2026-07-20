use desktoplab_model_manager::{
    ModelDownloadCapacity, ModelDownloadExecutionPolicy, ModelDownloadExecutor, ModelDownloadPlan,
    ModelManager,
};
use desktoplab_runtime::DeterministicProcessRunner;

#[test]
fn model_pull_executor_runs_ollama_pull_through_injected_process_runner() {
    let plan = gemma_plan(true);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(100_000));
    let runner = DeterministicProcessRunner::succeeds("pull complete", "");

    let result = executor
        .execute(plan, ModelDownloadExecutionPolicy::resumable(), &runner)
        .expect("accepted model download should execute through process runner");

    assert_eq!(result.state(), "completed");
    assert_eq!(result.command_evidence(), "ollama pull gemma4:12b");
    assert_eq!(result.stdout(), "pull complete");
    assert_eq!(result.stderr(), "");
    assert_eq!(result.reason(), None);
}

#[test]
fn model_pull_executor_reports_runtime_process_failure() {
    let plan = gemma_plan(true);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(100_000));
    let runner = DeterministicProcessRunner::fails("connection refused");

    let result = executor
        .execute(plan, ModelDownloadExecutionPolicy::resumable(), &runner)
        .expect("process failure should be reported as a download result");

    assert_eq!(result.state(), "blocked");
    assert_eq!(result.command_evidence(), "ollama pull gemma4:12b");
    assert_eq!(result.stderr(), "connection refused");
    assert_eq!(result.reason(), Some("runtime pull failed"));
}

#[test]
fn model_pull_executor_reuses_download_gates_before_spawning_runtime() {
    let plan = gemma_plan(false);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(100_000));
    let runner = DeterministicProcessRunner::succeeds("should not run", "");

    let error = executor
        .execute(plan, ModelDownloadExecutionPolicy::resumable(), &runner)
        .expect_err("unaccepted setup plan must block before runtime process execution");

    assert_eq!(
        error,
        desktoplab_model_manager::ModelDownloadError::SetupPlanNotAccepted
    );
}

fn gemma_plan(accepted: bool) -> ModelDownloadPlan {
    let variant = ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.gemma4-12b-q4")
        .expect("catalog should include Gemma 4 12B")
        .clone();
    ModelDownloadPlan::from_variant(&variant, accepted)
}
