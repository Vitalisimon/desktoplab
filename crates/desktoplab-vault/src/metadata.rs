use crate::SecretRef;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthModeMetadata {
    ApiKeyBilling,
    SubscriptionAccount,
    OauthDevice,
    LocalAppSession,
    CustomEndpoint,
}

impl AuthModeMetadata {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialMetadataError {
    SecretLikeMetadata(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CredentialMetadata {
    secret_ref: SecretRef,
    label: String,
    auth_mode: AuthModeMetadata,
    public_metadata: Vec<(String, String)>,
}

impl CredentialMetadata {
    #[must_use]
    pub fn new(secret_ref: SecretRef, label: impl Into<String>) -> Self {
        Self::with_auth_mode(secret_ref, label, AuthModeMetadata::ApiKeyBilling)
    }

    #[must_use]
    pub fn with_auth_mode(
        secret_ref: SecretRef,
        label: impl Into<String>,
        auth_mode: AuthModeMetadata,
    ) -> Self {
        Self {
            secret_ref,
            label: label.into(),
            auth_mode,
            public_metadata: Vec::new(),
        }
    }

    pub fn with_public_metadata(
        mut self,
        metadata: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Result<Self, CredentialMetadataError> {
        let mut public_metadata = Vec::new();
        for (key, value) in metadata {
            let key = key.into();
            let value = value.into();
            if is_secret_like(&key, &value) {
                return Err(CredentialMetadataError::SecretLikeMetadata(key));
            }
            public_metadata.push((key, value));
        }
        self.public_metadata = public_metadata;
        Ok(self)
    }

    #[must_use]
    pub fn secret_ref(&self) -> &SecretRef {
        &self.secret_ref
    }

    #[must_use]
    pub fn label(&self) -> &str {
        &self.label
    }

    #[must_use]
    pub fn auth_mode(&self) -> AuthModeMetadata {
        self.auth_mode
    }

    #[must_use]
    pub fn public_metadata(&self) -> &[(String, String)] {
        &self.public_metadata
    }
}

fn is_secret_like(key: &str, value: &str) -> bool {
    let key = key.to_ascii_lowercase();
    let value = value.to_ascii_lowercase();
    key.contains("token")
        || key.contains("cookie")
        || key.contains("session")
        || key.contains("authorization")
        || value.starts_with("sk-")
        || value.starts_with("bearer ")
        || value.contains("cookie")
}
