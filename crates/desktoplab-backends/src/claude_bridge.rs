use desktoplab_agent_session::AgentSession;
use desktoplab_domain::AccountMode;
use desktoplab_execution_router::ExecutionRouteCandidate;

use crate::{
    BridgeCallFailure, ExternalBackendHarness, ExternalBackendManifest, ExternalEvent,
    ImportedBridgeEvents, productization,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeBridgeConfig {
    sdk_name: String,
    auth_mode: AccountMode,
}

impl ClaudeBridgeConfig {
    #[must_use]
    pub fn new(sdk_name: impl Into<String>) -> Self {
        Self::subscription_account(sdk_name)
    }

    #[must_use]
    pub fn subscription_account(sdk_name: impl Into<String>) -> Self {
        Self {
            sdk_name: sdk_name.into(),
            auth_mode: AccountMode::SubscriptionAccount,
        }
    }

    #[must_use]
    pub fn api_key_billing(sdk_name: impl Into<String>) -> Self {
        Self {
            sdk_name: sdk_name.into(),
            auth_mode: AccountMode::ApiKeyBilling,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaudeAgentSdkBridge {
    config: ClaudeBridgeConfig,
    harness: ExternalBackendHarness,
}

impl ClaudeAgentSdkBridge {
    #[must_use]
    pub fn new(config: ClaudeBridgeConfig) -> Self {
        Self {
            config,
            harness: ExternalBackendHarness::new(ExternalBackendManifest::new(
                "backend.claude-agent-sdk",
                &[
                    "llm.chat",
                    "agent.sdk.claude",
                    "agent.events.stream",
                    "approvals.boundary.external",
                ],
            )),
        }
    }

    #[must_use]
    pub fn create_session(&self, session_id: impl Into<String>) -> AgentSession {
        self.harness.create_session(session_id)
    }

    #[must_use]
    pub fn route_candidate(&self) -> ExecutionRouteCandidate {
        self.harness.route_candidate()
    }

    #[must_use]
    pub fn requires_desktoplab_policy(&self) -> bool {
        true
    }

    #[must_use]
    pub fn requires_desktoplab_approval_mapping(&self) -> bool {
        true
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
    pub fn record_failure(
        &self,
        session_id: &str,
        failure: BridgeCallFailure,
    ) -> ImportedBridgeEvents {
        productization::record_failure(&self.harness, session_id, failure)
    }
}
