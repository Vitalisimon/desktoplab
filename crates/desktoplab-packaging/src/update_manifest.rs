use crate::{PlatformTarget, ReleaseChannel, SignatureState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateManifestEntry {
    version: String,
    channel: ReleaseChannel,
    platform: PlatformTarget,
    url: String,
    sha256: String,
    signature_state: SignatureState,
    release_notes_url: String,
}

impl UpdateManifestEntry {
    pub fn new(
        version: impl Into<String>,
        channel: ReleaseChannel,
        platform: PlatformTarget,
        url: impl Into<String>,
        sha256: impl Into<String>,
        signature_state: SignatureState,
        release_notes_url: impl Into<String>,
    ) -> Result<Self, UpdateManifestError> {
        let version = version.into();
        let url = url.into();
        let sha256 = sha256.into();
        let release_notes_url = release_notes_url.into();

        reject_blank("version", &version)?;
        reject_https_url("url", &url)?;
        reject_sha256(&sha256)?;
        reject_https_url("releaseNotesUrl", &release_notes_url)?;
        if !channel.accepts_signature_state(signature_state.label()) {
            return Err(format!(
                "{} channel rejects {} update signature state",
                channel.as_str(),
                signature_state.label()
            ));
        }

        Ok(Self {
            version,
            channel,
            platform,
            url,
            sha256,
            signature_state,
            release_notes_url,
        })
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub const fn channel(&self) -> ReleaseChannel {
        self.channel
    }

    #[must_use]
    pub const fn platform(&self) -> PlatformTarget {
        self.platform
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub const fn signature_state(&self) -> &SignatureState {
        &self.signature_state
    }

    #[must_use]
    pub fn release_notes_url(&self) -> &str {
        &self.release_notes_url
    }

    pub fn validate_version_transition(
        &self,
        current_version: &str,
    ) -> Result<(), UpdateManifestError> {
        reject_blank("currentVersion", current_version)?;
        if compare_versions(&self.version, current_version).is_lt() {
            return Err("downgrade blocked by default".to_string());
        }
        Ok(())
    }

    pub fn emergency_rollback_from(
        &self,
        current_version: &str,
    ) -> Result<(), UpdateManifestError> {
        reject_blank("currentVersion", current_version)?;
        if !self.signature_state.is_publish_signed() {
            return Err("emergency rollback requires signed update metadata".to_string());
        }
        if compare_versions(&self.version, current_version).is_lt() {
            return Ok(());
        }
        Err("emergency rollback requires a lower target version".to_string())
    }
}

pub type UpdateManifestError = String;

fn reject_blank(field: &'static str, value: &str) -> Result<(), UpdateManifestError> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be blank"));
    }
    Ok(())
}

fn reject_https_url(field: &'static str, value: &str) -> Result<(), UpdateManifestError> {
    reject_blank(field, value)?;
    if !value.starts_with("https://") && !value.starts_with("file://") {
        return Err(format!("{field} must be https or file URL"));
    }
    Ok(())
}

fn reject_sha256(value: &str) -> Result<(), UpdateManifestError> {
    if value.len() == 64
        && value
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
    {
        return Ok(());
    }
    Err("sha256 must be a 64 character lowercase hex digest".to_string())
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let left_parts = version_parts(left);
    let right_parts = version_parts(right);
    left_parts.cmp(&right_parts)
}

fn version_parts(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect()
}
