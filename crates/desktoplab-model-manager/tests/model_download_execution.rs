use desktoplab_model_manager::{
    DownloadPolicy, ModelDownloadCapacity, ModelDownloadError, ModelDownloadExecutionPolicy,
    ModelDownloadExecutor, ModelDownloadPlan, ModelDownloadState, ModelManager,
    ModelParameterClass, ModelVariant, RuntimeModelDownloadCommand, SetupSelection,
};
use desktoplab_runtime::RuntimeId;
use xtask::check_logical_line_limit;

#[test]
fn download_starts_after_accepted_setup_plan() {
    let selection = SetupSelection::accepted(RuntimeId::new("runtime.ollama"), "qwen3:8b", 256);
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(512));

    let job = executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .expect("accepted setup plan should start model download");

    assert_eq!(job.state(), ModelDownloadState::Running);
    assert_eq!(
        job.command(),
        &RuntimeModelDownloadCommand::new("ollama", &["pull", "qwen3:8b"])
    );
    assert_eq!(job.event_names(), vec!["queued", "started"]);
}

#[test]
fn unaccepted_preview_cannot_download() {
    let selection = SetupSelection::preview(RuntimeId::new("runtime.ollama"), "qwen3:8b");
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(512));

    let error = executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .expect_err("preview plan must not execute");

    assert_eq!(error, ModelDownloadError::SetupPlanNotAccepted);
}

#[test]
fn insufficient_disk_blocks_download_execution() {
    let selection = SetupSelection::accepted(RuntimeId::new("runtime.ollama"), "qwen3:8b", 1024);
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(512));

    let error = executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .expect_err("disk gate should block oversized model");

    assert_eq!(
        error,
        ModelDownloadError::InsufficientDisk {
            required_mb: 1024,
            available_mb: 512
        }
    );
}

#[test]
fn network_unavailable_blocks_download_before_runtime_command() {
    let selection = SetupSelection::accepted(RuntimeId::new("runtime.ollama"), "qwen3:8b", 1024);
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);
    let executor =
        ModelDownloadExecutor::new(ModelDownloadCapacity::new(2048).with_network_available(false));

    let error = executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .expect_err("network gate should block model download");

    assert_eq!(error, ModelDownloadError::NetworkUnavailable);
}

#[test]
fn model_variant_selects_runtime_download_capability_before_execution() {
    let catalog = ModelManager::new().default_family_catalog();
    let gemma = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.gemma4-12b-q4")
        .expect("Gemma 4 12B should be seeded");
    let future_variant = ModelVariant::new(
        "family.future",
        "Future",
        "model.future-unavailable",
        ModelParameterClass::Large,
        24_000,
        "runtime.future",
        "future:32b",
        "experimental",
    );

    let gemma_plan = ModelDownloadPlan::from_variant(gemma, true);
    assert_eq!(gemma_plan.runtime_id(), &RuntimeId::new("runtime.ollama"));
    assert_eq!(gemma_plan.model_id(), "model.gemma4-12b-q4");
    assert_eq!(gemma_plan.pull_ref(), "gemma4:12b");

    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(100_000));
    let job = executor
        .start(gemma_plan, ModelDownloadExecutionPolicy::resumable())
        .expect("Ollama-compatible variant should select pull adapter");
    assert_eq!(
        job.command(),
        &RuntimeModelDownloadCommand::new("ollama", &["pull", "gemma4:12b"])
    );

    let blocked = executor
        .start(
            ModelDownloadPlan::from_variant(&future_variant, true),
            ModelDownloadExecutionPolicy::resumable(),
        )
        .expect_err("future runtime should be blocked before execution");
    assert_eq!(
        blocked,
        ModelDownloadError::UnsupportedRuntime("runtime.future".to_string())
    );
}

#[test]
fn mlx_lm_variant_selects_python_generate_download_capability() {
    let mlx_variant = ModelVariant::new(
        "family.test-mlx",
        "Test MLX",
        "model.test-mlx-4b",
        ModelParameterClass::Small,
        4_000,
        "runtime.mlx-lm",
        "mlx-community/Qwen-3.5-4B-8bit",
        "test-only",
    );

    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(100_000));
    let job = executor
        .start(
            ModelDownloadPlan::from_variant(&mlx_variant, true),
            ModelDownloadExecutionPolicy::resumable(),
        )
        .expect("MLX-LM-compatible variant should select the Python download adapter");

    assert_eq!(
        job.command(),
        &RuntimeModelDownloadCommand::new(
            "mlx_lm.generate",
            &[
                "--model",
                "mlx-community/Qwen-3.5-4B-8bit",
                "--prompt",
                "DesktopLab model readiness check.",
                "--max-tokens",
                "1",
            ],
        )
    );
}

#[test]
fn ollama_pull_adapter_rejects_unsafe_model_references() {
    let selection =
        SetupSelection::accepted(RuntimeId::new("runtime.ollama"), "qwen:7b; rm -rf /", 256);
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(512));

    let error = executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .expect_err("unsafe pull ref should not create an ollama command");

    assert_eq!(
        error,
        ModelDownloadError::UnsafeModelReference("qwen:7b; rm -rf /".to_string())
    );
}

#[test]
fn model_download_plan_carries_family_variant_size_and_verification_metadata() {
    let catalog = ModelManager::new().default_family_catalog();
    let gemma = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.gemma4-12b-q4")
        .expect("Gemma 4 12B should be seeded");

    let plan = ModelDownloadPlan::from_variant(gemma, true);

    assert_eq!(plan.family_id(), "family.gemma4");
    assert_eq!(plan.variant_id(), "model.gemma4-12b-q4");
    assert_eq!(plan.expected_disk_mb(), 7_600);
    assert_eq!(
        plan.verification(),
        "runtime manifest plus local runtime inventory"
    );
}

#[test]
fn cancellation_can_resume_when_runtime_supports_resume() {
    let selection = SetupSelection::accepted(RuntimeId::new("runtime.ollama"), "qwen3:8b", 1024);
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);
    let executor = ModelDownloadExecutor::new(ModelDownloadCapacity::new(2048));
    let mut job = executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .expect("download should start");

    job.record_progress_mb(384);
    job.cancel();
    let resumed = executor
        .resume(job.metadata())
        .expect("resumable runtime should resume cancelled job");

    assert_eq!(resumed.state(), ModelDownloadState::Running);
    assert_eq!(resumed.progress_mb(), 384);
    assert_eq!(
        resumed.event_names(),
        vec!["queued", "started", "progress", "cancelled", "resumed"]
    );
}

#[test]
fn model_download_execution_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-model-manager/src/download_execution.rs",
        include_str!("../src/download_execution.rs"),
        280,
    )
    .expect("model download execution source should stay below the line-count guard");
}
