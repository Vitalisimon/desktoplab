mod commit_push;
mod savepoint;
mod worktree;

use std::fmt;
use std::path::Path;
use std::process::Command;

pub use commit_push::{CommitApproval, CommitOperation, PushApproval, PushOperation};
pub use savepoint::{
    RollbackApproval, RollbackOperation, RollbackPreview, SavePoint, SavePointManager,
};
pub use worktree::{
    ParallelAgentRoute, ParallelAgentRouter, ProductWorktreeManager, SessionIntent,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitProductizationOutcome {
    status: String,
    had_network_side_effect: bool,
    message: String,
}

impl GitProductizationOutcome {
    pub(super) fn new(status: &str, had_network_side_effect: bool, message: &str) -> Self {
        Self {
            status: status.to_string(),
            had_network_side_effect,
            message: message.to_string(),
        }
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub fn had_network_side_effect(&self) -> bool {
        self.had_network_side_effect
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductGitError {
    message: String,
}

impl ProductGitError {
    pub(super) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(super) fn from_display(error: impl fmt::Display) -> Self {
        Self::new(error.to_string())
    }
}

impl fmt::Display for ProductGitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ProductGitError {}

pub(super) fn git(root: &Path, args: &[&str]) -> Result<(), ProductGitError> {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=DesktopLab",
            "-c",
            "user.email=desktoplab@example.invalid",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .map_err(ProductGitError::from_display)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(ProductGitError::new(
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}

pub(super) fn git_stdout(root: &Path, args: &[&str]) -> Result<String, ProductGitError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(ProductGitError::from_display)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(ProductGitError::new(
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}
