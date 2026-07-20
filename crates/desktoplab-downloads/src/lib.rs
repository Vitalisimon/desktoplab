#![forbid(unsafe_code)]

mod productization;

pub use productization::{ProductDownloadReservation, ResumableDownloadPlan};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadRequest {
    download_id: String,
    url: String,
    size_bytes: u64,
    expected_checksum: Option<String>,
}

impl DownloadRequest {
    #[must_use]
    pub fn new(download_id: impl Into<String>, url: impl Into<String>, size_bytes: u64) -> Self {
        Self {
            download_id: download_id.into(),
            url: url.into(),
            size_bytes,
            expected_checksum: None,
        }
    }

    #[must_use]
    pub fn with_expected_checksum(mut self, checksum: impl Into<String>) -> Self {
        self.expected_checksum = Some(checksum.into());
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownloadCapacity {
    disk_available_bytes: u64,
}

impl DownloadCapacity {
    #[must_use]
    pub fn new(disk_available_bytes: u64) -> Self {
        Self {
            disk_available_bytes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadSignatureDecision {
    Trusted,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DownloadError {
    ChecksumMismatch { expected: String, actual: String },
    InsufficientDisk { required: u64, available: u64 },
    SignatureRejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadJobState {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadJob {
    request: DownloadRequest,
    state: DownloadJobState,
    progress_bytes: u64,
    events: Vec<DownloadEvent>,
}

impl DownloadJob {
    pub fn start(
        request: DownloadRequest,
        capacity: DownloadCapacity,
        signature: DownloadSignatureDecision,
    ) -> Result<Self, DownloadError> {
        if request.size_bytes > capacity.disk_available_bytes {
            return Err(DownloadError::InsufficientDisk {
                required: request.size_bytes,
                available: capacity.disk_available_bytes,
            });
        }

        if signature == DownloadSignatureDecision::Rejected {
            return Err(DownloadError::SignatureRejected);
        }

        let mut job = Self {
            request,
            state: DownloadJobState::Queued,
            progress_bytes: 0,
            events: Vec::new(),
        };
        job.push_event(DownloadEventKind::Queued);
        job.state = DownloadJobState::Running;
        job.push_event(DownloadEventKind::Started);
        Ok(job)
    }

    #[must_use]
    pub fn from_metadata(metadata: DownloadMetadata) -> Self {
        Self {
            request: metadata.request,
            state: metadata.state,
            progress_bytes: metadata.progress_bytes,
            events: metadata.events,
        }
    }

    pub fn record_progress(&mut self, downloaded_bytes: u64) {
        self.progress_bytes = downloaded_bytes;
        self.push_event(DownloadEventKind::Progress);
    }

    pub fn complete_with_checksum(
        &mut self,
        actual: impl Into<String>,
    ) -> Result<(), DownloadError> {
        let actual = actual.into();
        if let Some(expected) = self.request.expected_checksum.clone() {
            if expected != actual {
                self.state = DownloadJobState::Failed;
                self.push_event(DownloadEventKind::Failed);
                return Err(DownloadError::ChecksumMismatch { expected, actual });
            }
        }

        self.state = DownloadJobState::Completed;
        self.progress_bytes = self.request.size_bytes;
        self.push_event(DownloadEventKind::Completed);
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.state = DownloadJobState::Cancelled;
        self.push_event(DownloadEventKind::Cancelled);
    }

    #[must_use]
    pub fn metadata(&self) -> DownloadMetadata {
        DownloadMetadata {
            request: self.request.clone(),
            state: self.state,
            progress_bytes: self.progress_bytes,
            events: self.events.clone(),
        }
    }

    #[must_use]
    pub fn state(&self) -> DownloadJobState {
        self.state
    }

    #[must_use]
    pub fn progress_bytes(&self) -> u64 {
        self.progress_bytes
    }

    #[must_use]
    pub fn event_sequences(&self) -> Vec<u64> {
        self.events.iter().map(DownloadEvent::sequence).collect()
    }

    fn push_event(&mut self, kind: DownloadEventKind) {
        self.events.push(DownloadEvent {
            sequence: self.events.len() as u64 + 1,
            kind,
        });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadMetadata {
    request: DownloadRequest,
    state: DownloadJobState,
    progress_bytes: u64,
    events: Vec<DownloadEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadEvent {
    sequence: u64,
    kind: DownloadEventKind,
}

impl DownloadEvent {
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn kind(&self) -> DownloadEventKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadEventKind {
    Queued,
    Started,
    Progress,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadFailureClass {
    Retryable,
    NonRetryable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DownloadFailure {
    class: DownloadFailureClass,
}

impl DownloadFailure {
    #[must_use]
    pub fn network_timeout() -> Self {
        Self {
            class: DownloadFailureClass::Retryable,
        }
    }

    #[must_use]
    pub fn checksum_mismatch() -> Self {
        Self {
            class: DownloadFailureClass::NonRetryable,
        }
    }

    #[must_use]
    pub fn class(&self) -> DownloadFailureClass {
        self.class
    }
}
