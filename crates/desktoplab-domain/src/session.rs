use crate::{ExecutionBackendId, SessionId, WorkspaceId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionOwner {
    DesktopLab,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workspace {
    id: WorkspaceId,
}

impl Workspace {
    #[must_use]
    pub fn new(id: WorkspaceId) -> Self {
        Self { id }
    }

    #[must_use]
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Session {
    id: SessionId,
    workspace_id: WorkspaceId,
    execution_backend_id: ExecutionBackendId,
    owner: SessionOwner,
}

impl Session {
    #[must_use]
    pub fn new(
        id: SessionId,
        workspace_id: WorkspaceId,
        execution_backend_id: ExecutionBackendId,
    ) -> Self {
        Self {
            id,
            workspace_id,
            execution_backend_id,
            owner: SessionOwner::DesktopLab,
        }
    }

    #[must_use]
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    #[must_use]
    pub fn execution_backend_id(&self) -> &ExecutionBackendId {
        &self.execution_backend_id
    }

    #[must_use]
    pub fn owner(&self) -> SessionOwner {
        self.owner
    }
}
