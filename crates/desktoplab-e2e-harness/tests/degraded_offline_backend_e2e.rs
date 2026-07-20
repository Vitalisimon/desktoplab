use desktoplab_backend_services::{
    BackendDiagnosticsService, BackendRouteCandidate, BackendRouteService, BackendRouteStatus,
    CatalogChannel, DiagnosticServiceFamily, DiagnosticServiceState, RouteApiPolicy,
    RouteApiRequest, SetupCatalogEntry, SetupWizardApiService, SetupWizardPolicy,
    SetupWizardRegistryState,
};
use desktoplab_hardware_wizard::{ProbeSnapshot, WarningCode};
use xtask::check_logical_line_limit;

#[test]
fn degraded_offline_path_uses_safe_last_known_good_catalog() {
    let wizard = SetupWizardApiService::new();
    let preview = wizard.preview(
        partial_hardware_snapshot(),
        SetupWizardRegistryState::Degraded,
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
    assert!(
        preview
            .warnings()
            .contains(&WarningCode::GpuProbeUnavailable)
    );
    assert!(
        preview
            .warnings()
            .contains(&WarningCode::VramProbeUnavailable)
    );
    assert!(preview.expected_limitations().contains(
        &"compatibility catalog refresh unavailable; using last-known-good catalog".to_string()
    ));
}

#[test]
fn degraded_offline_path_blocks_setup_without_safe_catalog() {
    let wizard = SetupWizardApiService::new();
    let preview = wizard.preview(
        partial_hardware_snapshot(),
        SetupWizardRegistryState::Blocked,
        SetupWizardPolicy::stable_only(),
        vec![SetupCatalogEntry::runtime(
            "runtime.ollama",
            "Ollama",
            CatalogChannel::Stable,
        )],
    );

    assert!(!preview.is_ready());
    assert!(
        preview
            .expected_limitations()
            .contains(&"no safe compatibility catalog is available".to_string())
    );
}

#[test]
fn degraded_offline_path_surfaces_runtime_and_provider_unavailability() {
    let diagnostics = BackendDiagnosticsService::offline()
        .with_service(
            DiagnosticServiceFamily::Registry,
            DiagnosticServiceState::Degraded("refresh_unavailable".to_string()),
        )
        .with_service(
            DiagnosticServiceFamily::Runtime,
            DiagnosticServiceState::Degraded("ollama_not_running".to_string()),
        )
        .snapshot();

    assert!(diagnostics.offline());
    assert!(
        diagnostics
            .degraded_reasons()
            .contains(&"registry:refresh_unavailable".to_string())
    );
    assert!(
        diagnostics
            .degraded_reasons()
            .contains(&"runtime:ollama_not_running".to_string())
    );

    let route = BackendRouteService::new(RouteApiPolicy::local_only()).plan(
        RouteApiRequest::new(&["llm.chat"]),
        vec![
            BackendRouteCandidate::local("backend.ollama", &["llm.chat"])
                .mark_runtime_unavailable("ollama_not_running"),
            BackendRouteCandidate::cloud("backend.openai", &["llm.chat"]),
        ],
    );

    assert_eq!(route.status(), BackendRouteStatus::Blocked);
    assert!(
        route
            .blocked_reasons()
            .contains(&"runtime_unavailable:ollama_not_running".to_string())
    );
    assert!(
        route
            .blocked_reasons()
            .contains(&"egress_blocked".to_string())
    );
    assert!(
        route
            .explanations()
            .contains(&"local_only_policy".to_string())
    );
}

#[test]
fn degraded_offline_backend_e2e_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/tests/degraded_offline_backend_e2e.rs",
        include_str!("degraded_offline_backend_e2e.rs"),
        180,
    )
    .expect("degraded offline backend e2e source should stay below the line-count guard");
}

fn partial_hardware_snapshot() -> ProbeSnapshot {
    ProbeSnapshot::new()
        .with_operating_system("linux")
        .with_architecture("x86_64")
        .with_cpu("offline host")
        .with_ram_gb(16)
        .with_storage_available_gb(120)
}
