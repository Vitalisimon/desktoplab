use desktoplab_domain::{
    BackendCapability, ExecutionBackend, ExecutionBackendId, ExecutionBackendKind, Provider,
    ProviderId, ProviderKind,
};
use desktoplab_execution_router::{
    BackendTrust, ExecutionRouteCandidate, ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus,
};
use desktoplab_model_manager::{
    InMemoryModelReadinessStore, ModelReadinessService, ModelVerificationReport,
};
use desktoplab_policy::EgressClassification;
use desktoplab_runtime::RuntimeId;
use xtask::check_logical_line_limit;

#[test]
fn routing_uses_capabilities_not_vendor_branches() {
    let router = ExecutionRouter::new(RoutePolicy::local_only());
    let route = router.select(
        RouteRequest::new(&["llm.chat", "agent.events.stream"]),
        vec![candidate(
            "backend.local",
            &["llm.chat", "agent.events.stream"],
        )],
    );

    assert_eq!(route.status(), RouteStatus::Selected);
    assert_eq!(route.backend_id(), Some("backend.local"));
}

#[test]
fn blocked_routes_include_machine_readable_reasons() {
    let router = ExecutionRouter::new(RoutePolicy::local_only());
    let route = router.select(
        RouteRequest::new(&["tools.filesystem.write"]),
        vec![candidate("backend.chat-only", &["llm.chat"])],
    );

    assert_eq!(route.status(), RouteStatus::Blocked);
    assert!(
        route
            .reasons()
            .contains(&"missing_capability:tools.filesystem.write".to_string())
    );
}

#[test]
fn provider_egress_and_trust_filter_cloud_routes() {
    let router = ExecutionRouter::new(RoutePolicy::local_only());
    let route = router.select(
        RouteRequest::new(&["llm.chat"]),
        vec![
            candidate("backend.cloud", &["llm.chat"])
                .with_egress(EgressClassification::SafeToEgress)
                .with_trust(BackendTrust::Verified),
        ],
    );

    assert_eq!(route.status(), RouteStatus::Blocked);
    assert!(route.reasons().contains(&"egress_blocked".to_string()));
}

#[test]
fn fallback_requires_visibility_or_approval() {
    let router = ExecutionRouter::new(RoutePolicy::local_only());
    let route = router.select(
        RouteRequest::new(&["llm.chat"]).with_preferred_backend("backend.primary"),
        vec![
            candidate("backend.primary", &["llm.chat"]).mark_unavailable("runtime stopped"),
            candidate("backend.fallback", &["llm.chat"]),
        ],
    );

    assert_eq!(route.status(), RouteStatus::Blocked);
    assert!(
        route
            .reasons()
            .contains(&"fallback_requires_visibility_or_approval".to_string())
    );
}

#[test]
fn local_model_route_requires_downloaded_ready_model() {
    let router = ExecutionRouter::new(RoutePolicy::local_only());
    let readiness = ModelReadinessService::new(InMemoryModelReadinessStore::default());

    let blocked = router.select(
        RouteRequest::new(&["llm.chat"]),
        vec![
            candidate("backend.ollama", &["llm.chat"]).with_model_readiness(
                readiness.route_readiness("runtime.ollama", "model.qwen-coder-7b"),
            ),
        ],
    );

    assert_eq!(blocked.status(), RouteStatus::Blocked);
    assert!(
        blocked
            .reasons()
            .contains(&"model_unavailable:model_unavailable".to_string())
    );

    let mut readiness = ModelReadinessService::new(InMemoryModelReadinessStore::default());
    readiness.apply_verification(ModelVerificationReport::passed(
        RuntimeId::new("runtime.ollama"),
        "model.qwen-coder-7b",
    ));
    let selected = router.select(
        RouteRequest::new(&["llm.chat"]),
        vec![
            candidate("backend.ollama", &["llm.chat"]).with_model_readiness(
                readiness.route_readiness("runtime.ollama", "model.qwen-coder-7b"),
            ),
        ],
    );

    assert_eq!(selected.status(), RouteStatus::Selected);
    assert_eq!(selected.backend_id(), Some("backend.ollama"));
}

#[test]
fn provider_and_execution_backend_remain_separate_concepts() {
    let provider = Provider::new(
        ProviderId::new("provider.openai"),
        "OpenAI",
        ProviderKind::CloudAccount,
    );
    let backend = ExecutionBackend::new(
        ExecutionBackendId::new("backend.codex"),
        "Codex",
        ExecutionBackendKind::ExternalAgent,
        Some(provider.id().clone()),
    )
    .with_capability(BackendCapability::new("llm.chat"));

    assert_eq!(provider.kind(), ProviderKind::CloudAccount);
    assert_eq!(backend.kind(), ExecutionBackendKind::ExternalAgent);
    assert_eq!(backend.provider_id(), Some(provider.id()));
}

#[test]
fn execution_router_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-execution-router/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-execution-router/src/candidate.rs",
            include_str!("../src/candidate.rs"),
            250,
        ),
        (
            "crates/desktoplab-execution-router/src/router.rs",
            include_str!("../src/router.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("execution router source should stay below the initial line-count guard");
    }
}

fn candidate(id: &str, capabilities: &[&str]) -> ExecutionRouteCandidate {
    let mut candidate = ExecutionRouteCandidate::new(id);
    for capability in capabilities {
        candidate = candidate.with_capability(*capability);
    }
    candidate
}
