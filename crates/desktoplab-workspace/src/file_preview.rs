use crate::inspection::is_protected_path;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilePreviewState {
    Text,
    Binary,
    Denied,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FilePreviewLimits {
    max_bytes: usize,
    max_lines: usize,
}

impl FilePreviewLimits {
    #[must_use]
    pub fn new(max_bytes: usize, max_lines: usize) -> Self {
        Self {
            max_bytes,
            max_lines,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilePreview {
    path: String,
    state: FilePreviewState,
    text: Option<String>,
    denied_reason: Option<&'static str>,
    original_bytes: usize,
    returned_bytes: usize,
    original_lines: usize,
    returned_lines: usize,
    truncated: bool,
}

impl FilePreview {
    pub fn read(root: &Path, relative_path: &str, limits: FilePreviewLimits) -> io::Result<Self> {
        let relative_path = normalize_path(relative_path);
        if is_protected_path(&relative_path) || escapes_root(&relative_path) {
            return Ok(Self::denied(relative_path, "local_only_path"));
        }

        let Some(path) = contained_existing_path(root, &relative_path)? else {
            return Ok(Self::denied(relative_path, "path_escape"));
        };

        let bytes = fs::read(path)?;
        if bytes.contains(&0) {
            return Ok(Self::binary(relative_path, bytes.len()));
        }

        let Ok(text) = String::from_utf8(bytes) else {
            return Ok(Self::binary(relative_path, 0));
        };
        Ok(Self::text_preview(relative_path, text, limits))
    }

    #[must_use]
    pub fn state(&self) -> FilePreviewState {
        self.state
    }

    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    #[must_use]
    pub fn denied_reason(&self) -> Option<&'static str> {
        self.denied_reason
    }

    #[must_use]
    pub fn original_bytes(&self) -> usize {
        self.original_bytes
    }

    #[must_use]
    pub fn returned_lines(&self) -> usize {
        self.returned_lines
    }

    #[must_use]
    pub fn original_lines(&self) -> usize {
        self.original_lines
    }

    #[must_use]
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    fn denied(path: String, reason: &'static str) -> Self {
        Self {
            path,
            state: FilePreviewState::Denied,
            text: None,
            denied_reason: Some(reason),
            original_bytes: 0,
            returned_bytes: 0,
            original_lines: 0,
            returned_lines: 0,
            truncated: false,
        }
    }

    fn binary(path: String, original_bytes: usize) -> Self {
        Self {
            path,
            state: FilePreviewState::Binary,
            text: None,
            denied_reason: None,
            original_bytes,
            returned_bytes: 0,
            original_lines: 0,
            returned_lines: 0,
            truncated: false,
        }
    }

    fn text_preview(path: String, text: String, limits: FilePreviewLimits) -> Self {
        let original_bytes = text.len();
        let original_lines = text.lines().count();
        let redacted = redact_secrets(&text);
        let limited_by_lines = redacted
            .lines()
            .take(limits.max_lines)
            .collect::<Vec<_>>()
            .join("\n");
        let limited = truncate_utf8(&limited_by_lines, limits.max_bytes);
        let returned_bytes = limited.len();
        let returned_lines = limited.lines().count();
        Self {
            path,
            state: FilePreviewState::Text,
            text: Some(limited),
            denied_reason: None,
            original_bytes,
            returned_bytes,
            original_lines,
            returned_lines,
            truncated: original_bytes > returned_bytes || original_lines > returned_lines,
        }
    }
}

fn redact_secrets(text: &str) -> String {
    text.lines()
        .map(|line| {
            let upper = line.to_ascii_uppercase();
            if upper.contains("API_KEY=") || upper.contains("TOKEN=") || upper.contains("SECRET=") {
                line.split_once('=')
                    .map(|(key, _)| format!("{key}=[REDACTED_SECRET]"))
                    .unwrap_or_else(|| "[REDACTED_SECRET]".to_string())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn escapes_root(path: &str) -> bool {
    Path::new(path)
        .components()
        .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
}

fn contained_existing_path(root: &Path, relative_path: &str) -> io::Result<Option<PathBuf>> {
    let root = root.canonicalize()?;
    let candidate = root.join(relative_path).canonicalize()?;
    Ok(candidate.starts_with(&root).then_some(candidate))
}
