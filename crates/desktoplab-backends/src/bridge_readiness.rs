use crate::{ClaudeAgentSdkBridge, CodexAppServerBridge};
use desktoplab_domain::AccountMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeStatus {
    Ready,
    Blocked,
    OptionalUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BridgeFailureCode {
    SdkUnavailable,
    PluginMissing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeReadinessProbe {
    available: bool,
    detail: String,
}

impl BridgeReadinessProbe {
    #[must_use]
    pub fn available(detail: impl Into<String>) -> Self {
        Self {
            available: true,
            detail: detail.into(),
        }
    }

    #[must_use]
    pub fn failed(detail: impl Into<String>) -> Self {
        Self {
            available: false,
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BridgeReadiness {
    backend_id: String,
    status: BridgeStatus,
    provider_id: Option<String>,
    auth_mode: AccountMode,
    failure_code: Option<BridgeFailureCode>,
    provider_identity_is_metadata_only: bool,
}

impl BridgeReadiness {
    #[must_use]
    pub fn status(&self) -> BridgeStatus {
        self.status
    }

    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    #[must_use]
    pub fn provider_id(&self) -> Option<&str> {
        self.provider_id.as_deref()
    }

    #[must_use]
    pub fn auth_mode(&self) -> AccountMode {
        self.auth_mode
    }

    #[must_use]
    pub fn failure_code(&self) -> Option<BridgeFailureCode> {
        self.failure_code
    }

    #[must_use]
    pub fn provider_identity_is_metadata_only(&self) -> bool {
        self.provider_identity_is_metadata_only
    }
}

pub struct BridgeReadinessService;

impl BridgeReadinessService {
    #[must_use]
    pub fn check_codex(
        bridge: &CodexAppServerBridge,
        probe: BridgeReadinessProbe,
    ) -> BridgeReadiness {
        BridgeReadiness {
            backend_id: bridge.backend_id().to_string(),
            status: status_from_probe(&probe),
            provider_id: bridge
                .provider_id()
                .map(|provider_id| provider_id.as_str().into()),
            auth_mode: bridge.auth_mode(),
            failure_code: None,
            provider_identity_is_metadata_only: true,
        }
    }

    #[must_use]
    pub fn check_claude(
        bridge: &ClaudeAgentSdkBridge,
        probe: BridgeReadinessProbe,
    ) -> BridgeReadiness {
        BridgeReadiness {
            backend_id: bridge.route_candidate().id().to_string(),
            status: status_from_probe(&probe),
            provider_id: None,
            auth_mode: bridge.auth_mode(),
            failure_code: (!probe.available).then_some(BridgeFailureCode::SdkUnavailable),
            provider_identity_is_metadata_only: true,
        }
    }

    #[must_use]
    pub fn check_acp_plugin(plugin_loaded: Option<bool>) -> BridgeReadiness {
        let loaded = plugin_loaded.unwrap_or(false);
        BridgeReadiness {
            backend_id: "plugin.acp".to_string(),
            status: if loaded {
                BridgeStatus::Ready
            } else {
                BridgeStatus::OptionalUnavailable
            },
            provider_id: None,
            auth_mode: AccountMode::LocalAppSession,
            failure_code: (!loaded).then_some(BridgeFailureCode::PluginMissing),
            provider_identity_is_metadata_only: true,
        }
    }
}

fn status_from_probe(probe: &BridgeReadinessProbe) -> BridgeStatus {
    if probe.available {
        BridgeStatus::Ready
    } else {
        BridgeStatus::Blocked
    }
}
