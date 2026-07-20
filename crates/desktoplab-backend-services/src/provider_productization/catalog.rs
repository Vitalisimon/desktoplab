use desktoplab_vault::{AuthModeMetadata, SecretRef};

use super::diagnostics::{
    ProviderConnectivityDiagnostic, ProviderConnectivityInput, ProviderConnectivityState,
};
use super::routing::{ProviderCandidate, ProviderCandidateKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderSpec {
    provider_id: String,
    display_name: String,
    capabilities: Vec<String>,
    supported_account_modes: Vec<AuthModeMetadata>,
}

impl ProviderSpec {
    fn new(
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        capabilities: &[&str],
        supported_account_modes: &[AuthModeMetadata],
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            display_name: display_name.into(),
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
            supported_account_modes: supported_account_modes.to_vec(),
        }
    }

    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    #[must_use]
    pub fn supported_account_modes(&self) -> &[AuthModeMetadata] {
        &self.supported_account_modes
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProviderProductizationCatalog {
    providers: Vec<ProviderSpec>,
}

impl ProviderProductizationCatalog {
    #[must_use]
    pub fn default_cloud() -> Self {
        Self {
            providers: vec![
                ProviderSpec::new(
                    "provider.openai",
                    "OpenAI",
                    &["llm.chat", "tools.function_call"],
                    &[
                        AuthModeMetadata::ApiKeyBilling,
                        AuthModeMetadata::SubscriptionAccount,
                        AuthModeMetadata::OauthDevice,
                    ],
                ),
                ProviderSpec::new(
                    "provider.anthropic",
                    "Anthropic",
                    &["llm.chat", "tools.function_call"],
                    &[
                        AuthModeMetadata::ApiKeyBilling,
                        AuthModeMetadata::SubscriptionAccount,
                    ],
                ),
                ProviderSpec::new(
                    "provider.openai-compatible",
                    "OpenAI-compatible",
                    &["llm.chat"],
                    &[AuthModeMetadata::CustomEndpoint],
                ),
            ],
        }
    }

    #[must_use]
    pub fn provider(&self, provider_id: &str) -> Option<&ProviderSpec> {
        self.providers
            .iter()
            .find(|provider| provider.provider_id == provider_id)
    }

    #[must_use]
    pub fn readiness(
        &self,
        provider_id: &str,
        secret_ref: Option<SecretRef>,
    ) -> ProviderReadinessReport {
        let Some(provider) = self.provider(provider_id) else {
            return ProviderReadinessReport::new(
                ProviderReadinessStatus::UnknownProvider,
                Vec::new(),
            );
        };
        let supported_account_modes = provider.supported_account_modes().to_vec();
        if secret_ref.is_none() {
            return ProviderReadinessReport::new(
                ProviderReadinessStatus::MissingCredential,
                supported_account_modes,
            );
        }
        ProviderReadinessReport::new(
            ProviderReadinessStatus::CredentialReferenceMissing,
            supported_account_modes,
        )
    }

    pub fn validate_openai_compatible_endpoint(
        &self,
        url: &str,
    ) -> Result<ProviderEndpointMetadata, ProviderEndpointError> {
        let class = if url.starts_with("http://127.0.0.1")
            || url.starts_with("http://localhost")
            || url.starts_with("http://[::1]")
        {
            ProviderEndpointClass::Localhost
        } else if url.starts_with("https://") {
            ProviderEndpointClass::Remote
        } else {
            return Err(ProviderEndpointError::UnsupportedScheme);
        };
        Ok(ProviderEndpointMetadata {
            url: url.to_string(),
            class,
        })
    }

    #[must_use]
    pub fn connectivity_diagnostic(
        &self,
        input: ProviderConnectivityInput,
    ) -> ProviderConnectivityDiagnostic {
        let state = if input.secret_ref().is_some() && !input.probe_token().is_empty() {
            ProviderConnectivityState::Ready
        } else {
            ProviderConnectivityState::MissingCredential
        };
        ProviderConnectivityDiagnostic::new(input.provider_id().to_string(), state)
    }

    #[must_use]
    pub fn local_provider_candidate(
        &self,
        id: impl Into<String>,
        capabilities: &[&str],
    ) -> ProviderCandidate {
        ProviderCandidate::new(id, capabilities, ProviderCandidateKind::Local, None)
    }

    #[must_use]
    pub fn cloud_provider_candidate(
        &self,
        id: impl Into<String>,
        capabilities: &[&str],
        cost_hint: impl Into<String>,
    ) -> ProviderCandidate {
        ProviderCandidate::new(
            id,
            capabilities,
            ProviderCandidateKind::Cloud,
            Some(cost_hint.into()),
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderReadinessStatus {
    Ready,
    MissingCredential,
    CredentialReferenceMissing,
    UnknownProvider,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderReadinessReport {
    status: ProviderReadinessStatus,
    supported_account_modes: Vec<AuthModeMetadata>,
}

impl ProviderReadinessReport {
    fn new(
        status: ProviderReadinessStatus,
        supported_account_modes: Vec<AuthModeMetadata>,
    ) -> Self {
        Self {
            status,
            supported_account_modes,
        }
    }

    #[must_use]
    pub fn status(&self) -> ProviderReadinessStatus {
        self.status
    }

    #[must_use]
    pub fn supported_account_modes(&self) -> &[AuthModeMetadata] {
        &self.supported_account_modes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderEndpointClass {
    Localhost,
    Remote,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderEndpointMetadata {
    url: String,
    class: ProviderEndpointClass,
}

impl ProviderEndpointMetadata {
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn class(&self) -> ProviderEndpointClass {
        self.class
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderEndpointError {
    UnsupportedScheme,
}
