use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;
const MAX_VERSIONS: usize = 1_024;
const MAX_EVENTS: usize = 4_096;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSourceTrust {
    VerifiedPublisher,
    LocalOwner,
    Unverified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstallTrustPolicy {
    pub allow_local_owner: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionVersion {
    pub version: String,
    pub digest: String,
    pub predecessor_digest: Option<String>,
    pub publisher_id: String,
    pub source_trust: ExtensionSourceTrust,
    pub revoked: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipTransfer {
    pub from_publisher_id: String,
    pub to_publisher_id: String,
    pub at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionEvent {
    pub kind: String,
    pub digest: Option<String>,
    pub actor_id: String,
    pub at_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRecord {
    schema_version: u32,
    pub extension_id: String,
    pub owner_publisher_id: String,
    pub versions: Vec<ExtensionVersion>,
    pub ownership_transfers: Vec<OwnershipTransfer>,
    pub installed_digest: Option<String>,
    pub installed_revoked: bool,
    pub events: Vec<ExtensionEvent>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExtensionRegistryError {
    Invalid(&'static str),
    NotFound,
    OwnerMismatch,
    ImmutableVersionConflict,
    InvalidLineage,
    TrustDenied,
    VersionRevoked,
    CapacityExceeded,
    Persistence(String),
}

pub struct ExtensionRegistryService<'a> {
    storage: &'a SqliteStore,
}

impl<'a> ExtensionRegistryService<'a> {
    pub fn new(storage: &'a SqliteStore) -> Self {
        Self { storage }
    }

    pub fn publish(
        &self,
        extension_id: &str,
        owner_publisher_id: &str,
        version: ExtensionVersion,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        validate_id(extension_id)?;
        validate_id(owner_publisher_id)?;
        validate_version(&version)?;
        let mut record = match self.load(extension_id) {
            Ok(record) => record,
            Err(ExtensionRegistryError::NotFound) => ExtensionRecord {
                schema_version: SCHEMA_VERSION,
                extension_id: extension_id.to_string(),
                owner_publisher_id: owner_publisher_id.to_string(),
                versions: Vec::new(),
                ownership_transfers: Vec::new(),
                installed_digest: None,
                installed_revoked: false,
                events: Vec::new(),
            },
            Err(error) => return Err(error),
        };
        if record.owner_publisher_id != owner_publisher_id
            || version.publisher_id != owner_publisher_id
        {
            return Err(ExtensionRegistryError::OwnerMismatch);
        }
        if let Some(existing) = record
            .versions
            .iter()
            .find(|existing| existing.version == version.version)
        {
            return if existing == &version {
                Ok(record)
            } else {
                Err(ExtensionRegistryError::ImmutableVersionConflict)
            };
        }
        if record.versions.len() == MAX_VERSIONS {
            return Err(ExtensionRegistryError::CapacityExceeded);
        }
        let expected_predecessor = record.versions.last().map(|item| item.digest.as_str());
        if version.predecessor_digest.as_deref() != expected_predecessor {
            return Err(ExtensionRegistryError::InvalidLineage);
        }
        push_event(
            &mut record,
            "published",
            Some(&version.digest),
            owner_publisher_id,
            at_ms,
        )?;
        record.versions.push(version);
        self.persist(&record)?;
        Ok(record)
    }

    pub fn transfer_ownership(
        &self,
        extension_id: &str,
        from_publisher_id: &str,
        to_publisher_id: &str,
        source_approved: bool,
        destination_accepted: bool,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        let mut record = self.load(extension_id)?;
        if record.owner_publisher_id != from_publisher_id {
            return Err(ExtensionRegistryError::OwnerMismatch);
        }
        validate_id(to_publisher_id)?;
        if !source_approved || !destination_accepted {
            return Err(ExtensionRegistryError::Invalid("transfer_consent_required"));
        }
        record.ownership_transfers.push(OwnershipTransfer {
            from_publisher_id: from_publisher_id.to_string(),
            to_publisher_id: to_publisher_id.to_string(),
            at_ms,
        });
        record.owner_publisher_id = to_publisher_id.to_string();
        push_event(
            &mut record,
            "ownership_transferred",
            None,
            to_publisher_id,
            at_ms,
        )?;
        self.persist(&record)?;
        Ok(record)
    }

    pub fn install(
        &self,
        extension_id: &str,
        digest: &str,
        actor_id: &str,
        policy: InstallTrustPolicy,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        self.select_version(extension_id, digest, actor_id, policy, "installed", at_ms)
    }

    pub fn update(
        &self,
        extension_id: &str,
        digest: &str,
        actor_id: &str,
        policy: InstallTrustPolicy,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        let record = self.load(extension_id)?;
        let installed = record
            .installed_digest
            .as_deref()
            .ok_or(ExtensionRegistryError::Invalid("extension_not_installed"))?;
        let target = version_by_digest(&record, digest)?;
        if target.predecessor_digest.as_deref() != Some(installed) {
            return Err(ExtensionRegistryError::InvalidLineage);
        }
        self.select_version(extension_id, digest, actor_id, policy, "updated", at_ms)
    }

    pub fn rollback(
        &self,
        extension_id: &str,
        digest: &str,
        actor_id: &str,
        policy: InstallTrustPolicy,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        self.select_version(extension_id, digest, actor_id, policy, "rolled_back", at_ms)
    }

    pub fn revoke(
        &self,
        extension_id: &str,
        digest: &str,
        publisher_id: &str,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        let mut record = self.load(extension_id)?;
        if record.owner_publisher_id != publisher_id {
            return Err(ExtensionRegistryError::OwnerMismatch);
        }
        let version = record
            .versions
            .iter_mut()
            .find(|version| version.digest == digest)
            .ok_or(ExtensionRegistryError::NotFound)?;
        version.revoked = true;
        record.installed_revoked = record.installed_digest.as_deref() == Some(digest);
        push_event(&mut record, "revoked", Some(digest), publisher_id, at_ms)?;
        self.persist(&record)?;
        Ok(record)
    }

    pub fn load(&self, extension_id: &str) -> Result<ExtensionRecord, ExtensionRegistryError> {
        let record = self
            .storage
            .get_productization_state(ProductizationRecordKind::ExtensionRegistry, extension_id)
            .map_err(|error| ExtensionRegistryError::Persistence(error.to_string()))?
            .ok_or(ExtensionRegistryError::NotFound)?;
        let value: ExtensionRecord = serde_json::from_str(record.payload())
            .map_err(|error| ExtensionRegistryError::Persistence(error.to_string()))?;
        if value.schema_version != SCHEMA_VERSION {
            return Err(ExtensionRegistryError::Persistence(
                "unsupported_extension_registry_schema".to_string(),
            ));
        }
        Ok(value)
    }

    fn select_version(
        &self,
        extension_id: &str,
        digest: &str,
        actor_id: &str,
        policy: InstallTrustPolicy,
        event_kind: &str,
        at_ms: u64,
    ) -> Result<ExtensionRecord, ExtensionRegistryError> {
        let mut record = self.load(extension_id)?;
        let version = version_by_digest(&record, digest)?;
        if version.revoked {
            return Err(ExtensionRegistryError::VersionRevoked);
        }
        if version.source_trust == ExtensionSourceTrust::Unverified
            || (version.source_trust == ExtensionSourceTrust::LocalOwner
                && !policy.allow_local_owner)
        {
            return Err(ExtensionRegistryError::TrustDenied);
        }
        record.installed_digest = Some(digest.to_string());
        record.installed_revoked = false;
        push_event(&mut record, event_kind, Some(digest), actor_id, at_ms)?;
        self.persist(&record)?;
        Ok(record)
    }

    fn persist(&self, record: &ExtensionRecord) -> Result<(), ExtensionRegistryError> {
        let payload = serde_json::to_string(record)
            .map_err(|error| ExtensionRegistryError::Persistence(error.to_string()))?;
        self.storage
            .put_productization_state(ProductizationStateRecord::new(
                ProductizationRecordKind::ExtensionRegistry,
                &record.extension_id,
                payload,
            ))
            .map_err(|error| ExtensionRegistryError::Persistence(error.to_string()))
    }
}

fn validate_id(value: &str) -> Result<(), ExtensionRegistryError> {
    if value.is_empty() || value.len() > 160 {
        Err(ExtensionRegistryError::Invalid("invalid_identifier"))
    } else {
        Ok(())
    }
}

fn validate_version(version: &ExtensionVersion) -> Result<(), ExtensionRegistryError> {
    validate_id(&version.version)?;
    validate_id(&version.publisher_id)?;
    if version.digest.len() != 64 || !version.digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(ExtensionRegistryError::Invalid("invalid_digest"));
    }
    Ok(())
}

fn version_by_digest<'a>(
    record: &'a ExtensionRecord,
    digest: &str,
) -> Result<&'a ExtensionVersion, ExtensionRegistryError> {
    record
        .versions
        .iter()
        .find(|version| version.digest == digest)
        .ok_or(ExtensionRegistryError::NotFound)
}

fn push_event(
    record: &mut ExtensionRecord,
    kind: &str,
    digest: Option<&str>,
    actor_id: &str,
    at_ms: u64,
) -> Result<(), ExtensionRegistryError> {
    if record.events.len() == MAX_EVENTS {
        return Err(ExtensionRegistryError::CapacityExceeded);
    }
    record.events.push(ExtensionEvent {
        kind: kind.to_string(),
        digest: digest.map(ToString::to_string),
        actor_id: actor_id.to_string(),
        at_ms,
    });
    Ok(())
}
