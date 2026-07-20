use desktoplab_runtime::{
    InstallPlan, RuntimeCommand, RuntimeId, RuntimeLifecycleBoundary, RuntimeLifecycleState,
    RuntimeManager, RuntimeState, RuntimeStatus, VerificationResult,
};
use xtask::check_logical_line_limit;

#[test]
fn runtime_state_transitions_are_explicit_and_auditable() {
    let mut manager = RuntimeManager::new();
    let runtime_id = RuntimeId::new("runtime.llama-cpp");
    manager.register_runtime(runtime_id.clone(), "llama.cpp");

    assert_eq!(
        manager.status(&runtime_id).state(),
        RuntimeState::NotInstalled
    );

    manager.apply(RuntimeCommand::mark_installed(runtime_id.clone(), "0.3.1"));
    assert_eq!(manager.status(&runtime_id).state(), RuntimeState::Installed);

    manager.apply(RuntimeCommand::start(runtime_id.clone()));
    assert_eq!(manager.status(&runtime_id).state(), RuntimeState::Running);

    manager.apply(RuntimeCommand::stop(runtime_id.clone()));
    assert_eq!(manager.status(&runtime_id).state(), RuntimeState::Stopped);
    assert_eq!(manager.audit_log().len(), 3);
}

#[test]
fn install_plans_are_explainable_and_downloaded_on_demand() {
    let plan = InstallPlan::new(RuntimeId::new("runtime.vllm"), "vLLM")
        .with_step("download signed installer")
        .with_step("verify checksum")
        .with_step("install runtime")
        .with_requirement("network")
        .with_requirement("disk.available_gb >= 8");

    assert_eq!(plan.runtime_name(), "vLLM");
    assert!(!plan.is_bundled());
    assert!(plan.explanation().contains("download signed installer"));
    assert!(plan.explanation().contains("disk.available_gb >= 8"));
}

#[test]
fn verification_contract_blocks_readiness_until_health_is_confirmed() {
    let mut status = RuntimeStatus::installed(RuntimeId::new("runtime.mlx"), "MLX", "1.0");

    status.apply_verification(VerificationResult::failed("health endpoint unavailable"));
    assert_eq!(status.state(), RuntimeState::VerificationFailed);
    assert!(!status.is_ready());

    status.apply_verification(VerificationResult::passed());
    assert_eq!(status.state(), RuntimeState::Ready);
    assert!(status.is_ready());
}

#[test]
fn runtime_abstraction_does_not_encode_ollama_only_assumptions() {
    let mut manager = RuntimeManager::new();
    for runtime in ["runtime.ollama", "runtime.lm-studio", "runtime.mlx"] {
        manager.register_runtime(RuntimeId::new(runtime), runtime);
    }

    assert_eq!(manager.inventory().len(), 3);
    assert!(manager.status(&RuntimeId::new("runtime.mlx")).exists());
    assert!(
        manager
            .status(&RuntimeId::new("runtime.lm-studio"))
            .exists()
    );
}

#[test]
fn runtime_lifecycle_boundaries_are_explicit_before_packaging() {
    let mut manager = RuntimeManager::new();
    let ollama = RuntimeId::new("runtime.ollama");
    let lm_studio = RuntimeId::new("runtime.lm-studio");
    manager.register_runtime(ollama.clone(), "Ollama");
    manager.register_runtime(lm_studio.clone(), "LM Studio");

    manager.set_lifecycle(
        &ollama,
        RuntimeLifecycleBoundary::packaging_managed(
            "Runtime updates are handled by the desktop installer.",
        ),
        RuntimeLifecycleBoundary::packaging_managed(
            "Runtime removal is handled by the desktop installer.",
        ),
    );
    manager.set_lifecycle(
        &lm_studio,
        RuntimeLifecycleBoundary::blocked("Managed outside DesktopLab."),
        RuntimeLifecycleBoundary::blocked("Remove LM Studio from its own app."),
    );

    assert_eq!(
        manager.status(&ollama).update_lifecycle().state(),
        RuntimeLifecycleState::PackagingManaged
    );
    assert_eq!(
        manager.status(&lm_studio).uninstall_lifecycle().state(),
        RuntimeLifecycleState::Blocked
    );
}

#[test]
fn runtime_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-runtime/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-runtime/src/install.rs",
            include_str!("../src/install.rs"),
            250,
        ),
        (
            "crates/desktoplab-runtime/src/manager.rs",
            include_str!("../src/manager.rs"),
            250,
        ),
        (
            "crates/desktoplab-runtime/src/status.rs",
            include_str!("../src/status.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("runtime source should stay below the initial line-count guard");
    }
}
