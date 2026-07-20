use std::sync::{Arc, Mutex};

use desktoplab_events::{Job, JobEvent, JobEventKind, Progress};
pub use desktoplab_events::{JobId, JobState};
use desktoplab_model_manager::ModelDownloadPlan;

mod helpers;
mod replay;
mod snapshot;

use helpers::{next_sequence, sanitize_message};
pub use replay::SseReplay;
pub use snapshot::JobSnapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobRetryClass {
    Retryable,
    NonRetryable,
}

#[derive(Clone, Debug)]
pub(crate) struct JobRecord {
    pub(crate) job: Job,
    pub(crate) retry_class: Option<JobRetryClass>,
}

#[derive(Clone, Debug, Default)]
pub struct JobServiceStore {
    pub(crate) inner: Arc<Mutex<JobServiceData>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct JobServiceData {
    pub(crate) next_job_number: u64,
    pub(crate) jobs: Vec<JobRecord>,
    pub(crate) events: Vec<JobEvent>,
}

#[derive(Clone, Debug)]
pub struct JobService {
    store: JobServiceStore,
}

impl JobService {
    #[must_use]
    pub fn new(store: JobServiceStore) -> Self {
        Self { store }
    }

    pub fn create_job(&mut self, kind: impl Into<String>) -> Job {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned");
        data.next_job_number += 1;
        let job = Job::new(JobId::new(format!("job.{}", data.next_job_number)), kind);
        data.events
            .push(JobEvent::new(job.id().clone(), 1, JobEventKind::Queued));
        data.jobs.push(JobRecord {
            job: job.clone(),
            retry_class: None,
        });
        job
    }

    pub fn create_model_download_job(&mut self, plan: &ModelDownloadPlan) -> Job {
        self.create_job(format!(
            "model.download:{}:{}:{}MB",
            plan.family_id(),
            plan.variant_id(),
            plan.expected_disk_mb()
        ))
    }

    pub fn model_download_metadata(plan: &ModelDownloadPlan) -> String {
        sanitize_message(&format!(
            "model download {} {} {}MB",
            plan.family_id(),
            plan.variant_id(),
            plan.expected_disk_mb()
        ))
    }

    pub fn start(&mut self, job_id: &JobId) -> Result<(), &'static str> {
        self.set_state(job_id, JobState::Running, JobEventKind::Started)
    }

    pub fn cancel(&mut self, job_id: &JobId) -> Result<(), &'static str> {
        self.set_state(job_id, JobState::Cancelled, JobEventKind::Cancelled)
    }

    pub fn fail(&mut self, job_id: &JobId, retry_class: JobRetryClass) -> Result<(), &'static str> {
        self.fail_with_event(job_id, retry_class, None)
    }

    pub fn fail_with_message(
        &mut self,
        job_id: &JobId,
        retry_class: JobRetryClass,
        message: impl AsRef<str>,
    ) -> Result<(), &'static str> {
        self.fail_with_event(
            job_id,
            retry_class,
            Some(sanitize_message(message.as_ref())),
        )
    }

    pub fn block_with_message(
        &mut self,
        job_id: &JobId,
        message: impl AsRef<str>,
    ) -> Result<(), &'static str> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned");
        let record = data
            .jobs
            .iter_mut()
            .find(|record| record.job.id() == job_id)
            .ok_or("job_missing")?;
        record.job.set_state(JobState::Blocked);
        let sequence = next_sequence(&data.events, job_id);
        data.events.push(
            JobEvent::new(job_id.clone(), sequence, JobEventKind::Blocked)
                .with_message(sanitize_message(message.as_ref())),
        );
        Ok(())
    }

    pub fn progress(
        &mut self,
        job_id: &JobId,
        percent: u8,
        message: impl AsRef<str>,
    ) -> Result<(), &'static str> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned");
        let record = data
            .jobs
            .iter_mut()
            .find(|record| record.job.id() == job_id)
            .ok_or("job_missing")?;
        record.job.set_state(JobState::Running);
        let sequence = next_sequence(&data.events, job_id);
        data.events.push(
            JobEvent::new(job_id.clone(), sequence, JobEventKind::Progress).with_progress(
                Progress::new(percent.min(100), sanitize_message(message.as_ref())),
            ),
        );
        Ok(())
    }

    pub fn succeed(&mut self, job_id: &JobId) -> Result<(), &'static str> {
        self.set_state(job_id, JobState::Succeeded, JobEventKind::Succeeded)
    }

    fn fail_with_event(
        &mut self,
        job_id: &JobId,
        retry_class: JobRetryClass,
        message: Option<String>,
    ) -> Result<(), &'static str> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned");
        let record = data
            .jobs
            .iter_mut()
            .find(|record| record.job.id() == job_id)
            .ok_or("job_missing")?;
        record.job.set_state(JobState::Failed);
        record.retry_class = Some(retry_class);
        let sequence = next_sequence(&data.events, job_id);
        let mut event = JobEvent::new(job_id.clone(), sequence, JobEventKind::Failed);
        if let Some(message) = message {
            event = event.with_message(message);
        }
        data.events.push(event);
        Ok(())
    }

    pub fn retry(&mut self, job_id: &JobId) -> Result<(), &'static str> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned");
        let record = data
            .jobs
            .iter_mut()
            .find(|record| record.job.id() == job_id)
            .ok_or("job_missing")?;
        if record.retry_class != Some(JobRetryClass::Retryable) {
            return Err("retry_not_allowed");
        }
        record.job.set_state(JobState::Queued);
        let sequence = next_sequence(&data.events, job_id);
        data.events.push(JobEvent::new(
            job_id.clone(),
            sequence,
            JobEventKind::Queued,
        ));
        Ok(())
    }

    #[must_use]
    pub fn get_job(&self, job_id: &JobId) -> Option<Job> {
        self.store
            .inner
            .lock()
            .expect("job store lock should not be poisoned")
            .jobs
            .iter()
            .find(|record| record.job.id() == job_id)
            .map(|record| record.job.clone())
    }

    #[must_use]
    pub fn list_jobs(&self) -> Vec<Job> {
        self.store
            .inner
            .lock()
            .expect("job store lock should not be poisoned")
            .jobs
            .iter()
            .map(|record| record.job.clone())
            .collect()
    }

    #[must_use]
    pub fn snapshot_jobs(&self) -> Vec<JobSnapshot> {
        self.store
            .inner
            .lock()
            .expect("job store lock should not be poisoned")
            .jobs
            .iter()
            .map(|record| {
                JobSnapshot::new(
                    record.job.id().as_str(),
                    record.job.kind(),
                    record.job.state(),
                )
            })
            .collect()
    }

    #[must_use]
    pub fn replay(&self, job_id: &JobId, last_seen_sequence: u64) -> SseReplay {
        let mut events: Vec<JobEvent> = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned")
            .events
            .iter()
            .filter(|event| event.job_id() == job_id && event.sequence() > last_seen_sequence)
            .cloned()
            .collect();
        events.sort_by_key(JobEvent::sequence);
        SseReplay::new(events)
    }

    fn set_state(
        &mut self,
        job_id: &JobId,
        state: JobState,
        event: JobEventKind,
    ) -> Result<(), &'static str> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("job store lock should not be poisoned");
        let record = data
            .jobs
            .iter_mut()
            .find(|record| record.job.id() == job_id)
            .ok_or("job_missing")?;
        record.job.set_state(state);
        let sequence = next_sequence(&data.events, job_id);
        data.events
            .push(JobEvent::new(job_id.clone(), sequence, event));
        Ok(())
    }
}
