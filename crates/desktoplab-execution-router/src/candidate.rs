use desktoplab_model_manager::{ModelRouteReadiness, ModelRouteStatus};
use desktoplab_policy::EgressClassification;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendTrust {
    Local,
    Verified,
    Unverified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionRouteCandidate {
    id: String,
    capabilities: Vec<String>,
    egress: EgressClassification,
    trust: BackendTrust,
    available: bool,
    unavailable_reason: Option<String>,
    model_readiness: Option<ModelRouteReadiness>,
}

impl ExecutionRouteCandidate {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            capabilities: Vec::new(),
            egress: EgressClassification::LocalOnly,
            trust: BackendTrust::Local,
            available: true,
            unavailable_reason: None,
            model_readiness: None,
        }
    }

    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    #[must_use]
    pub fn with_egress(mut self, egress: EgressClassification) -> Self {
        self.egress = egress;
        self
    }

    #[must_use]
    pub fn with_trust(mut self, trust: BackendTrust) -> Self {
        self.trust = trust;
        self
    }

    #[must_use]
    pub fn mark_unavailable(mut self, reason: impl Into<String>) -> Self {
        self.available = false;
        self.unavailable_reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn with_model_readiness(mut self, readiness: ModelRouteReadiness) -> Self {
        self.model_readiness = Some(readiness);
        self
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn egress(&self) -> EgressClassification {
        self.egress
    }

    #[must_use]
    pub fn trust(&self) -> BackendTrust {
        self.trust
    }

    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available
    }

    #[must_use]
    pub fn unavailable_reason(&self) -> Option<&str> {
        self.unavailable_reason.as_deref()
    }

    #[must_use]
    pub fn model_unavailable_reason(&self) -> Option<&str> {
        self.model_readiness.as_ref().and_then(|readiness| {
            if readiness.status() == ModelRouteStatus::Blocked {
                Some(readiness.reason().unwrap_or("model_unavailable"))
            } else {
                None
            }
        })
    }

    #[must_use]
    pub fn missing_capabilities(&self, required: &[String]) -> Vec<String> {
        required
            .iter()
            .filter(|required| !self.capabilities.contains(required))
            .cloned()
            .collect()
    }
}
