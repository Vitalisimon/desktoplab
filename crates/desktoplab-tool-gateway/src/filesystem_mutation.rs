use std::path::Path;

use desktoplab_policy::{Action, PolicyEngine};

use crate::{FilesystemApproval, ToolGateway, ToolIntent, ToolOutcome, WorkspaceRoot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilesystemMutationOutcome {
    Changed,
    Unchanged,
    ApprovalRequired,
    Denied,
    Blocked(String),
}

pub struct FilesystemMutationExecutor {
    root: Option<WorkspaceRoot>,
    gateway: ToolGateway,
}

impl FilesystemMutationExecutor {
    #[must_use]
    pub fn new(root: &Path, policy: PolicyEngine) -> Self {
        Self {
            root: WorkspaceRoot::open(root).ok(),
            gateway: ToolGateway::new(policy),
        }
    }

    pub fn create_directory(
        &mut self,
        path: &str,
        approval: FilesystemApproval,
    ) -> FilesystemMutationOutcome {
        let intent = ToolIntent::filesystem_create_directory(path);
        if let Some(outcome) = self.authorize(intent, approval) {
            return outcome;
        }
        match self.root.as_ref().unwrap().create_directory(path) {
            Ok(true) => FilesystemMutationOutcome::Changed,
            Ok(false) => FilesystemMutationOutcome::Unchanged,
            Err(error) => FilesystemMutationOutcome::Blocked(error.to_string()),
        }
    }

    pub fn move_path(
        &mut self,
        source: &str,
        destination: &str,
        approval: FilesystemApproval,
    ) -> FilesystemMutationOutcome {
        let intent = ToolIntent::filesystem_move(source, destination);
        if let Some(outcome) = self.authorize(intent, approval) {
            return outcome;
        }
        match self.root.as_ref().unwrap().move_path(source, destination) {
            Ok(()) => FilesystemMutationOutcome::Changed,
            Err(error) => FilesystemMutationOutcome::Blocked(error.to_string()),
        }
    }

    pub fn delete_path(
        &mut self,
        path: &str,
        recursive: bool,
        approval: FilesystemApproval,
    ) -> FilesystemMutationOutcome {
        let intent = ToolIntent::filesystem_delete(path, recursive);
        if let Some(outcome) = self.authorize(intent, approval) {
            return outcome;
        }
        match self.root.as_ref().unwrap().delete_path(path, recursive) {
            Ok(()) => FilesystemMutationOutcome::Changed,
            Err(error) => FilesystemMutationOutcome::Blocked(error.to_string()),
        }
    }

    fn authorize(
        &mut self,
        intent: ToolIntent,
        approval: FilesystemApproval,
    ) -> Option<FilesystemMutationOutcome> {
        if self.root.is_none() {
            return Some(FilesystemMutationOutcome::Blocked(
                "path_escape".to_string(),
            ));
        }
        match self.gateway.authorize(intent) {
            ToolOutcome::Allowed(Action::FilesystemWrite) => None,
            ToolOutcome::ApprovalRequired(Action::FilesystemWrite) => match approval {
                FilesystemApproval::Pending => Some(FilesystemMutationOutcome::ApprovalRequired),
                FilesystemApproval::Denied => Some(FilesystemMutationOutcome::Denied),
                FilesystemApproval::Approved => None,
            },
            ToolOutcome::Blocked(reason) => Some(FilesystemMutationOutcome::Blocked(reason)),
            ToolOutcome::Allowed(_) | ToolOutcome::ApprovalRequired(_) => Some(
                FilesystemMutationOutcome::Blocked("unexpected_action".to_string()),
            ),
        }
    }
}
