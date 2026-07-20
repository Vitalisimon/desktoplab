use crate::{RepoCodeIndexSnapshot, RepoCodeIndexer};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepoIndexFreshnessState {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoIndexFreshnessReport {
    state: RepoIndexFreshnessState,
    reasons: Vec<String>,
}

impl RepoIndexFreshnessReport {
    #[must_use]
    pub fn state(&self) -> RepoIndexFreshnessState {
        self.state
    }

    #[must_use]
    pub fn is_fresh(&self) -> bool {
        self.state == RepoIndexFreshnessState::Fresh
    }

    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
}

pub struct RepoIndexFreshnessGuard;

impl RepoIndexFreshnessGuard {
    #[must_use]
    pub fn validate(
        index: &RepoCodeIndexSnapshot,
        current_root: &Path,
    ) -> RepoIndexFreshnessReport {
        let Ok(canonical_root) = current_root.canonicalize() else {
            return unknown("workspace_root_unavailable");
        };
        if canonical_root != index.root() {
            return stale(["workspace_relinked"]);
        }
        let Ok(current) = RepoCodeIndexer::new(index.limits().clone()).build(&canonical_root)
        else {
            return unknown("index_refresh_failed");
        };
        let mut reasons = Vec::new();
        if current.git().branch() != index.git().branch() {
            reasons.push("git_branch_changed".into());
        }
        if current.git().head() != index.git().head() {
            reasons.push("git_head_changed".into());
        }
        let before = document_hashes(index);
        let after = document_hashes(&current);
        for path in before.keys() {
            if !after.contains_key(path) {
                reasons.push(format!("indexed_file_deleted:{path}"));
            }
        }
        for (path, hash) in &after {
            match before.get(path) {
                None => reasons.push(format!("indexable_file_added:{path}")),
                Some(previous) if previous != hash => {
                    reasons.push(format!("indexed_file_changed:{path}"));
                }
                Some(_) => {}
            }
        }
        if reasons.is_empty() {
            RepoIndexFreshnessReport {
                state: RepoIndexFreshnessState::Fresh,
                reasons,
            }
        } else {
            RepoIndexFreshnessReport {
                state: RepoIndexFreshnessState::Stale,
                reasons,
            }
        }
    }
}

fn document_hashes(index: &RepoCodeIndexSnapshot) -> BTreeMap<&str, &str> {
    index
        .documents()
        .iter()
        .map(|document| (document.path(), document.content_hash()))
        .collect()
}

fn stale<const N: usize>(reasons: [&str; N]) -> RepoIndexFreshnessReport {
    RepoIndexFreshnessReport {
        state: RepoIndexFreshnessState::Stale,
        reasons: reasons.into_iter().map(str::to_string).collect(),
    }
}

fn unknown(reason: &str) -> RepoIndexFreshnessReport {
    RepoIndexFreshnessReport {
        state: RepoIndexFreshnessState::Unknown,
        reasons: vec![reason.into()],
    }
}
