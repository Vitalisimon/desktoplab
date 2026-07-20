use std::fmt;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct JobId(String);

impl JobId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for JobId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JobState {
    Queued,
    Running,
    AwaitingApproval,
    Succeeded,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Job {
    id: JobId,
    kind: String,
    state: JobState,
}

impl Job {
    #[must_use]
    pub fn new(id: JobId, kind: impl Into<String>) -> Self {
        Self {
            id,
            kind: kind.into(),
            state: JobState::Queued,
        }
    }

    #[must_use]
    pub fn id(&self) -> &JobId {
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

    pub fn set_state(&mut self, state: JobState) {
        self.state = state;
    }
}
