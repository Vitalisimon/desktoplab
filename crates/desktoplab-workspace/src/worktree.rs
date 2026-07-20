#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParallelExecutionKind {
    WriteCapableParallel,
    ReadOnlyParallel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IsolationDecision {
    RequiresWorktree,
    CanShareWorkspace,
}

#[derive(Default)]
pub struct WorktreePolicy;

impl WorktreePolicy {
    #[must_use]
    pub fn strict() -> Self {
        Self
    }

    #[must_use]
    pub fn evaluate(&self, execution_kind: ParallelExecutionKind) -> IsolationDecision {
        match execution_kind {
            ParallelExecutionKind::WriteCapableParallel => IsolationDecision::RequiresWorktree,
            ParallelExecutionKind::ReadOnlyParallel => IsolationDecision::CanShareWorkspace,
        }
    }
}
