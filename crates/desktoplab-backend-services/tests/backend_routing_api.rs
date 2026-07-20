use desktoplab_backend_services::{
    BackendRouteCandidate, BackendRouteService, BackendRouteStatus, FallbackVisibility,
    RouteApiPolicy, RouteApiRequest,
};
use xtask::check_logical_line_limit;

#[test]
fn routing_api_selects_backend_by_capability_contract() {
    let service = BackendRouteService::new(RouteApiPolicy::local_only());

    let decision = service.plan(
        RouteApiRequest::new(&["llm.chat", "tools.filesystem.read"]),
        vec![BackendRouteCandidate::local(
            "backend.ollama",
            &["llm.chat", "tools.filesystem.read"],
        )],
    );

    assert_eq!(decision.status(), BackendRouteStatus::Selected);
    assert_eq!(decision.backend_id(), Some("backend.ollama"));
    assert!(decision.blocked_reasons().is_empty());
}

#[test]
fn unavailable_model_blocks_route_with_visible_reason() {
    let service = BackendRouteService::new(RouteApiPolicy::local_only());
    let candidate = BackendRouteCandidate::local("backend.ollama", &["llm.chat"])
        .with_model("qwen2.5-coder:7b")
        .mark_model_unavailable("model not downloaded");

    let decision = service.plan(RouteApiRequest::new(&["llm.chat"]), vec![candidate]);

    assert_eq!(decision.status(), BackendRouteStatus::Blocked);
    assert!(decision.blocked_reasons().contains(
        &"backend_unavailable:model_unavailable:qwen2.5-coder:7b:model not downloaded".into()
    ));
}

#[test]
fn fallback_requires_visibility_or_explicit_approval() {
    let service = BackendRouteService::new(RouteApiPolicy::local_only());
    let request = RouteApiRequest::new(&["llm.chat"]).with_preferred_backend("backend.primary");
    let candidates = vec![
        BackendRouteCandidate::local("backend.primary", &["llm.chat"])
            .mark_runtime_unavailable("runtime stopped"),
        BackendRouteCandidate::local("backend.fallback", &["llm.chat"]),
    ];

    let hidden = service.plan(request.clone(), candidates.clone());
    let visible = service.plan(
        request.with_fallback_visibility(FallbackVisibility::Approved),
        candidates,
    );

    assert_eq!(hidden.status(), BackendRouteStatus::Blocked);
    assert!(
        hidden
            .blocked_reasons()
            .contains(&"fallback_requires_visibility_or_approval".into())
    );
    assert_eq!(visible.status(), BackendRouteStatus::Selected);
    assert_eq!(visible.backend_id(), Some("backend.fallback"));
    assert!(
        visible
            .explanations()
            .contains(&"fallback_visible_or_approved".into())
    );
}

#[test]
fn route_explanations_include_policy_and_trust_blocks() {
    let service = BackendRouteService::new(RouteApiPolicy::local_only());
    let candidate =
        BackendRouteCandidate::cloud("backend.community", &["llm.chat"]).mark_unverified();

    let decision = service.plan(RouteApiRequest::new(&["llm.chat"]), vec![candidate]);

    assert_eq!(decision.status(), BackendRouteStatus::Blocked);
    assert!(
        decision
            .blocked_reasons()
            .contains(&"egress_blocked".into())
    );
    assert!(
        decision
            .blocked_reasons()
            .contains(&"unverified_backend_blocked".into())
    );
    assert!(
        decision
            .explanations()
            .contains(&"local_only_policy".into())
    );
}

#[test]
fn routing_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/routing.rs",
        include_str!("../src/routing.rs"),
        280,
    )
    .expect("routing api source should stay below the line-count guard");
}
