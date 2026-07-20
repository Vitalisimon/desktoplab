use crate::inspection::{is_protected_path, should_skip_workspace_path};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTreeEntryKind {
    Directory,
    File,
    HiddenFile,
    Symlink,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileTreeProtection {
    Readable,
    Protected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileTreeEntry {
    path: String,
    kind: FileTreeEntryKind,
    protection: FileTreeProtection,
}

impl FileTreeEntry {
    #[must_use]
    pub fn directory(path: impl Into<String>) -> Self {
        Self::new(path, FileTreeEntryKind::Directory)
    }

    #[must_use]
    pub fn file(path: impl Into<String>) -> Self {
        Self::new(path, FileTreeEntryKind::File)
    }

    #[must_use]
    pub fn hidden_file(path: impl Into<String>) -> Self {
        Self::new(path, FileTreeEntryKind::HiddenFile)
    }

    #[must_use]
    pub fn symlink(path: impl Into<String>) -> Self {
        Self::new(path, FileTreeEntryKind::Symlink)
    }

    fn new(path: impl Into<String>, kind: FileTreeEntryKind) -> Self {
        let path = normalize_path(path.into());
        let protection = if is_protected_path(&path) {
            FileTreeProtection::Protected
        } else {
            FileTreeProtection::Readable
        };
        Self {
            path,
            kind,
            protection,
        }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn kind(&self) -> FileTreeEntryKind {
        self.kind
    }

    #[must_use]
    pub fn protection(&self) -> FileTreeProtection {
        self.protection
    }

    #[must_use]
    pub fn preview_text(&self) -> Option<&str> {
        None
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorkspaceFileTreeLimits {
    max_entries: usize,
    max_depth: usize,
}

impl WorkspaceFileTreeLimits {
    #[must_use]
    pub fn new(max_entries: usize, max_depth: usize) -> Self {
        Self {
            max_entries,
            max_depth,
        }
    }

    #[must_use]
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceFileTree {
    workspace_id: String,
    entries: Vec<FileTreeEntry>,
    degraded_reasons: Vec<&'static str>,
    limits: WorkspaceFileTreeLimits,
}

impl WorkspaceFileTree {
    #[must_use]
    pub fn new(
        workspace_id: impl Into<String>,
        mut entries: Vec<FileTreeEntry>,
        limits: WorkspaceFileTreeLimits,
    ) -> Self {
        let mut degraded_reasons = Vec::new();
        if entries.len() > limits.max_entries {
            entries.truncate(limits.max_entries);
            degraded_reasons.push("workspace_file_tree_entry_limit_exceeded");
        }
        Self {
            workspace_id: workspace_id.into(),
            entries,
            degraded_reasons,
            limits,
        }
    }

    pub fn scan(
        workspace_id: impl Into<String>,
        root: &Path,
        limits: WorkspaceFileTreeLimits,
    ) -> io::Result<Self> {
        let mut entries = Vec::new();
        let mut degraded_reasons = Vec::new();
        collect_entries(
            root,
            root,
            0,
            limits.max_depth,
            &mut entries,
            &mut degraded_reasons,
        )?;
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        if entries.len() > limits.max_entries {
            entries.truncate(limits.max_entries);
            push_reason(
                &mut degraded_reasons,
                "workspace_file_tree_entry_limit_exceeded",
            );
        }
        Ok(Self {
            workspace_id: workspace_id.into(),
            entries,
            degraded_reasons,
            limits,
        })
    }

    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    #[must_use]
    pub fn entries(&self) -> &[FileTreeEntry] {
        &self.entries
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        !self.degraded_reasons.is_empty()
    }

    #[must_use]
    pub fn degraded_reasons(&self) -> &[&'static str] {
        &self.degraded_reasons
    }

    #[must_use]
    pub fn limits(&self) -> WorkspaceFileTreeLimits {
        self.limits
    }
}

fn normalize_path(path: String) -> String {
    path.replace('\\', "/")
}

fn collect_entries(
    root: &Path,
    current: &Path,
    depth: usize,
    max_depth: usize,
    entries: &mut Vec<FileTreeEntry>,
    degraded_reasons: &mut Vec<&'static str>,
) -> io::Result<()> {
    if depth >= max_depth {
        push_reason(degraded_reasons, "workspace_file_tree_depth_limit_exceeded");
        return Ok(());
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let relative_text = normalize_path(relative.to_string_lossy().to_string());
        if should_skip_workspace_path(&relative_text) {
            continue;
        }
        if metadata.file_type().is_symlink() {
            entries.push(FileTreeEntry::symlink(relative_text));
        } else if metadata.is_dir() {
            entries.push(FileTreeEntry::directory(relative_text));
            collect_entries(root, &path, depth + 1, max_depth, entries, degraded_reasons)?;
        } else if is_hidden(&relative_text) {
            entries.push(FileTreeEntry::hidden_file(relative_text));
        } else {
            entries.push(FileTreeEntry::file(relative_text));
        }
    }
    Ok(())
}

fn push_reason(reasons: &mut Vec<&'static str>, reason: &'static str) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn is_hidden(path: &str) -> bool {
    path.rsplit('/')
        .next()
        .is_some_and(|name| name.starts_with('.'))
}
