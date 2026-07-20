use std::path::{Path, PathBuf};

use super::{GitProductizationOutcome, ProductGitError, git};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionIntent {
    ReadOnly,
    WriteCapable,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProductWorktreeManager {
    _private: (),
}

impl ProductWorktreeManager {
    pub fn create(
        &self,
        root: &Path,
        session_id: &str,
    ) -> Result<ParallelAgentRoute, ProductGitError> {
        let repo_name = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace");
        let session_name = sanitized_session_name(session_id);
        let worktree_path = root
            .parent()
            .unwrap_or(root)
            .join(format!("{repo_name}-desktoplab-{session_name}"));
        git(
            root,
            &[
                "worktree",
                "add",
                "--detach",
                worktree_path.to_str().unwrap_or(""),
                "HEAD",
            ],
        )?;
        Ok(ParallelAgentRoute::isolated(worktree_path))
    }

    pub fn cleanup(
        &self,
        root: &Path,
        session_id: &str,
    ) -> Result<GitProductizationOutcome, ProductGitError> {
        let worktree_path = self.owned_worktree_path(root, session_id)?;
        if !worktree_path.exists() {
            return Err(ProductGitError::new("managed_worktree_not_found"));
        }
        git(
            root,
            &["worktree", "remove", worktree_path.to_str().unwrap_or("")],
        )?;
        Ok(GitProductizationOutcome::new("cleaned", false, ""))
    }

    fn owned_worktree_path(
        &self,
        root: &Path,
        session_id: &str,
    ) -> Result<PathBuf, ProductGitError> {
        let repo_name = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("workspace");
        let session_name = sanitized_session_name(session_id);
        let expected_name = format!("{repo_name}-desktoplab-{session_name}");
        if !expected_name.contains("-desktoplab-") {
            return Err(ProductGitError::new("not_desktoplab_managed_worktree"));
        }
        Ok(root.parent().unwrap_or(root).join(expected_name))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ParallelAgentRouter {
    manager: ProductWorktreeManager,
}

impl ParallelAgentRouter {
    #[must_use]
    pub fn new(manager: ProductWorktreeManager) -> Self {
        Self { manager }
    }

    #[must_use]
    pub fn route(
        &self,
        root: &Path,
        session_id: &str,
        intent: SessionIntent,
    ) -> ParallelAgentRoute {
        match intent {
            SessionIntent::ReadOnly => ParallelAgentRoute::shared(),
            SessionIntent::WriteCapable => self
                .manager
                .create(root, session_id)
                .unwrap_or_else(|_| ParallelAgentRoute::blocked()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParallelAgentRoute {
    worktree_path: Option<PathBuf>,
    reason: String,
    blocked: bool,
}

impl ParallelAgentRoute {
    fn isolated(worktree_path: PathBuf) -> Self {
        Self {
            worktree_path: Some(worktree_path),
            reason: "write_capable_parallel_requires_worktree".to_string(),
            blocked: false,
        }
    }

    fn shared() -> Self {
        Self {
            worktree_path: None,
            reason: "read_only_can_share_workspace".to_string(),
            blocked: false,
        }
    }

    fn blocked() -> Self {
        Self {
            worktree_path: None,
            reason: "worktree_creation_failed".to_string(),
            blocked: true,
        }
    }

    #[must_use]
    pub fn isolation_reason(&self) -> &str {
        &self.reason
    }

    #[must_use]
    pub fn worktree_path(&self) -> Option<&Path> {
        self.worktree_path.as_deref()
    }

    #[must_use]
    pub fn can_share_workspace(&self) -> bool {
        self.worktree_path.is_none() && !self.blocked
    }
}

fn sanitized_session_name(session_id: &str) -> String {
    session_id.replace(['/', '.', ':'], "-")
}
