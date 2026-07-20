use crate::{AccountMode, BackendCapability, ExecutionBackendId, ProviderId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderKind {
    CloudAccount,
    LocalService,
    PrivateRegistry,
    EnterpriseService,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provider {
    id: ProviderId,
    name: String,
    kind: ProviderKind,
}

impl Provider {
    #[must_use]
    pub fn new(id: ProviderId, name: impl Into<String>, kind: ProviderKind) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
        }
    }

    #[must_use]
    pub fn id(&self) -> &ProviderId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn kind(&self) -> ProviderKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionBackendKind {
    LocalRuntime,
    CloudModel,
    ExternalAgent,
    RemoteRunner,
    Plugin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionBackend {
    id: ExecutionBackendId,
    name: String,
    kind: ExecutionBackendKind,
    provider_id: Option<ProviderId>,
    account_mode: Option<AccountMode>,
    capabilities: Vec<BackendCapability>,
}

impl ExecutionBackend {
    #[must_use]
    pub fn new(
        id: ExecutionBackendId,
        name: impl Into<String>,
        kind: ExecutionBackendKind,
        provider_id: Option<ProviderId>,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            kind,
            provider_id,
            account_mode: None,
            capabilities: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_account_mode(mut self, account_mode: AccountMode) -> Self {
        self.account_mode = Some(account_mode);
        self
    }

    #[must_use]
    pub fn with_capability(mut self, capability: BackendCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    #[must_use]
    pub fn id(&self) -> &ExecutionBackendId {
        &self.id
    }

    #[must_use]
    pub fn kind(&self) -> ExecutionBackendKind {
        self.kind
    }

    #[must_use]
    pub fn provider_id(&self) -> Option<&ProviderId> {
        self.provider_id.as_ref()
    }

    #[must_use]
    pub fn account_mode(&self) -> Option<AccountMode> {
        self.account_mode
    }

    #[must_use]
    pub fn supports(&self, capability: &str) -> bool {
        self.capabilities
            .iter()
            .any(|existing| existing.as_str() == capability)
    }

    #[must_use]
    pub fn supports_all(&self, required: &[BackendCapability]) -> bool {
        required
            .iter()
            .all(|capability| self.supports(capability.as_str()))
    }
}
