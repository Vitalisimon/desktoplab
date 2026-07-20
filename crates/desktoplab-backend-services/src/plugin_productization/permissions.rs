use std::collections::BTreeMap;

use crate::PluginTrustState;

use super::manifest::{PluginPermissionKind, PluginProductManifest, PluginRuntimeState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginTrustAction {
    UserApproved,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginProductizationHost {
    desktoplab_version: String,
    manifests: BTreeMap<String, PluginProductManifest>,
}

impl PluginProductizationHost {
    #[must_use]
    pub fn new(desktoplab_version: &str) -> Self {
        Self {
            desktoplab_version: desktoplab_version.to_string(),
            manifests: BTreeMap::new(),
        }
    }

    pub fn load_manifest(&mut self, manifest: PluginProductManifest) -> Result<(), String> {
        if manifest.runtime_state() == PluginRuntimeState::Blocked {
            return Err("plugin_blocked".to_string());
        }
        self.manifests
            .insert(manifest.plugin_id().to_string(), manifest);
        Ok(())
    }

    pub fn apply_trust_action(
        &mut self,
        plugin_id: &str,
        action: PluginTrustAction,
    ) -> Result<(), String> {
        let _ = (plugin_id, action);
        Err("approval_record_required".to_string())
    }

    pub fn apply_trust_action_with_approval(
        &mut self,
        plugin_id: &str,
        action: PluginTrustAction,
        approval_record_id: &str,
    ) -> Result<(), String> {
        if approval_record_id.is_empty() {
            return Err("approval_record_required".to_string());
        }
        let Some(manifest) = self.manifests.get_mut(plugin_id) else {
            return Err("plugin_missing".to_string());
        };
        match action {
            PluginTrustAction::UserApproved => manifest.set_trust(PluginTrustState::Verified),
        }
        Ok(())
    }

    #[must_use]
    pub fn manifest(&self, plugin_id: &str) -> Option<&PluginProductManifest> {
        self.manifests.get(plugin_id)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PluginPermissionEngine {
    _private: (),
}

impl PluginPermissionEngine {
    #[must_use]
    pub fn authorize(
        &self,
        host: &PluginProductizationHost,
        plugin_id: &str,
    ) -> PluginAuthorization {
        let Some(manifest) = host.manifest(plugin_id) else {
            return PluginAuthorization::blocked(vec!["plugin_missing"]);
        };
        if manifest.runtime_state() == PluginRuntimeState::Disabled {
            return PluginAuthorization::blocked(vec!["plugin_disabled"]);
        }
        if manifest.trust() == PluginTrustState::Unverified
            && manifest.has_permission(PluginPermissionKind::FilesystemWrite)
        {
            return PluginAuthorization::blocked(vec!["unverified_sensitive_permission"]);
        }
        PluginAuthorization::allowed()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginAuthorization {
    allowed: bool,
    reasons: Vec<String>,
}

impl PluginAuthorization {
    fn allowed() -> Self {
        Self {
            allowed: true,
            reasons: Vec::new(),
        }
    }

    fn blocked(reasons: Vec<&str>) -> Self {
        Self {
            allowed: false,
            reasons: reasons.into_iter().map(ToString::to_string).collect(),
        }
    }

    #[must_use]
    pub fn is_allowed(&self) -> bool {
        self.allowed
    }

    #[must_use]
    pub fn is_blocked(&self) -> bool {
        !self.allowed
    }

    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
}
