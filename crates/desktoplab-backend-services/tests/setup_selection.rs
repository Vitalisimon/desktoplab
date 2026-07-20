use desktoplab_backend_services::{
    CatalogChannel, SetupCatalogEntry, SetupPlanSelection, SetupWizardApiService,
    SetupWizardPolicy, SetupWizardRegistryState,
};
use desktoplab_hardware_wizard::ProbeSnapshot;
use xtask::check_logical_line_limit;

#[test]
fn setup_acceptance_starts_only_selected_runtime_and_model_jobs() {
    let service = SetupWizardApiService::new();
    let preview = service.preview(
        host_snapshot(),
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::stable_only(),
        vec![
            SetupCatalogEntry::runtime("runtime.ollama", "Ollama", CatalogChannel::Stable),
            SetupCatalogEntry::runtime("runtime.lm-studio", "LM Studio", CatalogChannel::Stable),
            SetupCatalogEntry::model(
                "model.qwen-coder-7b-q4",
                "Qwen Coder",
                CatalogChannel::Stable,
            )
            .for_runtime("runtime.ollama"),
            SetupCatalogEntry::model(
                "model.deepseek-coder-7b-q4",
                "DeepSeek",
                CatalogChannel::Stable,
            )
            .for_runtime("runtime.ollama"),
        ],
    );

    assert_eq!(preview.recommended_runtime_id(), Some("runtime.ollama"));
    assert_eq!(
        preview.recommended_model_id(),
        Some("model.qwen-coder-7b-q4")
    );
    assert_eq!(
        preview.alternative_model_ids(),
        vec!["model.deepseek-coder-7b-q4"]
    );

    let acceptance = service.accept_selected(
        &preview,
        SetupPlanSelection::new("runtime.ollama", Some("model.qwen-coder-7b-q4")),
    );

    assert_eq!(
        acceptance.started_job_ids(),
        &[
            "runtime.install:runtime.ollama".to_string(),
            "model.download:model.qwen-coder-7b-q4".to_string(),
        ]
    );
}

#[test]
fn setup_selection_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/setup_selection.rs",
        include_str!("../src/setup_selection.rs"),
        180,
    )
    .expect("setup selection source should stay focused");
}

fn host_snapshot() -> ProbeSnapshot {
    ProbeSnapshot::new()
        .with_operating_system("macOS")
        .with_architecture("arm64")
        .with_cpu("Apple M4 Pro")
        .with_ram_gb(48)
        .with_unified_memory_gb(48)
        .with_storage_available_gb(900)
}
