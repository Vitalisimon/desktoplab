use std::collections::BTreeMap;

use crate::inspection::is_protected_path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathClassification {
    LocalOnly,
    GeneratedVendor,
    Shareable,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspacePolicyClassifier {
    _private: (),
}

impl WorkspacePolicyClassifier {
    #[must_use]
    pub fn classify_paths<I, P>(&self, paths: I) -> ClassifiedWorkspacePaths
    where
        I: IntoIterator<Item = P>,
        P: AsRef<str>,
    {
        let classifications = paths
            .into_iter()
            .map(|path| {
                let path = path.as_ref().to_string();
                let classification = classify_path(&path);
                (path, classification)
            })
            .collect();
        ClassifiedWorkspacePaths { classifications }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClassifiedWorkspacePaths {
    classifications: BTreeMap<String, PathClassification>,
}

impl ClassifiedWorkspacePaths {
    #[must_use]
    pub fn is_local_only(&self, path: &str) -> bool {
        self.classifications.get(path) == Some(&PathClassification::LocalOnly)
    }

    #[must_use]
    pub fn is_shareable(&self, path: &str) -> bool {
        self.classifications.get(path) == Some(&PathClassification::Shareable)
    }

    #[must_use]
    pub fn provider_context_paths(&self) -> Vec<&str> {
        self.classifications
            .iter()
            .filter(|(_, classification)| **classification == PathClassification::Shareable)
            .map(|(path, _)| path.as_str())
            .collect()
    }

    #[must_use]
    pub fn override_for_provider(&self, path: &str, reason: &str) -> PolicyOverrideRecord {
        PolicyOverrideRecord {
            path: path.to_string(),
            reason: reason.to_string(),
            audited: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyOverrideRecord {
    path: String,
    reason: String,
    audited: bool,
}

impl PolicyOverrideRecord {
    #[must_use]
    pub fn is_audited(&self) -> bool {
        self.audited && !self.reason.is_empty() && !self.path.is_empty()
    }
}

fn classify_path(path: &str) -> PathClassification {
    if is_protected_path(path) {
        PathClassification::LocalOnly
    } else if path.starts_with("node_modules/") || path.starts_with("target/") {
        PathClassification::GeneratedVendor
    } else {
        PathClassification::Shareable
    }
}
