use std::path::Path;

use desktoplab_policy::{Action, PolicyEngine};

use crate::{ToolGateway, ToolIntent, ToolOutcome, WorkspaceRoot};

const GENERATED_ARTIFACT_WRITE_BUDGET_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilesystemApproval {
    Pending,
    Approved,
    Denied,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FilesystemToolOutcome {
    Read(String),
    Written,
    Unchanged,
    ApprovalRequired,
    Denied,
    Blocked(&'static str),
}

pub struct FilesystemToolExecutor {
    root: Option<WorkspaceRoot>,
    gateway: ToolGateway,
}

impl FilesystemToolExecutor {
    #[must_use]
    pub fn new(root: &Path, policy: PolicyEngine) -> Self {
        Self {
            root: WorkspaceRoot::open(root).ok(),
            gateway: ToolGateway::new(policy),
        }
    }

    pub fn read(&mut self, path: &str) -> FilesystemToolOutcome {
        let Some(root) = &self.root else {
            return FilesystemToolOutcome::Blocked("path_escape");
        };
        match self.gateway.authorize(ToolIntent::filesystem_read(path)) {
            ToolOutcome::Blocked(_) => FilesystemToolOutcome::Blocked("local_only_path"),
            ToolOutcome::Allowed(_) | ToolOutcome::ApprovalRequired(_) => {
                match root.read_text(path) {
                    Ok(contents) => FilesystemToolOutcome::Read(contents),
                    Err(crate::WorkspaceRootError::Io(error))
                        if error.kind() == std::io::ErrorKind::NotFound =>
                    {
                        FilesystemToolOutcome::Blocked("read_failed")
                    }
                    Err(_) => FilesystemToolOutcome::Blocked("path_escape"),
                }
            }
        }
    }

    pub fn write(
        &mut self,
        path: &str,
        contents: &str,
        approval: FilesystemApproval,
    ) -> FilesystemToolOutcome {
        let Some(root) = &self.root else {
            return FilesystemToolOutcome::Blocked("path_escape");
        };
        if is_generated_artifact_path(path)
            && contents.len() > GENERATED_ARTIFACT_WRITE_BUDGET_BYTES
        {
            return FilesystemToolOutcome::Blocked("generated_artifact_budget_exceeded");
        }

        match self.gateway.authorize(ToolIntent::filesystem_write(path)) {
            ToolOutcome::Blocked(_) => return FilesystemToolOutcome::Blocked("local_only_path"),
            ToolOutcome::Allowed(Action::FilesystemWrite | Action::GeneratedArtifactWrite) => {}
            ToolOutcome::ApprovalRequired(
                Action::FilesystemWrite | Action::GeneratedArtifactWrite,
            ) => match approval {
                FilesystemApproval::Pending => return FilesystemToolOutcome::ApprovalRequired,
                FilesystemApproval::Denied => return FilesystemToolOutcome::Denied,
                FilesystemApproval::Approved => {}
            },
            ToolOutcome::Allowed(_) | ToolOutcome::ApprovalRequired(_) => {
                return FilesystemToolOutcome::Blocked("unexpected_action");
            }
        }

        match root.write_text(path, contents) {
            Ok(true) => FilesystemToolOutcome::Written,
            Ok(false) => FilesystemToolOutcome::Unchanged,
            Err(_) => FilesystemToolOutcome::Blocked("path_escape"),
        }
    }

    #[must_use]
    pub fn approval_count(&self) -> usize {
        self.gateway.approval_requests().len()
    }

    #[must_use]
    pub fn audit_count(&self) -> usize {
        self.gateway.audit_records().len()
    }
}

fn is_generated_artifact_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.starts_with("dist/")
        || normalized.starts_with("build/")
        || normalized.starts_with("target/")
        || normalized.starts_with(".next/")
        || normalized.starts_with("generated/")
}
