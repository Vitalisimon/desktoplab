use crate::{ArtifactManifestEntry, PlatformTarget, ReleaseChannel};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAdapterReleaseSpec {
    adapter_id: String,
    targets: Vec<PlatformTarget>,
}

impl ProviderAdapterReleaseSpec {
    #[must_use]
    pub fn new(adapter_id: impl Into<String>, targets: Vec<PlatformTarget>) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            targets,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderProbeReleaseEvidence {
    adapter_id: String,
    target: PlatformTarget,
    passed: bool,
    observed_at: u64,
    fresh_until: u64,
    evidence_ref: String,
    artifact_sha256: String,
}

impl ProviderProbeReleaseEvidence {
    #[must_use]
    pub fn new(
        adapter_id: impl Into<String>,
        target: PlatformTarget,
        passed: bool,
        observed_at: u64,
        fresh_until: u64,
        evidence_ref: impl Into<String>,
        artifact_sha256: impl Into<String>,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            target,
            passed,
            observed_at,
            fresh_until,
            evidence_ref: evidence_ref.into(),
            artifact_sha256: artifact_sha256.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderReleaseStatus {
    Ready,
    Unsupported,
    ArtifactMissing,
    ProbeMissing,
    ProbeFailed,
    ProbeStale,
    ProbeArtifactMismatch,
    ArtifactUnpublishable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderReleaseMatrixEntry {
    adapter_id: String,
    target: PlatformTarget,
    status: ProviderReleaseStatus,
    artifact_sha256: Option<String>,
    probe_evidence_ref: Option<String>,
}

impl ProviderReleaseMatrixEntry {
    #[must_use]
    pub fn adapter_id(&self) -> &str {
        &self.adapter_id
    }
    #[must_use]
    pub fn target(&self) -> PlatformTarget {
        self.target
    }
    #[must_use]
    pub fn status(&self) -> ProviderReleaseStatus {
        self.status
    }
    #[must_use]
    pub fn artifact_sha256(&self) -> Option<&str> {
        self.artifact_sha256.as_deref()
    }
    #[must_use]
    pub fn probe_evidence_ref(&self) -> Option<&str> {
        self.probe_evidence_ref.as_deref()
    }
}

pub struct ProviderReleaseMatrix;

impl ProviderReleaseMatrix {
    #[must_use]
    pub fn build(
        specs: &[ProviderAdapterReleaseSpec],
        artifacts: &[ArtifactManifestEntry],
        probes: &[ProviderProbeReleaseEvidence],
        version: &str,
        channel: ReleaseChannel,
        now: u64,
    ) -> Vec<ProviderReleaseMatrixEntry> {
        let mut entries = Vec::new();
        for spec in specs {
            for target in PlatformTarget::supported() {
                let artifact = artifacts.iter().find(|entry| {
                    entry.target() == *target
                        && entry.version() == version
                        && entry.channel() == channel
                });
                let probe = probes
                    .iter()
                    .filter(|entry| entry.adapter_id == spec.adapter_id && entry.target == *target)
                    .max_by_key(|entry| entry.observed_at);
                let supported = spec.targets.contains(target);
                let status = if !supported {
                    ProviderReleaseStatus::Unsupported
                } else if artifact.is_none() {
                    ProviderReleaseStatus::ArtifactMissing
                } else if artifact.expect("checked").validate_for_publish().is_err() {
                    ProviderReleaseStatus::ArtifactUnpublishable
                } else if probe.is_none() {
                    ProviderReleaseStatus::ProbeMissing
                } else if !probe.expect("checked").passed
                    || probe.expect("checked").evidence_ref.trim().is_empty()
                    || secret_like(&probe.expect("checked").evidence_ref)
                {
                    ProviderReleaseStatus::ProbeFailed
                } else if probe.expect("checked").observed_at > now
                    || now > probe.expect("checked").fresh_until
                {
                    ProviderReleaseStatus::ProbeStale
                } else if probe.expect("checked").artifact_sha256
                    != artifact.expect("checked").sha256()
                {
                    ProviderReleaseStatus::ProbeArtifactMismatch
                } else {
                    ProviderReleaseStatus::Ready
                };
                entries.push(ProviderReleaseMatrixEntry {
                    adapter_id: spec.adapter_id.clone(),
                    target: *target,
                    status,
                    artifact_sha256: artifact.map(|entry| entry.sha256().to_string()),
                    probe_evidence_ref: probe.map(|entry| entry.evidence_ref.clone()),
                });
            }
        }
        entries.sort_by(|left, right| {
            (left.adapter_id.as_str(), left.target.as_str())
                .cmp(&(right.adapter_id.as_str(), right.target.as_str()))
        });
        entries
    }
}

fn secret_like(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.contains("token=")
        || value.contains("password=")
        || value.contains("cookie=")
        || value.contains("bearer ")
        || value.contains("sk-")
}
