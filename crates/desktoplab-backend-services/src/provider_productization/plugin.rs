const KNOWN_PROVIDER_CAPABILITIES: &[&str] = &["llm.chat", "tools.function_call", "llm.embeddings"];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderManifestTrust {
    Verified,
    Unverified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderPluginManifest {
    provider_id: String,
    trust: ProviderManifestTrust,
    capabilities: Vec<String>,
}

impl ProviderPluginManifest {
    #[must_use]
    pub fn new(
        provider_id: impl Into<String>,
        trust: ProviderManifestTrust,
        capabilities: &[&str],
    ) -> Self {
        Self {
            provider_id: provider_id.into(),
            trust,
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
        }
    }

    #[must_use]
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub fn validate(&self) -> Result<(), ProviderPluginError> {
        if self
            .capabilities
            .iter()
            .all(|capability| KNOWN_PROVIDER_CAPABILITIES.contains(&capability.as_str()))
        {
            Ok(())
        } else {
            Err(ProviderPluginError::UnknownCapability)
        }
    }

    pub fn route_sensitive_work(&self) -> Result<(), ProviderPluginError> {
        self.validate()?;
        if self.trust == ProviderManifestTrust::Unverified {
            return Err(ProviderPluginError::UnverifiedProviderPlugin);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderPluginError {
    UnknownCapability,
    UnverifiedProviderPlugin,
}
