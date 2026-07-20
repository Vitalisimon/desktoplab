use desktoplab_agent_session::{AgentSession, SessionEvent};
use desktoplab_execution_router::{BackendTrust, ExecutionRouteCandidate};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalBackendManifest {
    backend_id: String,
    capabilities: Vec<String>,
}

impl ExternalBackendManifest {
    #[must_use]
    pub fn new(backend_id: impl Into<String>, capabilities: &[&str]) -> Self {
        Self {
            backend_id: backend_id.into(),
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
        }
    }

    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExternalEvent {
    TextDelta(String),
    Completed(String),
}

impl ExternalEvent {
    #[must_use]
    pub fn text_delta(delta: impl Into<String>) -> Self {
        Self::TextDelta(delta.into())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalBackendHarness {
    manifest: ExternalBackendManifest,
}

impl ExternalBackendHarness {
    #[must_use]
    pub fn new(manifest: ExternalBackendManifest) -> Self {
        Self { manifest }
    }

    #[must_use]
    pub fn create_session(&self, session_id: impl Into<String>) -> AgentSession {
        AgentSession::new(session_id, self.manifest.backend_id())
    }

    #[must_use]
    pub fn backend_id(&self) -> &str {
        self.manifest.backend_id()
    }

    #[must_use]
    pub fn normalize_event(&self, event: ExternalEvent) -> SessionEvent {
        match event {
            ExternalEvent::TextDelta(delta) => SessionEvent::planning_started(delta),
            ExternalEvent::Completed(summary) => SessionEvent::completed(summary),
        }
    }

    #[must_use]
    pub fn route_candidate(&self) -> ExecutionRouteCandidate {
        let mut candidate = ExecutionRouteCandidate::new(self.manifest.backend_id())
            .with_trust(BackendTrust::Verified);
        for capability in &self.manifest.capabilities {
            candidate = candidate.with_capability(capability);
        }
        candidate
    }
}
