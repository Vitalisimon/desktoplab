use std::path::Path;

use super::{GitProductizationOutcome, ProductGitError, git};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommitApproval {
    Approved,
    Denied,
}

pub struct CommitOperation {
    approval: CommitApproval,
}

impl CommitOperation {
    #[must_use]
    pub fn new(approval: CommitApproval) -> Self {
        Self { approval }
    }

    pub fn commit(
        &self,
        root: &Path,
        session_id: &str,
        message: &str,
        files: &[String],
    ) -> Result<GitProductizationOutcome, ProductGitError> {
        if self.approval == CommitApproval::Denied {
            return Ok(GitProductizationOutcome::new("denied", false, ""));
        }
        let message = format!("{message}\n\nDesktopLab-Session: {session_id}");
        if files.is_empty() {
            return Err(ProductGitError::new("no_reviewed_files_to_commit"));
        }
        for file in files {
            git(root, &["add", "--", file])?;
        }
        let mut arguments = vec!["commit", "--only", "-m", message.as_str(), "--"];
        arguments.extend(files.iter().map(String::as_str));
        git(root, &arguments)?;
        Ok(GitProductizationOutcome::new("committed", false, &message))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PushApproval {
    Approved,
    Denied,
}

pub struct PushOperation {
    approval: PushApproval,
}

impl PushOperation {
    #[must_use]
    pub fn new(approval: PushApproval) -> Self {
        Self { approval }
    }

    pub fn push(
        &self,
        root: &Path,
        remote: &str,
        branch: &str,
    ) -> Result<GitProductizationOutcome, ProductGitError> {
        if self.approval == PushApproval::Denied {
            return Ok(GitProductizationOutcome::new("denied", false, ""));
        }
        git(root, &["push", remote, branch])?;
        Ok(GitProductizationOutcome::new("pushed", true, ""))
    }
}
