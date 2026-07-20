use std::path::{Path, PathBuf};

use desktoplab_policy::PolicyEngine;
use desktoplab_redaction::redact_sensitive_with_status;
use desktoplab_workspace::{
    GitDiff, GitRepository, GitStatus, GitStatusFile, IsolationDecision, ParallelExecutionKind,
    SavePointManager, WorktreePolicy,
};

use crate::ToolGateway;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParallelGitExecution {
    ReadOnly,
    WriteCapable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GitToolOutcome {
    Status(GitStatus),
    Diff(GitDiff),
    RollbackPreview {
        changed_files: Vec<String>,
        protected_untracked_files: Vec<String>,
    },
    CheckpointReady(String),
    ApprovalRequired,
    Denied,
    Blocked(&'static str),
}

pub struct GitToolExecutor {
    root: PathBuf,
    gateway: ToolGateway,
    worktree_policy: WorktreePolicy,
}

impl GitToolExecutor {
    #[must_use]
    pub fn new(root: &Path, policy: PolicyEngine) -> Self {
        Self {
            root: root.to_path_buf(),
            gateway: ToolGateway::new(policy),
            worktree_policy: WorktreePolicy::strict(),
        }
    }

    pub fn status(&mut self) -> GitToolOutcome {
        let Ok(repo) = GitRepository::open(&self.root) else {
            return GitToolOutcome::Blocked("git_open_failed");
        };
        repo.status()
            .map(GitToolOutcome::Status)
            .unwrap_or(GitToolOutcome::Blocked("git_status_failed"))
    }

    pub fn diff(&mut self) -> GitToolOutcome {
        let Ok(repo) = GitRepository::open(&self.root) else {
            return GitToolOutcome::Blocked("git_open_failed");
        };
        repo.diff()
            .map(GitToolOutcome::Diff)
            .unwrap_or(GitToolOutcome::Blocked("git_diff_failed"))
    }

    pub fn status_observation(&mut self) -> Result<String, &'static str> {
        match self.status() {
            GitToolOutcome::Status(status) if status.entries().is_empty() => {
                Ok("Git status: clean worktree".to_string())
            }
            GitToolOutcome::Status(status) => Ok(redacted_observation(
                "Git status",
                &semantic_status_entries(status.files()),
                "git.status",
            )),
            GitToolOutcome::Blocked(reason) => Err(reason),
            _ => Err("git_status_unavailable"),
        }
    }

    pub fn diff_observation(&mut self) -> Result<String, &'static str> {
        match self.diff() {
            GitToolOutcome::Diff(diff) if diff.as_text().trim().is_empty() => {
                Ok("Git diff: no changes".to_string())
            }
            GitToolOutcome::Diff(diff) => {
                Ok(redacted_observation("Git diff", diff.as_text(), "git.diff"))
            }
            GitToolOutcome::Blocked(reason) => Err(reason),
            _ => Err("git_diff_unavailable"),
        }
    }

    pub fn diff_path_observation(&mut self, path: &str) -> Result<String, &'static str> {
        let repo = GitRepository::open(&self.root).map_err(|_| "git_open_failed")?;
        let diff = repo.diff_path(path).map_err(|_| "git_diff_failed")?;
        if diff.as_text().trim().is_empty() {
            return Ok(format!("Git diff for {path}: no changes"));
        }
        Ok(redacted_observation(
            &format!("Git diff for {path}"),
            diff.as_text(),
            "git.diff",
        ))
    }

    pub fn prepare_checkpoint_ref(&mut self, reference: impl Into<String>) -> GitToolOutcome {
        match SavePointManager::default().create(&self.root, &reference.into()) {
            Ok(savepoint) => GitToolOutcome::CheckpointReady(savepoint.ref_name().to_string()),
            Err(_) => GitToolOutcome::Blocked("checkpoint_failed"),
        }
    }

    pub fn rollback_preview(&mut self) -> GitToolOutcome {
        let Ok(repo) = GitRepository::open(&self.root) else {
            return GitToolOutcome::Blocked("git_open_failed");
        };
        let Ok(status) = repo.status() else {
            return GitToolOutcome::Blocked("git_status_failed");
        };
        let mut changed_files = Vec::new();
        let mut protected_untracked_files = Vec::new();
        for file in status.files() {
            if file.is_untracked() {
                protected_untracked_files.push(file.path().to_string());
            } else {
                changed_files.push(file.path().to_string());
            }
        }
        GitToolOutcome::RollbackPreview {
            changed_files,
            protected_untracked_files,
        }
    }

    #[must_use]
    pub fn parallel_execution_policy(&self, execution: ParallelGitExecution) -> IsolationDecision {
        let kind = match execution {
            ParallelGitExecution::ReadOnly => ParallelExecutionKind::ReadOnlyParallel,
            ParallelGitExecution::WriteCapable => ParallelExecutionKind::WriteCapableParallel,
        };
        self.worktree_policy.evaluate(kind)
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

fn redacted_observation(label: &str, body: &str, source: &str) -> String {
    let redacted = redact_sensitive_with_status(body);
    format!(
        "{label}: redacted={} redaction_source={source}\n{}",
        redacted.redacted(),
        redacted.value()
    )
}

fn semantic_status_entries(entries: &[GitStatusFile]) -> String {
    entries
        .iter()
        .map(|entry| format!("- {}: {}", semantic_status_kind(entry.code()), entry.path()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn semantic_status_kind(code: &str) -> &'static str {
    if code == "??" {
        "untracked"
    } else if code.contains('U') {
        "conflicted"
    } else if code.contains('R') {
        "renamed"
    } else if code.contains('D') {
        "deleted"
    } else if code.contains('A') {
        "added"
    } else if code.contains('M') || code.contains('T') {
        "modified"
    } else {
        "changed"
    }
}
