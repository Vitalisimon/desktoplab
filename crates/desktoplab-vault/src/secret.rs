use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SecretScope {
    Provider,
    ExternalBackend,
    PrivateRegistry,
    EnterprisePolicy,
}

impl SecretScope {
    #[must_use]
    pub fn as_uri_segment(self) -> &'static str {
        match self {
            Self::Provider => "provider",
            Self::ExternalBackend => "external-backend",
            Self::PrivateRegistry => "private-registry",
            Self::EnterprisePolicy => "enterprise-policy",
        }
    }

    #[must_use]
    pub fn from_uri_segment(segment: &str) -> Option<Self> {
        match segment {
            "provider" => Some(Self::Provider),
            "external-backend" => Some(Self::ExternalBackend),
            "private-registry" => Some(Self::PrivateRegistry),
            "enterprise-policy" => Some(Self::EnterprisePolicy),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SecretRef {
    scope: SecretScope,
    id: String,
}

impl SecretRef {
    #[must_use]
    pub fn new(scope: SecretScope, id: impl Into<String>) -> Self {
        Self {
            scope,
            id: id.into(),
        }
    }

    #[must_use]
    pub fn scope(&self) -> SecretScope {
        self.scope
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn as_uri(&self) -> String {
        format!(
            "vault://desktoplab/{}/{}",
            self.scope.as_uri_segment(),
            self.id
        )
    }

    pub fn from_uri(uri: &str) -> Result<Self, SecretRefParseError> {
        let Some(rest) = uri.strip_prefix("vault://desktoplab/") else {
            return Err(SecretRefParseError::InvalidPrefix);
        };
        let Some((scope, id)) = rest.split_once('/') else {
            return Err(SecretRefParseError::MissingId);
        };
        if id.trim().is_empty() {
            return Err(SecretRefParseError::MissingId);
        }
        let Some(scope) = SecretScope::from_uri_segment(scope) else {
            return Err(SecretRefParseError::InvalidScope(scope.to_string()));
        };
        Ok(Self::new(scope, id))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SecretRefParseError {
    InvalidPrefix,
    InvalidScope(String),
    MissingId,
}

#[derive(Clone, Eq, PartialEq)]
pub struct SecretValue(String);

impl SecretValue {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn redacted(&self) -> &'static str {
        "[REDACTED]"
    }

    #[must_use]
    pub fn expose_for_adapter(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SecretValue")
            .field(&self.redacted())
            .finish()
    }
}
