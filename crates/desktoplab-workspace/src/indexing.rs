use crate::syntax_index::{CodeReference, CodeSymbol, parse_syntax};
use desktoplab_redaction::is_secret_bearing_path;
use ignore::WalkBuilder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::UNIX_EPOCH;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoIndexLimits {
    max_files: usize,
    max_bytes_per_file: usize,
}

impl RepoIndexLimits {
    #[must_use]
    pub fn new(max_files: usize, max_bytes_per_file: usize) -> Self {
        Self {
            max_files,
            max_bytes_per_file,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoGitMetadata {
    branch: Option<String>,
    head: Option<String>,
    dirty_paths: Vec<String>,
}

impl RepoGitMetadata {
    #[must_use]
    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    #[must_use]
    pub fn head(&self) -> Option<&str> {
        self.head.as_deref()
    }

    #[must_use]
    pub fn dirty_paths(&self) -> &[String] {
        &self.dirty_paths
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexedCodeDocument {
    path: String,
    content: String,
    content_hash: String,
    modified_unix_secs: Option<u64>,
    symbols: Vec<CodeSymbol>,
    references: Vec<CodeReference>,
    dependency_hints: Vec<String>,
}

impl IndexedCodeDocument {
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    #[must_use]
    pub fn modified_unix_secs(&self) -> Option<u64> {
        self.modified_unix_secs
    }

    #[must_use]
    pub fn symbols(&self) -> &[CodeSymbol] {
        &self.symbols
    }

    #[must_use]
    pub fn references(&self) -> &[CodeReference] {
        &self.references
    }

    #[must_use]
    pub fn dependency_hints(&self) -> &[String] {
        &self.dependency_hints
    }

    pub(crate) fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoCodeIndexSnapshot {
    root: PathBuf,
    generation_id: String,
    documents: Vec<IndexedCodeDocument>,
    git: RepoGitMetadata,
    truncated: bool,
    limits: RepoIndexLimits,
}

impl RepoCodeIndexSnapshot {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn generation_id(&self) -> &str {
        &self.generation_id
    }

    #[must_use]
    pub fn documents(&self) -> &[IndexedCodeDocument] {
        &self.documents
    }

    #[must_use]
    pub fn git(&self) -> &RepoGitMetadata {
        &self.git
    }

    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    pub(crate) fn limits(&self) -> &RepoIndexLimits {
        &self.limits
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepoCodeIndexer {
    limits: RepoIndexLimits,
}

impl RepoCodeIndexer {
    #[must_use]
    pub fn new(limits: RepoIndexLimits) -> Self {
        Self { limits }
    }

    pub fn build(&self, root: &Path) -> io::Result<RepoCodeIndexSnapshot> {
        let canonical_root = root.canonicalize()?;
        let mut documents = Vec::new();
        let mut truncated = false;
        let walker = WalkBuilder::new(&canonical_root)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .parents(true)
            .build();
        for entry in walker.filter_map(Result::ok) {
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                continue;
            }
            let Ok(relative) = entry.path().strip_prefix(&canonical_root) else {
                continue;
            };
            let relative_text = normalize(relative);
            if is_generated_path(&relative_text) || is_secret_bearing_path(&relative_text) {
                continue;
            }
            if documents.len() >= self.limits.max_files {
                truncated = true;
                continue;
            }
            let bytes = fs::read(entry.path())?;
            if bytes.len() > self.limits.max_bytes_per_file || bytes.contains(&0) {
                continue;
            }
            let Ok(content) = String::from_utf8(bytes) else {
                continue;
            };
            let syntax = parse_syntax(entry.path(), &content);
            let metadata = fs::metadata(entry.path())?;
            documents.push(IndexedCodeDocument {
                path: relative_text,
                content_hash: sha256(content.as_bytes()),
                content,
                modified_unix_secs: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs()),
                symbols: syntax.symbols().to_vec(),
                references: syntax.references().to_vec(),
                dependency_hints: syntax.dependency_hints().to_vec(),
            });
        }
        documents.sort_by(|left, right| left.path.cmp(&right.path));
        let git = git_metadata(&canonical_root);
        let generation_id = generation_id(&documents, &git);
        Ok(RepoCodeIndexSnapshot {
            root: canonical_root,
            generation_id,
            documents,
            git,
            truncated,
            limits: self.limits.clone(),
        })
    }
}

pub(crate) fn git_metadata(root: &Path) -> RepoGitMetadata {
    RepoGitMetadata {
        branch: run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"]),
        head: run_git(root, &["rev-parse", "HEAD"]),
        dirty_paths: run_git(root, &["status", "--porcelain"])
            .map(|output| {
                output
                    .lines()
                    .filter_map(|line| line.get(3..))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn run_git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout)
        .ok()?
        .trim_end()
        .to_string();
    (!value.trim().is_empty()).then_some(value)
}

fn generation_id(documents: &[IndexedCodeDocument], git: &RepoGitMetadata) -> String {
    let mut digest = Sha256::new();
    for document in documents {
        digest.update(document.path.as_bytes());
        digest.update(document.content_hash.as_bytes());
    }
    digest.update(git.branch.as_deref().unwrap_or_default().as_bytes());
    digest.update(git.head.as_deref().unwrap_or_default().as_bytes());
    format!("{:x}", digest.finalize())
}

fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn normalize(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_generated_path(path: &str) -> bool {
    [
        ".git",
        "node_modules",
        "target",
        "dist",
        "build",
        ".next",
        "coverage",
        ".cache",
    ]
    .iter()
    .any(|prefix| path == *prefix || path.starts_with(&format!("{prefix}/")))
}
