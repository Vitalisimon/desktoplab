use crate::{PackageArtifactSpec, PlatformTarget, ReleaseChannel};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactManifestEntry {
    spec: PackageArtifactSpec,
    sha256: String,
    size_bytes: u64,
    signature_state: SignatureState,
    build_source: BuildSource,
}

impl ArtifactManifestEntry {
    pub fn new(
        spec: PackageArtifactSpec,
        sha256: impl Into<String>,
        size_bytes: u64,
        signature_state: SignatureState,
        build_source: BuildSource,
    ) -> Result<Self, ManifestEntryError> {
        let sha256 = sha256.into();
        validate_sha256(&sha256)?;

        if size_bytes == 0 {
            return Err("sizeBytes must be greater than zero".to_string());
        }

        signature_state.validate()?;
        build_source.validate()?;

        Ok(Self {
            spec,
            sha256,
            size_bytes,
            signature_state,
            build_source,
        })
    }

    #[must_use]
    pub const fn target(&self) -> PlatformTarget {
        self.spec.target()
    }

    #[must_use]
    pub const fn channel(&self) -> ReleaseChannel {
        self.spec.channel()
    }

    #[must_use]
    pub fn version(&self) -> &str {
        self.spec.version()
    }

    #[must_use]
    pub fn file_name(&self) -> String {
        self.spec.file_name()
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub const fn signature_state(&self) -> &SignatureState {
        &self.signature_state
    }

    #[must_use]
    pub const fn build_source(&self) -> &BuildSource {
        &self.build_source
    }

    pub fn validate_for_publish(&self) -> Result<(), ManifestEntryError> {
        if self.channel().allows_unsigned_artifacts() || self.signature_state.is_publish_signed() {
            return Ok(());
        }

        Err(format!(
            "{} artifacts require signing fields before publish",
            self.channel().as_str()
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SignatureState {
    UnsignedDev,
    Signed {
        identity: String,
        signature_ref: String,
    },
    Notarized {
        identity: String,
        signature_ref: String,
        notarization_log_id: String,
        gatekeeper_assessment: String,
    },
}

impl SignatureState {
    #[must_use]
    pub const fn unsigned_dev() -> Self {
        Self::UnsignedDev
    }

    #[must_use]
    pub fn signed(identity: impl Into<String>, signature_ref: impl Into<String>) -> Self {
        Self::Signed {
            identity: identity.into(),
            signature_ref: signature_ref.into(),
        }
    }

    #[must_use]
    pub fn notarized(
        identity: impl Into<String>,
        signature_ref: impl Into<String>,
        notarization_log_id: impl Into<String>,
        gatekeeper_assessment: impl Into<String>,
    ) -> Self {
        Self::Notarized {
            identity: identity.into(),
            signature_ref: signature_ref.into(),
            notarization_log_id: notarization_log_id.into(),
            gatekeeper_assessment: gatekeeper_assessment.into(),
        }
    }

    #[must_use]
    pub const fn is_publish_signed(&self) -> bool {
        matches!(self, Self::Signed { .. } | Self::Notarized { .. })
    }

    #[must_use]
    pub const fn is_notarized(&self) -> bool {
        matches!(self, Self::Notarized { .. })
    }

    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::UnsignedDev => "unsigned_dev",
            Self::Signed { .. } => "signed",
            Self::Notarized { .. } => "notarized",
        }
    }

    fn validate(&self) -> Result<(), ManifestEntryError> {
        match self {
            Self::UnsignedDev => Ok(()),
            Self::Signed {
                identity,
                signature_ref,
            } => {
                reject_blank("signatureState.identity", identity)?;
                reject_blank("signatureState.signatureRef", signature_ref)
            }
            Self::Notarized {
                identity,
                signature_ref,
                notarization_log_id,
                gatekeeper_assessment,
            } => {
                reject_blank("signatureState.identity", identity)?;
                reject_blank("signatureState.signatureRef", signature_ref)?;
                reject_blank("signatureState.notarizationLogId", notarization_log_id)?;
                reject_blank("signatureState.gatekeeperAssessment", gatekeeper_assessment)
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildSource {
    commit_sha: String,
    workflow: String,
    runner: String,
}

impl BuildSource {
    #[must_use]
    pub fn new(
        commit_sha: impl Into<String>,
        workflow: impl Into<String>,
        runner: impl Into<String>,
    ) -> Self {
        Self {
            commit_sha: commit_sha.into(),
            workflow: workflow.into(),
            runner: runner.into(),
        }
    }

    #[must_use]
    pub fn commit_sha(&self) -> &str {
        &self.commit_sha
    }

    #[must_use]
    pub fn workflow(&self) -> &str {
        &self.workflow
    }

    #[must_use]
    pub fn runner(&self) -> &str {
        &self.runner
    }

    fn validate(&self) -> Result<(), ManifestEntryError> {
        if !is_hex_len(&self.commit_sha, 40) {
            return Err("buildSource.commitSha must be a 40 character hex digest".to_string());
        }

        reject_blank("buildSource.workflow", &self.workflow)?;
        reject_blank("buildSource.runner", &self.runner)
    }
}

pub type ManifestEntryError = String;

fn validate_sha256(value: &str) -> Result<(), ManifestEntryError> {
    if !is_lowercase_hex_len(value, 64) {
        return Err("sha256 must be a 64 character lowercase hex digest".to_string());
    }

    Ok(())
}

fn is_hex_len(value: &str, len: usize) -> bool {
    value.len() == len && value.chars().all(|character| character.is_ascii_hexdigit())
}

fn is_lowercase_hex_len(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}

fn reject_blank(field: &'static str, value: &str) -> Result<(), ManifestEntryError> {
    if value.trim().is_empty() {
        return Err(format!("{field} must not be blank"));
    }

    Ok(())
}
