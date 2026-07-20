use std::path::Path;

use crate::WorkspaceRoot;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchPatchItem {
    pub path: String,
    pub expected: String,
    pub replacement: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BatchPatchOutcome {
    Applied,
    Conflict(String),
    Blocked(String),
}

pub struct FilesystemBatchPatchExecutor {
    root: Option<WorkspaceRoot>,
}

impl FilesystemBatchPatchExecutor {
    pub fn new(root: &Path) -> Self {
        Self {
            root: WorkspaceRoot::open(root).ok(),
        }
    }

    pub fn apply(&self, items: &[BatchPatchItem]) -> BatchPatchOutcome {
        let Some(root) = &self.root else {
            return BatchPatchOutcome::Blocked("workspace_root_unavailable".to_string());
        };
        let mut planned = Vec::with_capacity(items.len());
        for item in items {
            let Ok(mut file) = root.open_update(&item.path) else {
                return BatchPatchOutcome::Blocked(format!("path_blocked:{}", item.path));
            };
            let Ok(contents) = file.read_text() else {
                return BatchPatchOutcome::Blocked(format!("read_failed:{}", item.path));
            };
            if !contents.contains(&item.expected) {
                return BatchPatchOutcome::Conflict(item.path.clone());
            }
            let updated = contents.replacen(&item.expected, &item.replacement, 1);
            planned.push((file, updated));
        }
        for (mut file, updated) in planned {
            if file.replace_text(&updated).is_err() {
                return BatchPatchOutcome::Blocked("write_failed".to_string());
            }
        }
        BatchPatchOutcome::Applied
    }
}
