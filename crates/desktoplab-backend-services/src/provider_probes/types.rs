use desktoplab_vault::{AuthModeMetadata, SecretRef};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ProviderProbeSource {
    LocalConfigDiscovery,
    CliInvocation,
    OauthDeviceFlow,
    ApiKeyRequest,
    LocalEndpoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProbePermission {
    LocalConfigMetadata,
    ProcessExecution,
    BrowserOpen,
    VaultRead,
    LoopbackNetwork,
    ProviderNetwork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProbeInitiation {
    Background,
    UserRequested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProbeConfidence {
    ConfigOnly,
    ExecutableResponded,
    AuthenticatedProviderResponse,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderProbeState {
    Ready,
    Unavailable,
    MissingCredential,
    Unauthorized,
    RateLimited,
    Cooldown,
    Stale,
    UnsupportedPackage,
    PermissionRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderProbeDefinition {
    pub(super) provider_id: String,
    pub(super) auth_mode: AuthModeMetadata,
    pub(super) source: ProviderProbeSource,
    pub(super) max_age_seconds: u64,
    pub(super) cooldown_seconds: u64,
    pub(super) permissions: Vec<ProviderProbePermission>,
    pub(super) supported_targets: Vec<String>,
}

impl ProviderProbeDefinition {
    pub fn new(
        provider_id: impl Into<String>,
        auth_mode: AuthModeMetadata,
        source: ProviderProbeSource,
        max_age_seconds: u64,
        cooldown_seconds: u64,
        permissions: Vec<ProviderProbePermission>,
        supported_targets: Vec<String>,
    ) -> Result<Self, ProviderProbeError> {
        let definition = Self {
            provider_id: provider_id.into(),
            auth_mode,
            source,
            max_age_seconds,
            cooldown_seconds,
            permissions,
            supported_targets,
        };
        definition.validate()?;
        Ok(definition)
    }

    fn validate(&self) -> Result<(), ProviderProbeError> {
        if self.provider_id.trim().is_empty()
            || self.max_age_seconds == 0
            || self.cooldown_seconds == 0
            || self.supported_targets.is_empty()
        {
            return Err(ProviderProbeError::InvalidDefinition);
        }
        for permission in required_permissions(self.source) {
            if !self.permissions.contains(permission) {
                return Err(ProviderProbeError::MissingPermission(*permission));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    #[must_use]
    pub fn auth_mode(&self) -> AuthModeMetadata {
        self.auth_mode
    }

    #[must_use]
    pub fn source(&self) -> ProviderProbeSource {
        self.source
    }

    #[must_use]
    pub fn permissions(&self) -> &[ProviderProbePermission] {
        &self.permissions
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderProbeExecution {
    pub(super) state: ProviderProbeState,
    pub(super) confidence: ProviderProbeConfidence,
    pub(super) summary: String,
    pub(super) evidence_ref: String,
}

impl ProviderProbeExecution {
    pub fn new(
        state: ProviderProbeState,
        confidence: ProviderProbeConfidence,
        summary: impl Into<String>,
        evidence_ref: impl Into<String>,
    ) -> Result<Self, ProviderProbeError> {
        let summary = summary.into();
        let evidence_ref = evidence_ref.into();
        if secret_like(&summary) || secret_like(&evidence_ref) || evidence_ref.trim().is_empty() {
            return Err(ProviderProbeError::SecretLikeEvidence);
        }
        Ok(Self {
            state,
            confidence,
            summary,
            evidence_ref,
        })
    }
}

pub struct ProviderProbeRequest<'a> {
    pub definition: &'a ProviderProbeDefinition,
    pub credential_ref: Option<&'a SecretRef>,
    pub target: &'a str,
}

pub trait ProviderProbeExecutor {
    fn execute(
        &mut self,
        request: ProviderProbeRequest<'_>,
    ) -> Result<ProviderProbeExecution, ProviderProbeError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderProbeError {
    InvalidDefinition,
    MissingPermission(ProviderProbePermission),
    SecretLikeEvidence,
    ExecutionFailed(String),
}

fn required_permissions(source: ProviderProbeSource) -> &'static [ProviderProbePermission] {
    match source {
        ProviderProbeSource::LocalConfigDiscovery => {
            &[ProviderProbePermission::LocalConfigMetadata]
        }
        ProviderProbeSource::CliInvocation => &[ProviderProbePermission::ProcessExecution],
        ProviderProbeSource::OauthDeviceFlow => &[
            ProviderProbePermission::BrowserOpen,
            ProviderProbePermission::VaultRead,
            ProviderProbePermission::ProviderNetwork,
        ],
        ProviderProbeSource::ApiKeyRequest => &[
            ProviderProbePermission::VaultRead,
            ProviderProbePermission::ProviderNetwork,
        ],
        ProviderProbeSource::LocalEndpoint => &[ProviderProbePermission::LoopbackNetwork],
    }
}

fn secret_like(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("bearer ")
        || lower.contains("password=")
        || lower.contains("token=")
        || lower.contains("cookie=")
        || lower.contains("sk-")
}
