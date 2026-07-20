use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryInspector {
    max_files: usize,
}

impl RepositoryInspector {
    #[must_use]
    pub fn new(max_files: usize) -> Self {
        Self { max_files }
    }

    pub fn inspect(&self, root: &Path) -> io::Result<RepositoryInspection> {
        let mut files = Vec::new();
        collect_files(root, root, &mut files)?;
        let degraded = files.len() > self.max_files;
        files.truncate(self.max_files);

        let mut languages = BTreeSet::new();
        let mut package_managers = BTreeSet::new();
        let mut summary = Vec::new();
        let mut protected_count = 0;

        for relative in files {
            let relative_text = relative.to_string_lossy().replace('\\', "/");
            if is_protected_path(&relative_text) {
                protected_count += 1;
                continue;
            }
            detect_language(&relative_text, &mut languages);
            detect_package_manager(&relative_text, &mut package_managers);
            summary.push(relative_text);
        }

        Ok(RepositoryInspection {
            languages,
            package_managers,
            summary,
            protected_count,
            degraded,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryInspection {
    languages: BTreeSet<String>,
    package_managers: BTreeSet<String>,
    summary: Vec<String>,
    protected_count: usize,
    degraded: bool,
}

impl RepositoryInspection {
    #[must_use]
    pub fn has_language(&self, language: &str) -> bool {
        self.languages.contains(language)
    }

    #[must_use]
    pub fn has_package_manager(&self, package_manager: &str) -> bool {
        self.package_managers.contains(package_manager)
    }

    #[must_use]
    pub fn protected_files_excluded(&self) -> bool {
        self.protected_count > 0
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    #[must_use]
    pub fn degraded_reason(&self) -> Option<&'static str> {
        self.degraded
            .then_some("workspace_scan_file_limit_exceeded")
    }

    #[must_use]
    pub fn summary_text(&self) -> String {
        let protected = if self.protected_files_excluded() {
            "\nprotected_files=[REDACTED]"
        } else {
            ""
        };
        format!("files={}{protected}", self.summary.join(","))
    }

    #[must_use]
    pub fn summary_paths(&self) -> &[String] {
        &self.summary
    }

    #[must_use]
    pub fn languages(&self) -> Vec<String> {
        self.languages.iter().cloned().collect()
    }

    #[must_use]
    pub fn package_managers(&self) -> Vec<String> {
        self.package_managers.iter().cloned().collect()
    }
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let relative_text = relative.to_string_lossy().replace('\\', "/");
        if should_skip_workspace_path(&relative_text) {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            files.push(relative.to_path_buf());
        }
    }
    files.sort();
    Ok(())
}

pub(crate) fn should_skip_workspace_path(path: &str) -> bool {
    path == ".DS_Store"
        || path.ends_with("/.DS_Store")
        || path == ".external-references"
        || path.starts_with(".external-references/")
        || path == ".git"
        || path.starts_with(".git/")
        || path == "build"
        || path.starts_with("build/")
        || path == "dist"
        || path.starts_with("dist/")
        || path == "node_modules"
        || path.starts_with("node_modules/")
        || path == "target"
        || path.starts_with("target/")
}

pub(crate) fn is_protected_path(path: &str) -> bool {
    let lower_path = path.to_ascii_lowercase();
    path.split('/').any(|segment| {
        let lower_segment = segment.to_ascii_lowercase();
        segment == ".env"
            || segment.starts_with(".env.")
            || segment == ".git"
            || segment == ".ssh"
            || segment == ".netrc"
            || segment == ".pypirc"
            || segment.contains("id_rsa")
            || segment.contains("id_dsa")
            || segment.contains("id_ecdsa")
            || segment.contains("credentials")
            || lower_segment.ends_with(".pem")
            || lower_segment.ends_with(".key")
            || lower_segment.ends_with(".p12")
            || lower_segment.ends_with(".pfx")
    }) || lower_path.contains("id_ed25519")
        || lower_path.contains("id_rsa")
        || lower_path.contains("id_dsa")
        || lower_path.contains("id_ecdsa")
        || lower_path.contains("credentials")
}

fn detect_language(path: &str, languages: &mut BTreeSet<String>) {
    if path.ends_with(".rs") || path == "Cargo.toml" {
        languages.insert("rust".to_string());
    }
    if path.ends_with(".ts") || path.ends_with(".tsx") || path == "package.json" {
        languages.insert("typescript".to_string());
    }
    if path.ends_with(".py") || path == "pyproject.toml" {
        languages.insert("python".to_string());
    }
}

fn detect_package_manager(path: &str, package_managers: &mut BTreeSet<String>) {
    match path {
        "Cargo.toml" => {
            package_managers.insert("cargo".to_string());
        }
        "package.json" => {
            package_managers.insert("npm".to_string());
        }
        "pyproject.toml" => {
            package_managers.insert("python".to_string());
        }
        _ => {}
    }
}
