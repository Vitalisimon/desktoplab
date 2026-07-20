mod snapshot;

use std::path::Path;

use crate::GitRepository;

use super::{GitProductizationOutcome, ProductGitError, git, git_stdout};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SavePoint {
    session_id: String,
    ref_name: String,
}

impl SavePoint {
    #[must_use]
    pub fn from_ref(session_id: impl Into<String>, ref_name: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            ref_name: ref_name.into(),
        }
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn ref_name(&self) -> &str {
        &self.ref_name
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SavePointManager {
    _private: (),
}

impl SavePointManager {
    pub fn create(&self, root: &Path, session_id: &str) -> Result<SavePoint, ProductGitError> {
        let repo = GitRepository::open(root).map_err(ProductGitError::from_display)?;
        let dirty = repo
            .status()
            .map_err(ProductGitError::from_display)?
            .is_dirty();
        let head = git_stdout(root, &["rev-parse", "--verify", "HEAD"])
            .ok()
            .map(|value| value.trim().to_string());
        let target = if dirty || head.is_none() {
            snapshot::capture_worktree(
                root,
                repo.identity().git_dir_path(),
                head.as_deref(),
                session_id,
            )?
        } else {
            head.expect("clean repository with a verified HEAD")
        };
        let ref_name = format!("desktoplab/savepoints/{session_id}");
        git(root, &["update-ref", &format!("refs/{ref_name}"), &target])?;
        Ok(SavePoint {
            session_id: session_id.to_string(),
            ref_name,
        })
    }

    pub fn list(&self, root: &Path) -> Result<Vec<SavePoint>, ProductGitError> {
        let refs = git_stdout(
            root,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/desktoplab/savepoints",
            ],
        )?;
        let mut savepoints = refs
            .lines()
            .filter_map(|reference| {
                let ref_name = reference.strip_prefix("refs/")?.to_string();
                let session_id = ref_name.strip_prefix("desktoplab/savepoints/")?.to_string();
                Some(SavePoint {
                    session_id,
                    ref_name,
                })
            })
            .collect::<Vec<_>>();
        savepoints.sort_by(|left, right| left.ref_name.cmp(&right.ref_name));
        Ok(savepoints)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RollbackApproval {
    Approved,
    Denied,
}

pub struct RollbackOperation {
    approval: RollbackApproval,
}

impl RollbackOperation {
    #[must_use]
    pub fn new(approval: RollbackApproval) -> Self {
        Self { approval }
    }

    pub fn rollback(
        &self,
        root: &Path,
        savepoint: &SavePoint,
    ) -> Result<GitProductizationOutcome, ProductGitError> {
        if self.approval == RollbackApproval::Denied {
            return Ok(GitProductizationOutcome::new("denied", false, ""));
        }
        let target = rollback_target(&savepoint.ref_name);
        git(root, &["reset", "--hard", &target])?;
        Ok(GitProductizationOutcome::new("restored", false, ""))
    }

    pub fn preview(
        &self,
        root: &Path,
        _savepoint: &SavePoint,
    ) -> Result<RollbackPreview, ProductGitError> {
        let repo = GitRepository::open(root).map_err(ProductGitError::from_display)?;
        let status = repo.status().map_err(ProductGitError::from_display)?;
        let mut changed_files = Vec::new();
        let mut protected_untracked_files = Vec::new();
        for file in status.files() {
            if file.is_untracked() {
                protected_untracked_files.push(file.path().to_string());
            } else {
                changed_files.push(file.path().to_string());
            }
        }
        Ok(RollbackPreview {
            changed_files,
            protected_untracked_files,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RollbackPreview {
    changed_files: Vec<String>,
    protected_untracked_files: Vec<String>,
}

impl RollbackPreview {
    #[must_use]
    pub fn changed_files(&self) -> &[String] {
        &self.changed_files
    }

    #[must_use]
    pub fn protected_untracked_files(&self) -> &[String] {
        &self.protected_untracked_files
    }
}

fn rollback_target(ref_name: &str) -> String {
    if ref_name == "HEAD" || ref_name.starts_with("refs/") {
        ref_name.to_string()
    } else {
        format!("refs/{ref_name}")
    }
}
