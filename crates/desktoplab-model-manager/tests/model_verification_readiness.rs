use desktoplab_model_manager::{
    InMemoryModelReadinessStore, ModelReadiness, ModelReadinessService, ModelRouteStatus,
    ModelVerification, ModelVerificationReport,
};
use desktoplab_runtime::RuntimeId;
use xtask::check_logical_line_limit;

#[test]
fn verification_failure_blocks_readiness() {
    let mut service = ModelReadinessService::new(InMemoryModelReadinessStore::default());

    let readiness = service.apply_verification(ModelVerificationReport::failed(
        RuntimeId::new("runtime.ollama"),
        "qwen3:8b",
        "checksum mismatch",
    ));

    assert!(!readiness.is_ready());
    assert_eq!(readiness.reason(), Some("checksum mismatch"));
}

#[test]
fn unavailable_model_blocks_execution_route() {
    let service = ModelReadinessService::new(InMemoryModelReadinessStore::default());

    let route = service.route_readiness("runtime.ollama", "qwen3:8b");

    assert_eq!(route.status(), ModelRouteStatus::Blocked);
    assert_eq!(route.reason(), Some("model_unavailable"));
}

#[test]
fn successful_verification_marks_model_ready() {
    let mut service = ModelReadinessService::new(InMemoryModelReadinessStore::default());

    let readiness = service.apply_verification(ModelVerificationReport::passed(
        RuntimeId::new("runtime.ollama"),
        "qwen3:8b",
    ));
    let route = service.route_readiness("runtime.ollama", "qwen3:8b");

    assert!(readiness.is_ready());
    assert_eq!(route.status(), ModelRouteStatus::Ready);
    assert_eq!(route.reason(), None);
}

#[test]
fn runtime_inventory_verifies_exact_or_prefixed_pull_ref() {
    let ready = ModelVerification::from_runtime_inventory(
        "qwen2.5-coder:7b",
        "qwen2.5-coder:7b    5.2 GB\nllama3.1:8b",
    );
    let redacted_runtime_output = ModelVerification::from_runtime_inventory(
        "qwen2.5-coder:7b",
        "NAME ID SIZE MODIFIED qwen2.5-coder:7b dae161e27b0e 4.7 GB 50 seconds ago",
    );
    let missing = ModelVerification::from_runtime_inventory(
        "deepseek-coder:6.7b",
        "qwen2.5-coder:7b    5.2 GB\nllama3.1:8b",
    );

    assert!(ModelReadiness::from_verification(ready).is_ready());
    assert!(ModelReadiness::from_verification(redacted_runtime_output).is_ready());
    assert_eq!(
        ModelReadiness::from_verification(missing).reason(),
        Some("model_not_reported_by_runtime")
    );
}

#[test]
fn readiness_is_persisted_across_service_restart() {
    let store = InMemoryModelReadinessStore::default();
    let mut first_service = ModelReadinessService::new(store.clone());
    first_service.apply_verification(ModelVerificationReport::passed(
        RuntimeId::new("runtime.ollama"),
        "qwen3:8b",
    ));

    let restarted_service = ModelReadinessService::new(store);
    let route = restarted_service.route_readiness("runtime.ollama", "qwen3:8b");

    assert_eq!(route.status(), ModelRouteStatus::Ready);
}

#[test]
fn model_readiness_service_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-model-manager/src/readiness_service.rs",
        include_str!("../src/readiness_service.rs"),
        280,
    )
    .expect("model readiness service source should stay below the line-count guard");
}
