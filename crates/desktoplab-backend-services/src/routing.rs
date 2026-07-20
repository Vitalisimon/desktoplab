use desktoplab_execution_router::{
    BackendTrust, ExecutionRouteCandidate, ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus,
};
use desktoplab_policy::EgressClassification;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendRouteStatus {
    Selected,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FallbackVisibility {
    Hidden,
    Approved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteApiPolicy {
    local_only: bool,
}

impl RouteApiPolicy {
    #[must_use]
    pub fn local_only() -> Self {
        Self { local_only: true }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteApiRequest {
    required_capabilities: Vec<String>,
    preferred_backend: Option<String>,
    fallback_visibility: FallbackVisibility,
}

impl RouteApiRequest {
    #[must_use]
    pub fn new(required_capabilities: &[&str]) -> Self {
        Self {
            required_capabilities: required_capabilities
                .iter()
                .map(ToString::to_string)
                .collect(),
            preferred_backend: None,
            fallback_visibility: FallbackVisibility::Hidden,
        }
    }

    #[must_use]
    pub fn with_preferred_backend(mut self, backend_id: impl Into<String>) -> Self {
        self.preferred_backend = Some(backend_id.into());
        self
    }

    #[must_use]
    pub fn with_fallback_visibility(mut self, visibility: FallbackVisibility) -> Self {
        self.fallback_visibility = visibility;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendRouteCandidate {
    id: String,
    capabilities: Vec<String>,
    egress: EgressClassification,
    trust: BackendTrust,
    unavailable_reason: Option<String>,
    model_id: Option<String>,
}

impl BackendRouteCandidate {
    #[must_use]
    pub fn local(id: impl Into<String>, capabilities: &[&str]) -> Self {
        Self::new(id, capabilities, EgressClassification::LocalOnly)
    }

    #[must_use]
    pub fn cloud(id: impl Into<String>, capabilities: &[&str]) -> Self {
        Self::new(id, capabilities, EgressClassification::SafeToEgress)
    }

    #[must_use]
    pub fn with_model(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = Some(model_id.into());
        self
    }

    #[must_use]
    pub fn mark_model_unavailable(mut self, reason: impl Into<String>) -> Self {
        let model_id = self.model_id.as_deref().unwrap_or("unknown-model");
        self.unavailable_reason = Some(format!("model_unavailable:{model_id}:{}", reason.into()));
        self
    }

    #[must_use]
    pub fn mark_runtime_unavailable(mut self, reason: impl Into<String>) -> Self {
        self.unavailable_reason = Some(format!("runtime_unavailable:{}", reason.into()));
        self
    }

    #[must_use]
    pub fn mark_unverified(mut self) -> Self {
        self.trust = BackendTrust::Unverified;
        self
    }

    fn new(id: impl Into<String>, capabilities: &[&str], egress: EgressClassification) -> Self {
        Self {
            id: id.into(),
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
            egress,
            trust: BackendTrust::Local,
            unavailable_reason: None,
            model_id: None,
        }
    }

    fn into_router_candidate(self) -> ExecutionRouteCandidate {
        let mut candidate = ExecutionRouteCandidate::new(self.id)
            .with_egress(self.egress)
            .with_trust(self.trust);
        for capability in self.capabilities {
            candidate = candidate.with_capability(capability);
        }
        if let Some(reason) = self.unavailable_reason {
            candidate = candidate.mark_unavailable(reason);
        }
        candidate
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendRouteDecision {
    status: BackendRouteStatus,
    backend_id: Option<String>,
    blocked_reasons: Vec<String>,
    explanations: Vec<String>,
}

impl BackendRouteDecision {
    #[must_use]
    pub fn status(&self) -> BackendRouteStatus {
        self.status
    }

    #[must_use]
    pub fn backend_id(&self) -> Option<&str> {
        self.backend_id.as_deref()
    }

    #[must_use]
    pub fn blocked_reasons(&self) -> &[String] {
        &self.blocked_reasons
    }

    #[must_use]
    pub fn explanations(&self) -> &[String] {
        &self.explanations
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendRouteService {
    policy: RouteApiPolicy,
}

impl BackendRouteService {
    #[must_use]
    pub fn new(policy: RouteApiPolicy) -> Self {
        Self { policy }
    }

    #[must_use]
    pub fn plan(
        &self,
        request: RouteApiRequest,
        candidates: Vec<BackendRouteCandidate>,
    ) -> BackendRouteDecision {
        let router_request = build_router_request(&request);
        let router_policy = build_router_policy(&request);
        let router = ExecutionRouter::new(router_policy);
        let route = router.select(
            router_request,
            candidates
                .into_iter()
                .map(BackendRouteCandidate::into_router_candidate)
                .collect(),
        );
        let mut explanations = Vec::new();
        if route
            .reasons()
            .iter()
            .any(|reason| reason == "egress_blocked")
            && self.policy.local_only
        {
            explanations.push("local_only_policy".to_string());
        }
        if selected_visible_fallback(&request, route.backend_id()) {
            explanations.push("fallback_visible_or_approved".to_string());
        }
        BackendRouteDecision {
            status: map_status(route.status()),
            backend_id: route.backend_id().map(ToString::to_string),
            blocked_reasons: expand_blocked_reasons(route.reasons()),
            explanations,
        }
    }
}

fn build_router_request(request: &RouteApiRequest) -> RouteRequest {
    let refs = request
        .required_capabilities
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut router_request = RouteRequest::new(&refs);
    if let Some(preferred) = &request.preferred_backend {
        router_request = router_request.with_preferred_backend(preferred);
    }
    router_request
}

fn build_router_policy(request: &RouteApiRequest) -> RoutePolicy {
    let policy = RoutePolicy::local_only();
    match request.fallback_visibility {
        FallbackVisibility::Hidden => policy,
        FallbackVisibility::Approved => policy.with_visible_fallback_allowed(),
    }
}

fn selected_visible_fallback(request: &RouteApiRequest, backend_id: Option<&str>) -> bool {
    request.fallback_visibility == FallbackVisibility::Approved
        && request
            .preferred_backend
            .as_deref()
            .zip(backend_id)
            .is_some_and(|(preferred, selected)| preferred != selected)
}

fn map_status(status: RouteStatus) -> BackendRouteStatus {
    match status {
        RouteStatus::Selected => BackendRouteStatus::Selected,
        RouteStatus::Blocked => BackendRouteStatus::Blocked,
    }
}

fn expand_blocked_reasons(reasons: &[String]) -> Vec<String> {
    let mut expanded = reasons.to_vec();
    for reason in reasons {
        if let Some(unavailable_reason) = reason.strip_prefix("backend_unavailable:") {
            expanded.push(unavailable_reason.to_string());
            if let Some(plain_reason) = plain_model_unavailable_reason(unavailable_reason) {
                expanded.push(plain_reason.to_string());
            }
        } else if let Some(plain_reason) = plain_model_unavailable_reason(reason) {
            expanded.push(plain_reason.to_string());
        }
    }
    expanded
}

fn plain_model_unavailable_reason(reason: &str) -> Option<&str> {
    let payload = reason.strip_prefix("model_unavailable:")?;
    payload.rsplit_once(':').map(|(_, plain)| plain)
}
