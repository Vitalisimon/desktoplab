use desktoplab_domain::WorkspaceId;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRegistration {
    workspace_id: WorkspaceId,
    root_path: PathBuf,
}

impl WorkspaceRegistration {
    #[must_use]
    pub fn new(workspace_id: WorkspaceId, root_path: PathBuf) -> Self {
        Self {
            workspace_id,
            root_path,
        }
    }

    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    #[must_use]
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}

#[derive(Default)]
pub struct WorkspaceRegistry {
    workspaces: HashMap<WorkspaceId, WorkspaceRegistration>,
}

impl WorkspaceRegistry {
    pub fn register(&mut self, registration: WorkspaceRegistration) {
        self.workspaces
            .insert(registration.workspace_id().clone(), registration);
    }

    #[must_use]
    pub fn get(&self, workspace_id: &WorkspaceId) -> Option<&WorkspaceRegistration> {
        self.workspaces.get(workspace_id)
    }
}
