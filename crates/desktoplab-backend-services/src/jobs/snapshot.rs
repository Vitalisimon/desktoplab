use std::sync::{Arc, Mutex};

use desktoplab_events::{Job, JobId};

use super::{JobRecord, JobServiceData, JobServiceStore, JobState};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JobSnapshot {
    id: String,
    kind: String,
    state: JobState,
}

impl JobSnapshot {
    #[must_use]
    pub fn new(id: impl Into<String>, kind: impl Into<String>, state: JobState) -> Self {
        Self {
            id: id.into(),
            kind: kind.into(),
            state,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn state(&self) -> JobState {
        self.state
    }
}

impl JobServiceStore {
    #[must_use]
    pub fn from_snapshots(snapshots: impl IntoIterator<Item = JobSnapshot>) -> Self {
        let mut data = JobServiceData::default();
        for snapshot in snapshots {
            let mut job = Job::new(JobId::new(snapshot.id.clone()), snapshot.kind);
            job.set_state(snapshot.state);
            data.next_job_number = data.next_job_number.max(job_number(&snapshot.id));
            data.jobs.push(JobRecord {
                job,
                retry_class: None,
            });
        }
        Self {
            inner: Arc::new(Mutex::new(data)),
        }
    }
}

fn job_number(job_id: &str) -> u64 {
    job_id
        .strip_prefix("job.")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0)
}
