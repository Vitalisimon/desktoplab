#![forbid(unsafe_code)]

mod protocol;

pub use protocol::{
    AcpCapabilityStatus, AcpDispatch, AcpHostPrompt, AcpProtocolAdapter, AcpSessionHost,
    acp_capability_matrix,
};

use desktoplab_backends::ExternalBackendManifest;
use desktoplab_execution_router::{BackendTrust, ExecutionRouteCandidate};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginTrust {
    Unverified,
    Verified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcpBackendPlugin {
    plugin_id: String,
    trust: PluginTrust,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AcpPluginLoader {
    loaded_through_plugin_host: bool,
}

impl AcpPluginLoader {
    #[must_use]
    pub fn load(&self, plugin: AcpBackendPlugin) -> AcpBackendPlugin {
        plugin
    }

    #[must_use]
    pub fn loaded_through_plugin_host(&self) -> bool {
        true
    }
}

impl AcpBackendPlugin {
    #[must_use]
    pub fn new_unverified(plugin_id: impl Into<String>) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            trust: PluginTrust::Unverified,
        }
    }

    #[must_use]
    pub fn new_verified(plugin_id: impl Into<String>) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            trust: PluginTrust::Verified,
        }
    }

    #[must_use]
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    #[must_use]
    pub fn is_core_component(&self) -> bool {
        false
    }

    #[must_use]
    pub fn trust(&self) -> PluginTrust {
        self.trust
    }

    #[must_use]
    pub fn backend_manifest(&self) -> ExternalBackendManifest {
        ExternalBackendManifest::new(
            "backend.acp-plugin",
            &["llm.chat", "agent.events.stream", "plugin.acp"],
        )
    }

    #[must_use]
    pub fn route_candidate(&self) -> ExecutionRouteCandidate {
        let trust = match self.trust {
            PluginTrust::Unverified => BackendTrust::Unverified,
            PluginTrust::Verified => BackendTrust::Verified,
        };

        ExecutionRouteCandidate::new("backend.acp-plugin")
            .with_capability("llm.chat")
            .with_capability("agent.events.stream")
            .with_capability("plugin.acp")
            .with_trust(trust)
    }
}
