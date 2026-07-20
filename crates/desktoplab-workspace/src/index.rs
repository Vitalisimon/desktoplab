use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::inspection::is_protected_path;
use crate::search::{WorkspaceFileSafety, language_for_path};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIndexLimits {
    max_files: usize,
    max_bytes_per_file: usize,
    max_skipped_reasons: usize,
}

impl WorkspaceIndexLimits {
    #[must_use]
    pub fn new(max_files: usize, max_bytes_per_file: usize) -> Self {
        Self {
            max_files,
            max_bytes_per_file,
            max_skipped_reasons: 16,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIndexEntry {
    path: String,
    size_bytes: u64,
    modified_unix_secs: Option<u64>,
    language: Option<&'static str>,
    safety: WorkspaceFileSafety,
    text_preview_eligible: bool,
}

impl WorkspaceIndexEntry {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub fn modified_unix_secs(&self) -> Option<u64> {
        self.modified_unix_secs
    }

    #[must_use]
    pub fn language(&self) -> Option<&'static str> {
        self.language
    }

    #[must_use]
    pub fn safety(&self) -> WorkspaceFileSafety {
        self.safety
    }

    #[must_use]
    pub fn text_preview_eligible(&self) -> bool {
        self.text_preview_eligible
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIndexSnapshot {
    entries: Vec<WorkspaceIndexEntry>,
    truncated: bool,
    skipped: Vec<String>,
}

impl WorkspaceIndexSnapshot {
    #[must_use]
    pub fn entries(&self) -> &[WorkspaceIndexEntry] {
        &self.entries
    }

    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    #[must_use]
    pub fn skipped(&self) -> &[String] {
        &self.skipped
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIndex {
    limits: WorkspaceIndexLimits,
}

impl WorkspaceIndex {
    #[must_use]
    pub fn new(limits: WorkspaceIndexLimits) -> Self {
        Self { limits }
    }

    pub fn build(&self, root: &Path) -> io::Result<WorkspaceIndexSnapshot> {
        let ignore = IgnoreRules::load(root);
        let mut entries = Vec::new();
        let mut skipped = Vec::new();
        let mut truncated = false;
        collect_index_entries(
            root,
            root,
            &ignore,
            &self.limits,
            &mut entries,
            &mut skipped,
            &mut truncated,
        )?;
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(WorkspaceIndexSnapshot {
            entries,
            truncated,
            skipped,
        })
    }
}

fn collect_index_entries(
    root: &Path,
    current: &Path,
    ignore: &IgnoreRules,
    limits: &WorkspaceIndexLimits,
    entries: &mut Vec<WorkspaceIndexEntry>,
    skipped: &mut Vec<String>,
    truncated: &mut bool,
) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let relative_text = normalize(relative);
        if should_skip_index_path(&relative_text) || ignore.matches(&relative_text) {
            record_skipped(skipped, limits, format!("{relative_text}:ignored"));
            continue;
        }
        if path.is_dir() {
            collect_index_entries(root, &path, ignore, limits, entries, skipped, truncated)?;
            continue;
        }
        if entries.len() >= limits.max_files {
            *truncated = true;
            continue;
        }
        if let Some(index_entry) = index_entry(root, relative, limits)? {
            entries.push(index_entry);
        }
    }
    Ok(())
}

fn index_entry(
    root: &Path,
    relative: &Path,
    limits: &WorkspaceIndexLimits,
) -> io::Result<Option<WorkspaceIndexEntry>> {
    let path = normalize(relative);
    let full_path = root.join(relative);
    let metadata = fs::metadata(&full_path)?;
    if is_binary(&full_path, limits.max_bytes_per_file)? {
        return Ok(None);
    }
    let safety = if is_protected_path(&path) {
        WorkspaceFileSafety::Protected
    } else {
        WorkspaceFileSafety::Readable
    };
    let text_preview_eligible = safety == WorkspaceFileSafety::Readable
        && metadata.len() <= limits.max_bytes_per_file as u64;
    Ok(Some(WorkspaceIndexEntry {
        language: language_for_path(&path),
        modified_unix_secs: metadata
            .modified()
            .ok()
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
        path,
        size_bytes: metadata.len(),
        safety,
        text_preview_eligible,
    }))
}

fn record_skipped(skipped: &mut Vec<String>, limits: &WorkspaceIndexLimits, reason: String) {
    if skipped.len() < limits.max_skipped_reasons {
        skipped.push(reason);
    }
}

fn should_skip_index_path(path: &str) -> bool {
    let generated = [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        "coverage",
        ".turbo",
        ".cache",
    ];
    generated
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
        || path == ".DS_Store"
        || path.ends_with("/.DS_Store")
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_binary(path: &Path, max_bytes: usize) -> io::Result<bool> {
    let bytes = fs::read(path)?;
    let sample = bytes.into_iter().take(max_bytes).collect::<Vec<_>>();
    Ok(sample.contains(&0) || std::str::from_utf8(&sample).is_err())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IgnoreRules {
    patterns: Vec<String>,
}

impl IgnoreRules {
    fn load(root: &Path) -> Self {
        let patterns = fs::read_to_string(root.join(".gitignore"))
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.trim_start_matches('/').to_string())
            .collect();
        Self { patterns }
    }

    fn matches(&self, path: &str) -> bool {
        self.patterns
            .iter()
            .any(|pattern| pattern_matches(pattern, path))
    }
}

fn pattern_matches(pattern: &str, path: &str) -> bool {
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        return path.starts_with(prefix) && path.ends_with(suffix);
    }
    if let Some(dir) = pattern.strip_suffix('/') {
        return path == dir || path.starts_with(&format!("{dir}/"));
    }
    if pattern.contains('/') {
        return path == pattern || path.starts_with(&format!("{pattern}/"));
    }
    PathBuf::from(path)
        .components()
        .any(|component| component.as_os_str() == pattern)
}
