use crate::{EventStream, FailureReason, Job, JobEvent, JobEventKind, JobId, JobState, Progress};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq)]
pub enum JobQueueError {
    MissingJob(JobId),
}

#[derive(Default)]
pub struct JobQueue {
    next_job_number: u64,
    jobs: HashMap<JobId, Job>,
    events: EventStream,
}

impl JobQueue {
    pub fn enqueue(&mut self, kind: impl Into<String>) -> JobId {
        self.next_job_number += 1;
        let job_id = JobId::new(format!("job.{}", self.next_job_number));
        let job = Job::new(job_id.clone(), kind);
        self.jobs.insert(job_id.clone(), job);
        self.events.append(job_id.clone(), JobEventKind::Queued);
        job_id
    }

    pub fn start(&mut self, job_id: &JobId) -> Result<(), JobQueueError> {
        self.set_state(job_id, JobState::Running)?;
        self.events.append(job_id.clone(), JobEventKind::Started);
        Ok(())
    }

    pub fn progress(&mut self, job_id: &JobId, progress: Progress) {
        let event = JobEvent::new(
            job_id.clone(),
            self.events_for(job_id).len() as u64 + 1,
            JobEventKind::Progress,
        )
        .with_progress(progress);
        self.events.append_event(event);
    }

    pub fn cancel(
        &mut self,
        job_id: &JobId,
        message: impl Into<String>,
    ) -> Result<(), JobQueueError> {
        self.set_state(job_id, JobState::Cancelled)?;
        let event = JobEvent::new(
            job_id.clone(),
            self.events_for(job_id).len() as u64 + 1,
            JobEventKind::Cancelled,
        )
        .with_message(message);
        self.events.append_event(event);
        Ok(())
    }

    pub fn fail(&mut self, job_id: &JobId, failure: FailureReason) -> Result<(), JobQueueError> {
        self.set_state(job_id, JobState::Failed)?;
        self.record_failure(job_id, JobEventKind::Failed, failure);
        Ok(())
    }

    pub fn block(&mut self, job_id: &JobId, failure: FailureReason) -> Result<(), JobQueueError> {
        self.set_state(job_id, JobState::Blocked)?;
        self.record_failure(job_id, JobEventKind::Blocked, failure);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, job_id: &JobId) -> Option<&Job> {
        self.jobs.get(job_id)
    }

    #[must_use]
    pub fn events_for(&self, job_id: &JobId) -> Vec<JobEvent> {
        self.events.replay(job_id)
    }

    fn set_state(&mut self, job_id: &JobId, state: JobState) -> Result<(), JobQueueError> {
        let job = self
            .jobs
            .get_mut(job_id)
            .ok_or_else(|| JobQueueError::MissingJob(job_id.clone()))?;
        job.set_state(state);
        Ok(())
    }

    fn record_failure(&mut self, job_id: &JobId, kind: JobEventKind, failure: FailureReason) {
        let event = JobEvent::new(
            job_id.clone(),
            self.events_for(job_id).len() as u64 + 1,
            kind,
        )
        .with_failure(failure);
        self.events.append_event(event);
    }
}
