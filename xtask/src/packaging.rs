use crate::check_logical_line_limit;

const SUPPORTED_TARGETS: &[&str] = &[
    "macos-universal",
    "macos-aarch64",
    "macos-x64",
    "windows-x64",
    "linux-x64",
];

const SUPPORTED_CHANNELS: &[&str] = &["dev", "beta", "stable"];

pub const PACKAGING_MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagingArtifactRecord {
    file_name: String,
    target: String,
    channel: String,
    sha256: String,
    size_bytes: u64,
    signature_state: PackagingSignatureState,
}

impl PackagingArtifactRecord {
    #[must_use]
    pub fn new(
        file_name: impl Into<String>,
        target: impl Into<String>,
        channel: impl Into<String>,
        sha256: impl Into<String>,
        size_bytes: u64,
        signature_state: PackagingSignatureState,
    ) -> Self {
        Self {
            file_name: file_name.into(),
            target: target.into(),
            channel: channel.into(),
            sha256: sha256.into(),
            size_bytes,
            signature_state,
        }
    }

    #[must_use]
    pub fn with_signature_state(mut self, signature_state: PackagingSignatureState) -> Self {
        self.signature_state = signature_state;
        self
    }

    #[must_use]
    pub fn with_sha256(mut self, sha256: impl Into<String>) -> Self {
        self.sha256 = sha256.into();
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackagingSignatureState {
    Unsigned,
    Signed,
    Notarized,
}

impl PackagingSignatureState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsigned => "unsigned_dev",
            Self::Signed => "signed",
            Self::Notarized => "notarized",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateChannelManifest {
    channel: String,
    target: String,
    url: String,
    signature_state: PackagingSignatureState,
    rollback: bool,
}

impl UpdateChannelManifest {
    #[must_use]
    pub fn new(
        channel: impl Into<String>,
        target: impl Into<String>,
        url: impl Into<String>,
        signature_state: PackagingSignatureState,
        rollback: bool,
    ) -> Self {
        Self {
            channel: channel.into(),
            target: target.into(),
            url: url.into(),
            signature_state,
            rollback,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UpdateManifestPolicyViolation {
    InvalidChannel,
    InvalidTarget,
    UnsignedRemoteDev,
    UnsignedReleaseChannel,
    UnsignedRollback,
}

pub struct UpdateManifestPolicy;

impl UpdateManifestPolicy {
    pub fn verify(manifest: &UpdateChannelManifest) -> Result<(), UpdateManifestPolicyViolation> {
        verify_channel_and_target(&manifest.channel, &manifest.target)?;
        if manifest.rollback && manifest.signature_state == PackagingSignatureState::Unsigned {
            return Err(UpdateManifestPolicyViolation::UnsignedRollback);
        }
        if manifest.channel == "dev"
            && manifest.signature_state == PackagingSignatureState::Unsigned
            && !manifest.url.starts_with("file://")
        {
            return Err(UpdateManifestPolicyViolation::UnsignedRemoteDev);
        }
        if manifest.channel != "dev"
            && manifest.signature_state == PackagingSignatureState::Unsigned
        {
            return Err(UpdateManifestPolicyViolation::UnsignedReleaseChannel);
        }
        Ok(())
    }
}

pub fn channel_manifest_path(
    channel: &str,
    target: &str,
) -> Result<String, UpdateManifestPolicyViolation> {
    verify_channel_and_target(channel, target)?;
    Ok(format!("channels/{channel}/{target}/manifest.json"))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagingSourceBudget {
    path: String,
    source: String,
    max_lines: usize,
}

impl PackagingSourceBudget {
    #[must_use]
    pub fn new(path: impl Into<String>, source: impl Into<String>, max_lines: usize) -> Self {
        Self {
            path: path.into(),
            source: source.into(),
            max_lines,
        }
    }
}

pub struct PackagingGate;

impl PackagingGate {
    pub fn verify_artifacts(
        records: &[PackagingArtifactRecord],
    ) -> Result<(), PackagingGateViolation> {
        for record in records {
            verify_artifact(record)?;
        }

        Ok(())
    }

    pub fn verify_source_budgets(
        budgets: &[PackagingSourceBudget],
    ) -> Result<(), PackagingGateViolation> {
        for budget in budgets {
            check_logical_line_limit(&budget.path, &budget.source, budget.max_lines).map_err(
                |violation| PackagingGateViolation::LineBudgetExceeded {
                    path: violation.path,
                    logical_lines: violation.logical_lines,
                    max_lines: violation.max_lines,
                },
            )?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PackagingGateViolation {
    InvalidTarget {
        file_name: String,
        target: String,
    },
    InvalidChannel {
        file_name: String,
        channel: String,
    },
    InvalidSha256 {
        file_name: String,
    },
    EmptyArtifact {
        file_name: String,
    },
    UnsignedReleaseArtifact {
        file_name: String,
        channel: String,
    },
    LineBudgetExceeded {
        path: String,
        logical_lines: usize,
        max_lines: usize,
    },
}

fn verify_artifact(record: &PackagingArtifactRecord) -> Result<(), PackagingGateViolation> {
    if !SUPPORTED_TARGETS.contains(&record.target.as_str()) {
        return Err(PackagingGateViolation::InvalidTarget {
            file_name: record.file_name.clone(),
            target: record.target.clone(),
        });
    }

    if !SUPPORTED_CHANNELS.contains(&record.channel.as_str()) {
        return Err(PackagingGateViolation::InvalidChannel {
            file_name: record.file_name.clone(),
            channel: record.channel.clone(),
        });
    }

    if !is_sha256(&record.sha256) {
        return Err(PackagingGateViolation::InvalidSha256 {
            file_name: record.file_name.clone(),
        });
    }

    if record.size_bytes == 0 {
        return Err(PackagingGateViolation::EmptyArtifact {
            file_name: record.file_name.clone(),
        });
    }

    if record.channel != "dev" && record.signature_state == PackagingSignatureState::Unsigned {
        return Err(PackagingGateViolation::UnsignedReleaseArtifact {
            file_name: record.file_name.clone(),
            channel: record.channel.clone(),
        });
    }

    Ok(())
}

fn verify_channel_and_target(
    channel: &str,
    target: &str,
) -> Result<(), UpdateManifestPolicyViolation> {
    if !SUPPORTED_CHANNELS.contains(&channel) {
        return Err(UpdateManifestPolicyViolation::InvalidChannel);
    }
    if !SUPPORTED_TARGETS.contains(&target) {
        return Err(UpdateManifestPolicyViolation::InvalidTarget);
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| matches!(character, '0'..='9' | 'a'..='f'))
}
