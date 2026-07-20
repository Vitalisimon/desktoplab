use desktoplab_policy::EgressClassification;

use crate::{BackendTrust, ExecutionRouteCandidate};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutePolicy {
    allow_egress: bool,
    allow_unverified: bool,
    allow_silent_fallback: bool,
}

impl RoutePolicy {
    #[must_use]
    pub fn local_only() -> Self {
        Self {
            allow_egress: false,
            allow_unverified: false,
            allow_silent_fallback: false,
        }
    }

    #[must_use]
    pub fn with_visible_fallback_allowed(mut self) -> Self {
        self.allow_silent_fallback = true;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteRequest {
    required_capabilities: Vec<String>,
    preferred_backend: Option<String>,
}

impl RouteRequest {
    #[must_use]
    pub fn new(required_capabilities: &[&str]) -> Self {
        Self {
            required_capabilities: required_capabilities
                .iter()
                .map(ToString::to_string)
                .collect(),
            preferred_backend: None,
        }
    }

    #[must_use]
    pub fn with_preferred_backend(mut self, backend_id: impl Into<String>) -> Self {
        self.preferred_backend = Some(backend_id.into());
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteStatus {
    Selected,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteDecision {
    status: RouteStatus,
    backend_id: Option<String>,
    reasons: Vec<String>,
}

impl RouteDecision {
    #[must_use]
    pub fn status(&self) -> RouteStatus {
        self.status
    }

    #[must_use]
    pub fn backend_id(&self) -> Option<&str> {
        self.backend_id.as_deref()
    }

    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionRouter {
    policy: RoutePolicy,
}

impl ExecutionRouter {
    #[must_use]
    pub fn new(policy: RoutePolicy) -> Self {
        Self { policy }
    }

    #[must_use]
    pub fn select(
        &self,
        request: RouteRequest,
        candidates: Vec<ExecutionRouteCandidate>,
    ) -> RouteDecision {
        let mut reasons = Vec::new();
        let preferred_failed = request.preferred_backend.as_ref().is_some_and(|preferred| {
            candidates
                .iter()
                .any(|candidate| candidate.id() == preferred && !candidate.is_available())
        });

        for candidate in candidates {
            let candidate_reasons = self.blocking_reasons(&request, &candidate);
            if candidate_reasons.is_empty() {
                if preferred_failed && !self.policy.allow_silent_fallback {
                    return blocked(vec!["fallback_requires_visibility_or_approval".into()]);
                }
                return RouteDecision {
                    status: RouteStatus::Selected,
                    backend_id: Some(candidate.id().to_string()),
                    reasons: Vec::new(),
                };
            }
            reasons.extend(candidate_reasons);
        }

        blocked(reasons)
    }

    fn blocking_reasons(
        &self,
        request: &RouteRequest,
        candidate: &ExecutionRouteCandidate,
    ) -> Vec<String> {
        let mut reasons = Vec::new();
        if !candidate.is_available() {
            reasons.push(format!(
                "backend_unavailable:{}",
                candidate.unavailable_reason().unwrap_or("unknown")
            ));
        }
        if let Some(reason) = candidate.model_unavailable_reason() {
            reasons.push(format!("model_unavailable:{reason}"));
        }
        for missing in candidate.missing_capabilities(&request.required_capabilities) {
            reasons.push(format!("missing_capability:{missing}"));
        }
        if candidate.egress() == EgressClassification::SafeToEgress && !self.policy.allow_egress {
            reasons.push("egress_blocked".to_string());
        }
        if candidate.trust() == BackendTrust::Unverified && !self.policy.allow_unverified {
            reasons.push("unverified_backend_blocked".to_string());
        }
        reasons
    }
}

fn blocked(reasons: Vec<String>) -> RouteDecision {
    RouteDecision {
        status: RouteStatus::Blocked,
        backend_id: None,
        reasons,
    }
}
