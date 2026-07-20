#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EgressClassification {
    SafeToEgress,
    LocalOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EgressAccountMode {
    ApiKeyBilling,
    SubscriptionAccount,
    OauthDevice,
    LocalAppSession,
    CustomEndpoint,
}

impl EgressAccountMode {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ApiKeyBilling => "api_key_billing",
            Self::SubscriptionAccount => "subscription_account",
            Self::OauthDevice => "oauth_device",
            Self::LocalAppSession => "local_app_session",
            Self::CustomEndpoint => "custom_endpoint",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProviderEgressContext {
    classification: EgressClassification,
    account_mode: EgressAccountMode,
    fallback_account_mode: Option<EgressAccountMode>,
}

impl ProviderEgressContext {
    #[must_use]
    pub fn new(classification: EgressClassification, account_mode: EgressAccountMode) -> Self {
        Self {
            classification,
            account_mode,
            fallback_account_mode: None,
        }
    }

    #[must_use]
    pub fn with_fallback_account_mode(mut self, fallback_account_mode: EgressAccountMode) -> Self {
        self.fallback_account_mode = Some(fallback_account_mode);
        self
    }

    #[must_use]
    pub fn classification(self) -> EgressClassification {
        self.classification
    }

    #[must_use]
    pub fn account_mode(self) -> EgressAccountMode {
        self.account_mode
    }

    #[must_use]
    pub fn fallback_account_mode(self) -> Option<EgressAccountMode> {
        self.fallback_account_mode
    }

    #[must_use]
    pub fn requires_billing_fallback_approval(self) -> bool {
        matches!(
            (self.account_mode, self.fallback_account_mode),
            (
                EgressAccountMode::SubscriptionAccount,
                Some(EgressAccountMode::ApiKeyBilling)
            )
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Action {
    ProtectedWorkspaceAccess,
    FilesystemRead,
    FilesystemWrite,
    GeneratedArtifactWrite,
    TerminalCommand,
    ProcessStart,
    ProcessControl,
    TestRun,
    DependencyInstall,
    GitRead,
    GitCommit,
    GitPush,
    CheckpointCreate,
    McpInvoke,
    Clarification,
    AgentControl,
    ModelDownload,
    RuntimeInstall,
    ProviderEgress(EgressClassification),
    ProviderEgressWithAccount(ProviderEgressContext),
}
