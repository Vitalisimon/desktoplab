use crate::{ManifestGroup, ManifestStatus};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryRecommendation {
    recommended: HashSet<String>,
    blocked: HashMap<String, String>,
}

impl RegistryRecommendation {
    #[must_use]
    pub fn from_group(group: &ManifestGroup) -> Self {
        let mut recommended = HashSet::new();
        let mut blocked = HashMap::new();

        for manifest in group.manifests() {
            if manifest.status().blocks_recommendation() {
                blocked.insert(
                    manifest.manifest_id().to_string(),
                    blocked_reason(manifest.status()).to_string(),
                );
            } else {
                recommended.insert(manifest.manifest_id().to_string());
            }
        }

        Self {
            recommended,
            blocked,
        }
    }

    #[must_use]
    pub fn is_recommended(&self, manifest_id: &str) -> bool {
        self.recommended.contains(manifest_id) && !self.blocked.contains_key(manifest_id)
    }

    #[must_use]
    pub fn blocked_reason(&self, manifest_id: &str) -> Option<&str> {
        self.blocked.get(manifest_id).map(String::as_str)
    }
}

fn blocked_reason(status: ManifestStatus) -> &'static str {
    match status {
        ManifestStatus::Blocked => "manifest status is blocked",
        ManifestStatus::Revoked => "manifest status is revoked",
        _ => "manifest status blocks recommendation",
    }
}
