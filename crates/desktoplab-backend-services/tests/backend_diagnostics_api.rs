use desktoplab_backend_services::{
    BackendDiagnosticsService, DiagnosticServiceFamily, DiagnosticServiceState,
    DiagnosticsSnapshot, ModelDownloadDiagnosticFailure,
};
use xtask::check_logical_line_limit;

#[test]
fn diagnostics_include_all_backend_service_families() {
    let snapshot = BackendDiagnosticsService::new()
        .with_service(
            DiagnosticServiceFamily::Storage,
            DiagnosticServiceState::Ready,
        )
        .with_service(
            DiagnosticServiceFamily::Registry,
            DiagnosticServiceState::Ready,
        )
        .with_service(
            DiagnosticServiceFamily::Runtime,
            DiagnosticServiceState::Ready,
        )
        .with_service(
            DiagnosticServiceFamily::Model,
            DiagnosticServiceState::Ready,
        )
        .with_service(
            DiagnosticServiceFamily::Session,
            DiagnosticServiceState::Ready,
        )
        .with_service(DiagnosticServiceFamily::Job, DiagnosticServiceState::Ready)
        .snapshot();

    assert!(snapshot.has_family(DiagnosticServiceFamily::Storage));
    assert!(snapshot.has_family(DiagnosticServiceFamily::Registry));
    assert!(snapshot.has_family(DiagnosticServiceFamily::Runtime));
    assert!(snapshot.has_family(DiagnosticServiceFamily::Model));
    assert!(snapshot.has_family(DiagnosticServiceFamily::Session));
    assert!(snapshot.has_family(DiagnosticServiceFamily::Job));
}

#[test]
fn diagnostics_bundle_redacts_secrets() {
    let snapshot = DiagnosticsSnapshot::empty().with_note("token=sk-secret");

    assert_eq!(snapshot.bundle(), "token=[REDACTED]");
}

#[test]
fn degraded_services_include_actionable_reason() {
    let snapshot = BackendDiagnosticsService::new()
        .with_service(
            DiagnosticServiceFamily::Registry,
            DiagnosticServiceState::Degraded("using last known good catalog".to_string()),
        )
        .snapshot();

    assert_eq!(
        snapshot.degraded_reasons(),
        vec!["registry:using last known good catalog".to_string()]
    );
}

#[test]
fn model_download_failures_have_actionable_diagnostics_without_secrets() {
    let snapshot = BackendDiagnosticsService::new()
        .with_model_download_failure(
            "model.qwen-coder-7b",
            ModelDownloadDiagnosticFailure::InsufficientDisk,
            "download failed token=secret",
        )
        .with_model_download_failure(
            "model.deepseek-coder-7b",
            ModelDownloadDiagnosticFailure::NetworkUnavailable,
            "network interrupted api_key=secret",
        )
        .with_model_download_failure(
            "model.glm4-9b",
            ModelDownloadDiagnosticFailure::RuntimeUnavailable,
            "runtime missing",
        )
        .with_model_download_failure(
            "model.nemotron-70b-q4",
            ModelDownloadDiagnosticFailure::VerificationFailed,
            "signature mismatch",
        )
        .snapshot();

    assert_eq!(
        snapshot.degraded_reasons(),
        vec![
            "model:model.qwen-coder-7b insufficient disk for model download".to_string(),
            "model:model.deepseek-coder-7b network unavailable during model download".to_string(),
            "model:model.glm4-9b local runtime unavailable for model download".to_string(),
            "model:model.nemotron-70b-q4 model verification failed".to_string(),
        ]
    );
    assert!(snapshot.bundle().contains("token=[REDACTED]"));
    assert!(snapshot.bundle().contains("api_key=[REDACTED]"));
    assert!(!snapshot.bundle().contains("token=secret"));
    assert!(!snapshot.bundle().contains("api_key=secret"));
}

#[test]
fn diagnostics_bundle_includes_setup_hardware_and_jobs_without_paths() {
    let snapshot = BackendDiagnosticsService::new()
        .with_setup_context(
            "runtime.ollama",
            "model.qwen-coder-7b-q4",
            "model.download:blocked",
        )
        .with_hardware_fact("os", "macos")
        .with_hardware_fact("memory", "48gb")
        .with_model_download_failure(
            "model.qwen-coder-7b-q4",
            ModelDownloadDiagnosticFailure::RuntimeUnavailable,
            "workspace_path=/Users/example/secret/repo local_path=/Users/example/.ssh token=secret",
        )
        .snapshot();

    let bundle = snapshot.bundle();

    assert!(bundle.contains("runtime_id=runtime.ollama"));
    assert!(bundle.contains("model_id=model.qwen-coder-7b-q4"));
    assert!(bundle.contains("hardware os=macos"));
    assert!(bundle.contains("job_state=model.download:blocked"));
    assert!(bundle.contains("workspace_path=[REDACTED]"));
    assert!(bundle.contains("local_path=[REDACTED]"));
    assert!(!bundle.contains("/Users/example"));
    assert!(!bundle.contains("token=secret"));
}

#[test]
fn diagnostics_can_be_generated_offline() {
    let snapshot = BackendDiagnosticsService::offline().snapshot();

    assert!(snapshot.offline());
    assert!(snapshot.bundle().contains("offline=true"));
}

#[test]
fn diagnostics_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/diagnostics.rs",
        include_str!("../src/diagnostics.rs"),
        260,
    )
    .expect("diagnostics source should stay below the line-count guard");
}
