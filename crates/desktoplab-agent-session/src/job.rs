#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentJobSnapshot {
    job_id: String,
    state: String,
    started_at: String,
    last_heartbeat_at: Option<String>,
    last_observation: Option<String>,
    cancellable: bool,
    recovery_guidance: Option<String>,
}

impl AgentJobSnapshot {
    #[must_use]
    pub fn running(
        job_id: impl Into<String>,
        started_at: impl Into<String>,
        cancellable: bool,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            state: "running".to_string(),
            started_at: started_at.into(),
            last_heartbeat_at: None,
            last_observation: None,
            cancellable,
            recovery_guidance: None,
        }
    }

    pub fn heartbeat(&mut self, at: impl Into<String>) {
        self.last_heartbeat_at = Some(at.into());
    }

    pub fn observe(&mut self, message: impl Into<String>) {
        self.last_observation = Some(message.into());
    }

    pub fn interrupt(&mut self, guidance: impl Into<String>) {
        self.state = "interrupted".to_string();
        self.cancellable = false;
        self.recovery_guidance = Some(guidance.into());
    }

    pub fn cancel(&mut self) {
        self.state = "cancelled".to_string();
        self.cancellable = false;
    }

    pub fn pause(&mut self, state: &str) {
        self.state = state.to_string();
        self.cancellable = false;
    }

    pub fn resume(&mut self) {
        self.state = "running".to_string();
        self.cancellable = true;
    }

    pub fn complete(&mut self) {
        self.state = "completed".to_string();
        self.cancellable = false;
    }

    pub fn fail(&mut self) {
        self.state = "failed".to_string();
        self.cancellable = false;
    }

    #[must_use]
    pub fn job_id(&self) -> &str {
        &self.job_id
    }
    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }
    #[must_use]
    pub fn started_at(&self) -> &str {
        &self.started_at
    }
    #[must_use]
    pub fn last_heartbeat_at(&self) -> Option<&str> {
        self.last_heartbeat_at.as_deref()
    }
    #[must_use]
    pub fn last_observation(&self) -> Option<&str> {
        self.last_observation.as_deref()
    }
    #[must_use]
    pub fn cancellable(&self) -> bool {
        self.cancellable
    }
    #[must_use]
    pub fn recovery_guidance(&self) -> Option<&str> {
        self.recovery_guidance.as_deref()
    }
}
