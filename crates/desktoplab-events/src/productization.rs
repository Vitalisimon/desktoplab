#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductizationEventFamily {
    Provider,
    Runtime,
    Model,
    AgentTool,
    GitWorktree,
    PluginTrust,
    DiagnosticsRepair,
}

impl ProductizationEventFamily {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::Runtime => "runtime",
            Self::Model => "model",
            Self::AgentTool => "agent_tool",
            Self::GitWorktree => "git_worktree",
            Self::PluginTrust => "plugin_trust",
            Self::DiagnosticsRepair => "diagnostics_repair",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductizationEventKind {
    ProviderCredentialValidated,
    ProviderConnectivityChecked,
    RuntimeInstallStarted,
    RuntimeRepairSuggested,
    ModelDownloadProgress,
    AgentToolRequested,
    GitWorktreeCreated,
    PluginTrustChanged,
    DiagnosticsRepairSuggested,
}

impl ProductizationEventKind {
    #[must_use]
    pub fn requires_redaction(self) -> bool {
        matches!(
            self,
            Self::ProviderCredentialValidated | Self::ProviderConnectivityChecked
        )
    }
}
