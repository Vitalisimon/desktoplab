use crate::{FailureReason, JobId};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobEventKind {
    Queued,
    Started,
    Progress,
    Succeeded,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Progress {
    percent: u8,
    message: String,
}

impl Progress {
    #[must_use]
    pub fn new(percent: u8, message: impl Into<String>) -> Self {
        Self {
            percent,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn percent(&self) -> u8 {
        self.percent
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobEvent {
    job_id: JobId,
    sequence: u64,
    kind: JobEventKind,
    progress: Option<Progress>,
    failure: Option<FailureReason>,
    message: Option<String>,
}

impl JobEvent {
    #[must_use]
    pub fn new(job_id: JobId, sequence: u64, kind: JobEventKind) -> Self {
        Self {
            job_id,
            sequence,
            kind,
            progress: None,
            failure: None,
            message: None,
        }
    }

    #[must_use]
    pub fn with_progress(mut self, progress: Progress) -> Self {
        self.progress = Some(progress);
        self
    }

    #[must_use]
    pub fn with_failure(mut self, failure: FailureReason) -> Self {
        self.failure = Some(failure);
        self
    }

    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    #[must_use]
    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }

    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn kind(&self) -> JobEventKind {
        self.kind
    }

    #[must_use]
    pub fn progress(&self) -> Option<&Progress> {
        self.progress.as_ref()
    }

    #[must_use]
    pub fn failure(&self) -> Option<&FailureReason> {
        self.failure.as_ref()
    }

    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }
}

#[derive(Default)]
pub struct EventStream {
    events_by_job: HashMap<JobId, Vec<JobEvent>>,
}

impl EventStream {
    pub fn append(&mut self, job_id: JobId, kind: JobEventKind) -> JobEvent {
        let sequence = self.next_sequence(&job_id);
        let event = JobEvent::new(job_id.clone(), sequence, kind);
        self.events_by_job
            .entry(job_id)
            .or_default()
            .push(event.clone());
        event
    }

    pub fn append_event(&mut self, event: JobEvent) {
        self.events_by_job
            .entry(event.job_id().clone())
            .or_default()
            .push(event);
    }

    pub fn record_for_test(&mut self, job_id: JobId, sequence: u64, kind: JobEventKind) {
        self.append_event(JobEvent::new(job_id, sequence, kind));
    }

    #[must_use]
    pub fn replay(&self, job_id: &JobId) -> Vec<JobEvent> {
        let mut events = self.events_by_job.get(job_id).cloned().unwrap_or_default();
        events.sort_by_key(JobEvent::sequence);
        events
    }

    fn next_sequence(&self, job_id: &JobId) -> u64 {
        self.events_by_job
            .get(job_id)
            .map(|events| events.iter().map(JobEvent::sequence).max().unwrap_or(0) + 1)
            .unwrap_or(1)
    }
}
