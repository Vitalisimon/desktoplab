#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeDownloadRetryClass {
    Retryable,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeDownloadFailureKind {
    NetworkUnavailable,
    SourceUnavailable,
    ChecksumMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDownloadFailure {
    kind: RuntimeDownloadFailureKind,
}

impl RuntimeDownloadFailure {
    #[must_use]
    pub fn new(kind: RuntimeDownloadFailureKind) -> Self {
        Self { kind }
    }

    #[must_use]
    pub fn kind(&self) -> RuntimeDownloadFailureKind {
        self.kind
    }

    #[must_use]
    pub fn retry_class(&self) -> RuntimeDownloadRetryClass {
        match self.kind {
            RuntimeDownloadFailureKind::NetworkUnavailable
            | RuntimeDownloadFailureKind::SourceUnavailable => RuntimeDownloadRetryClass::Retryable,
            RuntimeDownloadFailureKind::ChecksumMismatch => RuntimeDownloadRetryClass::Blocked,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeCachedInstallerArtifact {
    uri: String,
    checksum: String,
    verified: bool,
}

impl RuntimeCachedInstallerArtifact {
    #[must_use]
    pub fn verified(uri: impl Into<String>, checksum: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            checksum: checksum.into(),
            verified: true,
        }
    }

    #[must_use]
    pub fn is_verified(&self) -> bool {
        self.verified && !self.uri.trim().is_empty() && !self.checksum.trim().is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDownloadVerification {
    expected_checksum: String,
    actual_checksum: String,
}

impl RuntimeDownloadVerification {
    #[must_use]
    pub fn new(expected_checksum: impl Into<String>, actual_checksum: impl Into<String>) -> Self {
        Self {
            expected_checksum: expected_checksum.into(),
            actual_checksum: actual_checksum.into(),
        }
    }

    pub fn verify(&self) -> Result<(), RuntimeDownloadFailure> {
        if self.expected_checksum == self.actual_checksum {
            return Ok(());
        }
        Err(RuntimeDownloadFailure::new(
            RuntimeDownloadFailureKind::ChecksumMismatch,
        ))
    }
}
