use std::fs;
use std::io;
use std::path::Path;

use crate::search_pattern::WorkspaceSearchPattern;
use crate::{WorkspaceIndex, WorkspaceIndexLimits};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceFileSafety {
    Readable,
    Protected,
    Binary,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchLimits {
    max_files: usize,
    max_matches: usize,
    max_bytes_per_file: usize,
}

impl WorkspaceSearchLimits {
    #[must_use]
    pub fn new(max_files: usize, max_matches: usize, max_bytes_per_file: usize) -> Self {
        Self {
            max_files,
            max_matches,
            max_bytes_per_file,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceFileEntry {
    path: String,
    size_bytes: u64,
    language: Option<&'static str>,
    safety: WorkspaceFileSafety,
}

impl WorkspaceFileEntry {
    #[must_use]
    pub(crate) fn new(
        path: impl Into<String>,
        size_bytes: u64,
        language: Option<&'static str>,
        safety: WorkspaceFileSafety,
    ) -> Self {
        Self {
            path: path.into(),
            size_bytes,
            language,
            safety,
        }
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub fn language(&self) -> Option<&'static str> {
        self.language
    }

    #[must_use]
    pub fn safety(&self) -> WorkspaceFileSafety {
        self.safety
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchHit {
    path: String,
    preview: String,
    line_number: Option<usize>,
}

impl WorkspaceSearchHit {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn preview(&self) -> &str {
        &self.preview
    }

    #[must_use]
    pub fn line_number(&self) -> Option<usize> {
        self.line_number
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearchReport {
    matches: Vec<WorkspaceSearchHit>,
    truncated: bool,
}

impl WorkspaceSearchReport {
    #[must_use]
    pub fn matches(&self) -> &[WorkspaceSearchHit] {
        &self.matches
    }

    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSearch {
    limits: WorkspaceSearchLimits,
}

impl WorkspaceSearch {
    #[must_use]
    pub fn new(limits: WorkspaceSearchLimits) -> Self {
        Self { limits }
    }

    pub fn list_files(&self, root: &Path) -> io::Result<Vec<WorkspaceFileEntry>> {
        let entries = WorkspaceIndex::new(WorkspaceIndexLimits::new(
            self.limits.max_files,
            self.limits.max_bytes_per_file,
        ))
        .build(root)?
        .entries()
        .iter()
        .map(|entry| {
            WorkspaceFileEntry::new(
                entry.path(),
                entry.size_bytes(),
                entry.language(),
                entry.safety(),
            )
        })
        .collect();
        Ok(entries)
    }

    pub fn search(&self, root: &Path, query: &str) -> io::Result<WorkspaceSearchReport> {
        self.search_with_options(root, query, false, false)
    }

    pub fn search_with_options(
        &self,
        root: &Path,
        query: &str,
        regex: bool,
        case_sensitive: bool,
    ) -> io::Result<WorkspaceSearchReport> {
        let pattern = WorkspaceSearchPattern::new(query, regex, case_sensitive)?;
        let mut matches = Vec::new();
        let mut truncated = false;
        for entry in self.list_files(root)? {
            if entry.safety != WorkspaceFileSafety::Readable {
                continue;
            }
            let Some(text) = self.read_limited_text(root, entry.path())? else {
                continue;
            };
            let mut hits = preview_lines(&text, &pattern);
            if hits.is_empty() && pattern.matches(entry.path()) {
                hits.push((String::new(), None));
            }
            for (preview, line_number) in hits {
                matches.push(WorkspaceSearchHit {
                    path: entry.path.clone(),
                    preview,
                    line_number,
                });
                if matches.len() >= self.limits.max_matches {
                    truncated = true;
                    break;
                }
            }
            if truncated {
                break;
            }
        }
        Ok(WorkspaceSearchReport { matches, truncated })
    }

    pub fn ranked_context_paths(
        &self,
        root: &Path,
        prompt: &str,
        recent_mentions: &[&str],
        max_paths: usize,
    ) -> io::Result<Vec<String>> {
        let terms = prompt_terms(prompt);
        let mut scored = Vec::new();
        for entry in self.list_files(root)? {
            if entry.safety != WorkspaceFileSafety::Readable {
                continue;
            }
            let mut score = entrypoint_score(entry.path());
            if recent_mentions.iter().any(|path| *path == entry.path()) {
                score += 100;
            }
            let path_lower = entry.path().to_ascii_lowercase();
            for term in &terms {
                if path_lower.contains(term) {
                    score += 50;
                }
            }
            if score == 0 {
                if let Some(text) = self.read_limited_text(root, entry.path())? {
                    let lower = text.to_ascii_lowercase();
                    score += terms.iter().filter(|term| lower.contains(*term)).count() * 10;
                }
            }
            if score > 0 {
                scored.push((score, entry.path));
            }
        }
        scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        Ok(scored
            .into_iter()
            .map(|(_, path)| path)
            .take(max_paths)
            .collect())
    }

    fn read_limited_text(&self, root: &Path, relative: &str) -> io::Result<Option<String>> {
        let bytes = fs::read(root.join(relative))?;
        let bytes = bytes
            .into_iter()
            .take(self.limits.max_bytes_per_file)
            .collect::<Vec<_>>();
        Ok(String::from_utf8(bytes).ok())
    }
}

pub(crate) fn language_for_path(path: &str) -> Option<&'static str> {
    if path.ends_with(".rs") || path == "Cargo.toml" {
        Some("rust")
    } else if path.ends_with(".ts") || path.ends_with(".tsx") || path == "package.json" {
        Some("typescript")
    } else if path.ends_with(".py") || path == "pyproject.toml" {
        Some("python")
    } else if path.ends_with(".md") {
        Some("markdown")
    } else {
        None
    }
}

fn entrypoint_score(path: &str) -> usize {
    match path {
        "README.md" | "Cargo.toml" | "package.json" | "pyproject.toml" => 25,
        path if path.starts_with("src/") => 15,
        _ => 0,
    }
}

fn prompt_terms(prompt: &str) -> Vec<String> {
    prompt
        .split(|ch: char| !ch.is_alphanumeric())
        .map(str::to_ascii_lowercase)
        .filter(|term| term.len() > 3 && !matches!(term.as_str(), "questa" | "repo"))
        .collect()
}

fn preview_lines(text: &str, pattern: &WorkspaceSearchPattern) -> Vec<(String, Option<usize>)> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| pattern.matches(line))
        .map(|(index, line)| (line.chars().take(240).collect(), Some(index + 1)))
        .collect()
}
