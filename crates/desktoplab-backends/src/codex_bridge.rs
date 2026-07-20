use desktoplab_agent_session::AgentSession;
use desktoplab_domain::{AccountMode, ExecutionBackendKind, ProviderId};

use crate::{
    ExternalBackendHarness, ExternalBackendManifest, ExternalEvent, ImportedBridgeEvents,
    productization,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexBridgeConfig {
    endpoint: String,
    provider_id: Option<ProviderId>,
    auth_mode: AccountMode,
}

impl CodexBridgeConfig {
    #[must_use]
    pub fn local(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            provider_id: None,
            auth_mode: AccountMode::LocalAppSession,
        }
    }

    #[must_use]
    pub fn with_provider(endpoint: impl Into<String>, provider_id: ProviderId) -> Self {
        Self::with_provider_auth_mode(endpoint, provider_id, AccountMode::SubscriptionAccount)
    }

    #[must_use]
    pub fn with_provider_auth_mode(
        endpoint: impl Into<String>,
        provider_id: ProviderId,
        auth_mode: AccountMode,
    ) -> Self {
        Self {
            endpoint: endpoint.into(),
            provider_id: Some(provider_id),
            auth_mode,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexAppServerBridge {
    config: CodexBridgeConfig,
    harness: ExternalBackendHarness,
}

impl CodexAppServerBridge {
    #[must_use]
    pub fn new(config: CodexBridgeConfig) -> Self {
        Self {
            config,
            harness: ExternalBackendHarness::new(ExternalBackendManifest::new(
                "backend.codex-app-server",
                &[
                    "llm.chat",
                    "agent.events.stream",
                    "approvals.boundary.external",
                ],
            )),
        }
    }

    #[must_use]
    pub fn backend_id(&self) -> &str {
        "backend.codex-app-server"
    }

    #[must_use]
    pub fn backend_kind(&self) -> ExecutionBackendKind {
        ExecutionBackendKind::ExternalAgent
    }

    #[must_use]
    pub fn create_session(&self, session_id: impl Into<String>) -> AgentSession {
        self.harness.create_session(session_id)
    }

    #[must_use]
    pub fn requires_desktoplab_policy(&self) -> bool {
        true
    }

    #[must_use]
    pub fn provider_id(&self) -> Option<&ProviderId> {
        self.config.provider_id.as_ref()
    }

    #[must_use]
    pub fn auth_mode(&self) -> AccountMode {
        self.config.auth_mode
    }

    #[must_use]
    pub fn import_events(
        &self,
        session_id: &str,
        events: Vec<ExternalEvent>,
    ) -> ImportedBridgeEvents {
        productization::import_events(&self.harness, session_id, events)
    }

    #[must_use]
    pub fn redacted_auth_diagnostic(&self, authorization: &str) -> String {
        format!(
            "backend={} authorization={}",
            self.backend_id(),
            productization::redact_auth(authorization)
        )
    }
}
