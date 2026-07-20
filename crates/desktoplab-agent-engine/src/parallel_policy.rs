#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentParallelIntent {
    ReadOnly,
    WriteCapable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentParallelDecision {
    CanShareWorkspace,
    RequiresWorktree,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentParallelPolicy;

impl AgentParallelPolicy {
    #[must_use]
    pub fn strict() -> Self {
        Self
    }

    #[must_use]
    pub fn decide(&self, intent: AgentParallelIntent) -> AgentParallelDecision {
        match intent {
            AgentParallelIntent::ReadOnly => AgentParallelDecision::CanShareWorkspace,
            AgentParallelIntent::WriteCapable => AgentParallelDecision::RequiresWorktree,
        }
    }

    #[must_use]
    pub fn ui_allows_shared_parallel_mode(&self, intent: AgentParallelIntent) -> bool {
        self.decide(intent) == AgentParallelDecision::CanShareWorkspace
    }
}
