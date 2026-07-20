#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointStatus {
    Ready,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointPlan {
    status: CheckpointStatus,
}

impl CheckpointPlan {
    #[must_use]
    pub fn ready() -> Self {
        Self {
            status: CheckpointStatus::Ready,
        }
    }

    #[must_use]
    pub fn status(&self) -> CheckpointStatus {
        self.status
    }

    #[must_use]
    pub fn can_continue_with_risky_execution(&self) -> bool {
        true
    }
}
