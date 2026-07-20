use desktoplab_runtime::{
    InMemoryRuntimeInventoryStore, LmStudioEndpointProbe, LmStudioRuntime, LmStudioRuntimeDetector,
    OllamaRuntime, OllamaRuntimeDetector, RuntimeDetectionEventKind, RuntimeDetectionOutcome,
    RuntimeDetectionService, RuntimeDetector, RuntimeId, RuntimeProbe, RuntimeState, RuntimeStatus,
};
use xtask::check_logical_line_limit;

#[test]
fn ollama_detection_uses_runtime_abstraction() {
    let mut service = RuntimeDetectionService::new(InMemoryRuntimeInventoryStore::default());
    service.register_detector(OllamaRuntimeDetector::new(
        OllamaRuntime::new(),
        RuntimeProbe::new()
            .with_binary_path("/usr/local/bin/ollama")
            .with_version("0.9.0")
            .with_model("qwen3:8b"),
    ));

    let report = service.detect_all();
    let status = report
        .status(&RuntimeId::new("runtime.ollama"))
        .expect("ollama status should be present");

    assert_eq!(status.state(), RuntimeState::Installed);
    assert_eq!(status.version(), Some("0.9.0"));
    assert_eq!(
        report.event_kinds(),
        vec![RuntimeDetectionEventKind::RuntimeDetected]
    );
}

#[test]
fn lm_studio_endpoint_unavailable_becomes_degraded_state() {
    let mut service = RuntimeDetectionService::new(InMemoryRuntimeInventoryStore::default());
    service.register_detector(LmStudioRuntimeDetector::new(
        LmStudioRuntime::new(),
        LmStudioEndpointProbe::new("http://127.0.0.1:1234").mark_unavailable("connection refused"),
    ));

    let report = service.detect_all();
    let status = report
        .status(&RuntimeId::new("runtime.lm-studio"))
        .expect("lm studio status should be present");

    assert_eq!(status.state(), RuntimeState::Degraded);
    assert_eq!(status.failure_reason(), Some("connection refused"));
    assert_eq!(
        report.event_kinds(),
        vec![RuntimeDetectionEventKind::RuntimeDegraded]
    );
}

#[test]
fn detection_results_persist_in_runtime_inventory_store() {
    let mut service = RuntimeDetectionService::new(InMemoryRuntimeInventoryStore::default());
    service.register_detector(OllamaRuntimeDetector::new(
        OllamaRuntime::new(),
        RuntimeProbe::new()
            .with_binary_path("/usr/local/bin/ollama")
            .with_version("0.9.0"),
    ));

    service.detect_all();
    let reopened_store = service.store().clone();

    assert_eq!(
        reopened_store
            .load(&RuntimeId::new("runtime.ollama"))
            .expect("stored runtime should reload")
            .version(),
        Some("0.9.0")
    );
}

#[test]
fn detection_service_accepts_custom_detector_without_runtime_specific_branches() {
    let mut service = RuntimeDetectionService::new(InMemoryRuntimeInventoryStore::default());
    service.register_detector(CustomRuntimeDetector);

    let report = service.detect_all();

    assert_eq!(
        report
            .status(&RuntimeId::new("runtime.mlx"))
            .expect("custom detector should be represented")
            .state(),
        RuntimeState::Installed
    );
}

#[test]
fn runtime_detection_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/detection.rs",
        include_str!("../src/detection.rs"),
        280,
    )
    .expect("runtime detection source should stay below the initial line-count guard");
}

struct CustomRuntimeDetector;

impl RuntimeDetector for CustomRuntimeDetector {
    fn detect(&self) -> RuntimeDetectionOutcome {
        RuntimeDetectionOutcome::new(RuntimeStatus::installed(
            RuntimeId::new("runtime.mlx"),
            "MLX",
            "1.0",
        ))
    }
}
