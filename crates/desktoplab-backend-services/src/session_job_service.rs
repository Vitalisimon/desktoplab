use desktoplab_agent_session::{SessionEvent, SessionReplay};

use crate::sessions::SessionService;

impl SessionService {
    pub fn start_job(
        &mut self,
        session_id: &str,
        job_id: impl Into<String>,
        started_at: impl Into<String>,
        cancellable: bool,
    ) {
        self.append(
            session_id,
            SessionEvent::job_started(job_id, started_at, cancellable),
        );
    }

    pub fn heartbeat_job(
        &mut self,
        session_id: &str,
        job_id: impl Into<String>,
        at: impl Into<String>,
    ) {
        self.append(session_id, SessionEvent::job_heartbeat(job_id, at));
    }

    pub fn observe_job(
        &mut self,
        session_id: &str,
        job_id: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.append(session_id, SessionEvent::job_observation(job_id, message));
    }

    pub fn interrupt_running_jobs(
        &mut self,
        reason: impl Into<String>,
        guidance: impl Into<String>,
    ) {
        let reason = reason.into();
        let guidance = guidance.into();
        let interrupted = {
            let data = self
                .store
                .inner
                .lock()
                .expect("session store lock should not be poisoned");
            data.records
                .iter()
                .filter_map(|record| SessionReplay::replay(record.events.clone()).ok())
                .filter_map(|session| {
                    let job = session.job()?;
                    (job.state() == "running")
                        .then(|| (session.session_id().to_string(), job.job_id().to_string()))
                })
                .collect::<Vec<_>>()
        };
        for (session_id, job_id) in interrupted {
            self.append(
                &session_id,
                SessionEvent::job_interrupted(
                    job_id,
                    reason.clone(),
                    guidance.clone(),
                    current_timestamp(),
                ),
            );
        }
    }
}

fn current_timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}
