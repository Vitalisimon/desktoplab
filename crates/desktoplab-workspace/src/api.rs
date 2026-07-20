use desktoplab_domain::WorkspaceId;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{CheckpointStatus, GitRepository, WorkspaceGitError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceApiState {
    Clean,
    Dirty,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceApiSnapshot {
    workspace_id: WorkspaceId,
    root_path: PathBuf,
    git_dir_path: PathBuf,
    status_entries: Vec<String>,
    diff_text: String,
    checkpoint_status: CheckpointStatus,
}

impl WorkspaceApiSnapshot {
    #[must_use]
    pub fn workspace_id(&self) -> &WorkspaceId {
        &self.workspace_id
    }

    #[must_use]
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    #[must_use]
    pub fn git_dir_path(&self) -> &Path {
        &self.git_dir_path
    }

    #[must_use]
    pub fn status_entries(&self) -> &[String] {
        &self.status_entries
    }

    #[must_use]
    pub fn diff_text(&self) -> &str {
        &self.diff_text
    }

    #[must_use]
    pub fn checkpoint_status(&self) -> CheckpointStatus {
        self.checkpoint_status
    }

    #[must_use]
    pub fn can_checkpoint_risky_execution(&self) -> bool {
        self.checkpoint_status == CheckpointStatus::Ready
    }

    #[must_use]
    pub fn api_state(&self) -> WorkspaceApiState {
        if self.status_entries.is_empty() {
            WorkspaceApiState::Clean
        } else {
            WorkspaceApiState::Dirty
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceApiErrorCode {
    NotGitRepository,
    GitCommandFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceApiError {
    code: WorkspaceApiErrorCode,
    message: String,
}

impl WorkspaceApiError {
    fn new(code: WorkspaceApiErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn code(&self) -> WorkspaceApiErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for WorkspaceApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for WorkspaceApiError {}

#[derive(Default)]
pub struct WorkspaceApiService {
    workspaces: HashMap<WorkspaceId, WorkspaceApiSnapshot>,
}

impl WorkspaceApiService {
    pub fn open_existing(
        &mut self,
        workspace_id: WorkspaceId,
        path: &Path,
    ) -> Result<WorkspaceApiSnapshot, WorkspaceApiError> {
        let repo = GitRepository::open(path).map_err(map_git_error)?;
        let snapshot = snapshot_from_repo(workspace_id, &repo)?;
        self.workspaces
            .insert(snapshot.workspace_id.clone(), snapshot.clone());
        Ok(snapshot)
    }

    pub fn create_repository(
        &mut self,
        workspace_id: WorkspaceId,
        path: &Path,
    ) -> Result<WorkspaceApiSnapshot, WorkspaceApiError> {
        std::fs::create_dir_all(path).map_err(|error| {
            WorkspaceApiError::new(WorkspaceApiErrorCode::GitCommandFailed, error.to_string())
        })?;
        run_git(path, &["init"])?;
        self.open_existing(workspace_id, path)
    }

    #[must_use]
    pub fn get(&self, workspace_id: &WorkspaceId) -> Option<&WorkspaceApiSnapshot> {
        self.workspaces.get(workspace_id)
    }
}

fn snapshot_from_repo(
    workspace_id: WorkspaceId,
    repo: &GitRepository,
) -> Result<WorkspaceApiSnapshot, WorkspaceApiError> {
    let status = repo.status().map_err(map_git_error)?;
    let diff = repo.diff().map_err(map_git_error)?;
    let checkpoint = repo.prepare_checkpoint().map_err(map_git_error)?;

    Ok(WorkspaceApiSnapshot {
        workspace_id,
        root_path: repo.identity().root_path().to_path_buf(),
        git_dir_path: repo.identity().git_dir_path().to_path_buf(),
        status_entries: status.entries().to_vec(),
        diff_text: diff.as_text().to_string(),
        checkpoint_status: checkpoint.status(),
    })
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<(), WorkspaceApiError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| {
            WorkspaceApiError::new(WorkspaceApiErrorCode::GitCommandFailed, error.to_string())
        })?;

    if output.status.success() {
        Ok(())
    } else {
        Err(WorkspaceApiError::new(
            WorkspaceApiErrorCode::GitCommandFailed,
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ))
    }
}

fn map_git_error(error: WorkspaceGitError) -> WorkspaceApiError {
    let message = error.to_string();
    if message.contains("not a git repository") {
        WorkspaceApiError::new(WorkspaceApiErrorCode::NotGitRepository, message)
    } else {
        WorkspaceApiError::new(WorkspaceApiErrorCode::GitCommandFailed, message)
    }
}
