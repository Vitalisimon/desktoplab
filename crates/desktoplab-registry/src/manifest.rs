use serde::{Deserialize, Serialize};

use crate::{ManifestFamily, ManifestStatus, RegistryError};

pub const REGISTRY_SCHEMA: &str = "registry.desktoplab.dev/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RegistryManifest {
    schema: String,
    manifest_id: String,
    manifest_version: String,
    family: ManifestFamily,
    status: ManifestStatus,
    channel: String,
    created_at: String,
    updated_at: String,
    publisher: String,
    content_hash: String,
    compatibility: serde_json::Value,
    evidence: serde_json::Value,
    policy: serde_json::Value,
}

impl RegistryManifest {
    #[must_use]
    pub fn new_for_test(
        manifest_id: impl Into<String>,
        family: ManifestFamily,
        status: ManifestStatus,
    ) -> Self {
        Self {
            schema: REGISTRY_SCHEMA.to_string(),
            manifest_id: manifest_id.into(),
            manifest_version: "1".to_string(),
            family,
            status,
            channel: "stable".to_string(),
            created_at: "2026-06-25T00:00:00Z".to_string(),
            updated_at: "2026-06-25T00:00:00Z".to_string(),
            publisher: "desktoplab".to_string(),
            content_hash: "sha256:test".to_string(),
            compatibility: serde_json::json!({}),
            evidence: serde_json::json!({}),
            policy: serde_json::json!({}),
        }
    }

    #[must_use]
    pub fn manifest_id(&self) -> &str {
        &self.manifest_id
    }

    #[must_use]
    pub fn family(&self) -> ManifestFamily {
        self.family
    }

    #[must_use]
    pub fn status(&self) -> ManifestStatus {
        self.status
    }

    pub(crate) fn validate_for_family(
        &self,
        expected: ManifestFamily,
    ) -> Result<(), RegistryError> {
        if self.schema != REGISTRY_SCHEMA {
            return Err(RegistryError::InvalidManifest(format!(
                "manifest {} has unsupported schema {}",
                self.manifest_id, self.schema
            )));
        }

        if self.family != expected {
            return Err(RegistryError::InvalidManifest(format!(
                "manifest {} has family {}, expected {}",
                self.manifest_id,
                self.family.as_str(),
                expected.as_str()
            )));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SignedManifestGroup {
    pub schema: String,
    pub family: ManifestFamily,
    pub signature: String,
    pub payload: ManifestPayload,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct ManifestPayload {
    pub manifests: Vec<RegistryManifest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ManifestGroup {
    family: ManifestFamily,
    manifests: Vec<RegistryManifest>,
    from_last_known_good: bool,
}

impl ManifestGroup {
    #[must_use]
    pub fn new(family: ManifestFamily, manifests: Vec<RegistryManifest>) -> Self {
        Self {
            family,
            manifests,
            from_last_known_good: false,
        }
    }

    #[must_use]
    pub fn family(&self) -> ManifestFamily {
        self.family
    }

    #[must_use]
    pub fn manifests(&self) -> &[RegistryManifest] {
        &self.manifests
    }

    #[must_use]
    pub fn from_last_known_good(&self) -> bool {
        self.from_last_known_good
    }

    #[must_use]
    pub fn mark_last_known_good(mut self) -> Self {
        self.from_last_known_good = true;
        self
    }
}
