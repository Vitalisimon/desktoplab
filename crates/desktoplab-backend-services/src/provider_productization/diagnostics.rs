use desktoplab_vault::SecretRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConnectivityInput {
    provider_id: String,
    secret_ref: Option<SecretRef>,
    probe_token: String,
}

impl ProviderConnectivityInput {
    #[must_use]
    pub fn new(
        provider_id: impl Into<String>,
        secret_ref: Option<SecretRef>,
        probe_token: impl Into<String>,
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            secret_ref,
            probe_token: probe_token.into(),
        }
    }

    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    #[must_use]
    pub fn secret_ref(&self) -> Option<&SecretRef> {
        self.secret_ref.as_ref()
    }

    #[must_use]
    pub fn probe_token(&self) -> &str {
        &self.probe_token
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderConnectivityState {
    Ready,
    MissingCredential,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderConnectivityDiagnostic {
    provider_id: String,
    state: ProviderConnectivityState,
    redacted_authorization: String,
}

impl ProviderConnectivityDiagnostic {
    #[must_use]
    pub(super) fn new(provider_id: String, state: ProviderConnectivityState) -> Self {
        Self {
            provider_id,
            state,
            redacted_authorization: "Bearer [REDACTED]".to_string(),
        }
    }

    #[must_use]
    pub fn state(&self) -> ProviderConnectivityState {
        self.state
    }

    #[must_use]
    pub fn redacted_authorization(&self) -> &str {
        &self.redacted_authorization
    }

    #[must_use]
    pub fn diagnostic_payload(&self) -> String {
        format!(
            r#"{{"provider_id":"{}","authorization":"{}","workspace_content_included":false}}"#,
            self.provider_id, self.redacted_authorization
        )
    }
}
