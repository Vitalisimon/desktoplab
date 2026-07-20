use desktoplab_backend_services::{
    CatalogChannel, CatalogRefreshRequestState, SetupCatalogEntry, SetupWizardApiService,
    SetupWizardPolicy, SetupWizardRegistryState,
};
use desktoplab_hardware_wizard::{ProbeSnapshot, WarningCode};
use xtask::check_logical_line_limit;

#[test]
fn setup_recommendations_are_data_driven_from_catalog_entries() {
    let service = SetupWizardApiService::new();
    let preview = service.preview(
        host_snapshot(),
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::stable_only(),
        vec![
            SetupCatalogEntry::runtime("runtime.ollama", "Ollama", CatalogChannel::Stable),
            SetupCatalogEntry::model("model.qwen-coder", "Qwen Coder", CatalogChannel::Stable)
                .for_runtime("runtime.ollama"),
        ],
    );

    assert!(preview.is_ready());
    assert_eq!(
        preview.runtime_recommendations()[0].manifest_id(),
        "runtime.ollama"
    );
    assert_eq!(
        preview.model_recommendations()[0].manifest_id(),
        "model.qwen-coder"
    );
}

#[test]
fn hardware_warnings_and_expected_limitations_propagate_to_api_response() {
    let service = SetupWizardApiService::new();
    let preview = service.preview(
        ProbeSnapshot::new()
            .with_operating_system("linux")
            .with_architecture("x86_64")
            .with_cpu("low power cpu")
            .with_ram_gb(8)
            .with_storage_available_gb(24),
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::stable_only(),
        vec![SetupCatalogEntry::runtime(
            "runtime.ollama",
            "Ollama",
            CatalogChannel::Stable,
        )],
    );

    assert!(preview.warnings().contains(&WarningCode::LimitedMemory));
    assert!(preview.warnings().contains(&WarningCode::LowStorage));
    assert!(
        preview.expected_limitations().contains(
            &"small local models or cloud/external backends are more realistic".to_string()
        )
    );
}

#[test]
fn beta_and_experimental_entries_require_explicit_channel_policy() {
    let service = SetupWizardApiService::new();
    let entries = vec![
        SetupCatalogEntry::runtime("runtime.stable", "Stable Runtime", CatalogChannel::Stable),
        SetupCatalogEntry::runtime("runtime.beta", "Beta Runtime", CatalogChannel::Beta),
        SetupCatalogEntry::runtime(
            "runtime.experimental",
            "Experimental Runtime",
            CatalogChannel::Experimental,
        ),
    ];

    let stable = service.preview(
        host_snapshot(),
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::stable_only(),
        entries.clone(),
    );
    let beta = service.preview(
        host_snapshot(),
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::allow_beta(),
        entries,
    );

    assert_eq!(stable.runtime_recommendations().len(), 1);
    assert!(
        stable
            .hidden_reasons()
            .contains(&"runtime.beta:hidden_channel:beta".to_string())
    );
    assert_eq!(beta.runtime_recommendations().len(), 2);
    assert!(
        beta.hidden_reasons()
            .contains(&"runtime.experimental:hidden_channel:experimental".to_string())
    );
}

#[test]
fn accepting_setup_plan_starts_runtime_and_model_jobs() {
    let service = SetupWizardApiService::new();
    let preview = service.preview(
        host_snapshot(),
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::stable_only(),
        vec![
            SetupCatalogEntry::runtime("runtime.ollama", "Ollama", CatalogChannel::Stable),
            SetupCatalogEntry::model("model.qwen-coder", "Qwen Coder", CatalogChannel::Stable)
                .for_runtime("runtime.ollama"),
        ],
    );

    let acceptance = service.accept(preview);

    assert_eq!(
        acceptance.started_job_ids(),
        &[
            "runtime.install:runtime.ollama".to_string(),
            "model.download:model.qwen-coder".to_string(),
        ]
    );
}

#[test]
fn degraded_catalog_refresh_status_keeps_last_known_good_setup_available() {
    let service = SetupWizardApiService::new();
    let preview = service.preview(
        host_snapshot(),
        SetupWizardRegistryState::Degraded,
        SetupWizardPolicy::stable_only(),
        vec![SetupCatalogEntry::runtime(
            "runtime.ollama",
            "Ollama",
            CatalogChannel::Stable,
        )],
    );

    let status = service.catalog_refresh_status(
        SetupWizardRegistryState::Degraded,
        true,
        vec!["Using last-known-good runtime catalog.".to_string()],
    );

    assert!(preview.is_ready());
    assert!(status.last_known_good_available);
    assert_eq!(status.state, SetupWizardRegistryState::Degraded);
    assert_eq!(
        status.degraded_reasons,
        vec!["Using last-known-good runtime catalog.".to_string()]
    );
    assert_eq!(
        status.manual_refresh.job_id,
        Some("registry.refresh.manual".to_string())
    );
}

#[test]
fn manual_catalog_refresh_returns_job_id_or_blocked_reason() {
    let service = SetupWizardApiService::new();

    let queued = service.request_catalog_refresh(CatalogRefreshRequestState::Available);
    let blocked = service.request_catalog_refresh(CatalogRefreshRequestState::BlockedNoSafeCatalog);

    assert_eq!(queued.job_id, Some("registry.refresh.manual".to_string()));
    assert_eq!(queued.blocked_reason, None);
    assert_eq!(blocked.job_id, None);
    assert_eq!(
        blocked.blocked_reason,
        Some("No safe compatibility catalog is available.".to_string())
    );
}

#[test]
fn setup_wizard_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/setup_wizard.rs",
        include_str!("../src/setup_wizard.rs"),
        320,
    )
    .expect("setup wizard api source should stay below the line-count guard");
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
