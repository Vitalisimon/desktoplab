#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginTrustState {
    Verified,
    Unverified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginCompatibility {
    required_desktoplab_version: String,
}

impl PluginCompatibility {
    #[must_use]
    pub fn requires_desktoplab(version: impl Into<String>) -> Self {
        Self {
            required_desktoplab_version: version.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginManifest {
    plugin_id: String,
    capabilities: Vec<String>,
    trust: PluginTrustState,
    compatibility: Option<PluginCompatibility>,
}

impl PluginManifest {
    #[must_use]
    pub fn community(plugin_id: impl Into<String>, capabilities: &[&str]) -> Self {
        Self::new(plugin_id, capabilities, PluginTrustState::Unverified)
    }

    #[must_use]
    pub fn verified(plugin_id: impl Into<String>, capabilities: &[&str]) -> Self {
        Self::new(plugin_id, capabilities, PluginTrustState::Verified)
    }

    #[must_use]
    pub fn with_compatibility(mut self, compatibility: PluginCompatibility) -> Self {
        self.compatibility = Some(compatibility);
        self
    }

    fn new(plugin_id: impl Into<String>, capabilities: &[&str], trust: PluginTrustState) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
            trust,
            compatibility: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedPlugin {
    manifest: PluginManifest,
    enabled: bool,
}

impl LoadedPlugin {
    #[must_use]
    pub fn trust(&self) -> PluginTrustState {
        self.manifest.trust
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginRouteStatus {
    Routable,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginRouteDecision {
    status: PluginRouteStatus,
    reasons: Vec<String>,
}

impl PluginRouteDecision {
    #[must_use]
    pub fn status(&self) -> PluginRouteStatus {
        self.status
    }

    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginHost {
    desktoplab_version: String,
    plugins: Vec<LoadedPlugin>,
}

impl PluginHost {
    #[must_use]
    pub fn new(desktoplab_version: impl Into<String>) -> Self {
        Self {
            desktoplab_version: desktoplab_version.into(),
            plugins: Vec::new(),
        }
    }

    pub fn load(&mut self, manifest: PluginManifest) -> LoadedPlugin {
        let plugin = LoadedPlugin {
            manifest,
            enabled: true,
        };
        self.plugins.push(plugin.clone());
        plugin
    }

    pub fn disable(&mut self, plugin_id: &str) {
        if let Some(plugin) = self.find_mut(plugin_id) {
            plugin.enabled = false;
        }
    }

    #[must_use]
    pub fn route(&self, plugin_id: &str) -> PluginRouteDecision {
        let Some(plugin) = self.find(plugin_id) else {
            return blocked("plugin_missing");
        };
        let mut reasons = Vec::new();
        if !plugin.enabled {
            reasons.push("plugin_disabled".to_string());
        }
        if plugin.manifest.trust == PluginTrustState::Unverified {
            reasons.push("unverified_plugin_requires_trust_approval".to_string());
        }
        if let Some(reason) = self.compatibility_reason(plugin) {
            reasons.push(reason);
        }
        if reasons.is_empty() {
            PluginRouteDecision {
                status: PluginRouteStatus::Routable,
                reasons,
            }
        } else {
            PluginRouteDecision {
                status: PluginRouteStatus::Blocked,
                reasons,
            }
        }
    }

    fn find(&self, plugin_id: &str) -> Option<&LoadedPlugin> {
        self.plugins
            .iter()
            .find(|plugin| plugin.manifest.plugin_id == plugin_id)
    }

    fn find_mut(&mut self, plugin_id: &str) -> Option<&mut LoadedPlugin> {
        self.plugins
            .iter_mut()
            .find(|plugin| plugin.manifest.plugin_id == plugin_id)
    }

    fn compatibility_reason(&self, plugin: &LoadedPlugin) -> Option<String> {
        let compatibility = plugin.manifest.compatibility.as_ref()?;
        (!version_satisfies(
            &self.desktoplab_version,
            &compatibility.required_desktoplab_version,
        ))
        .then(|| {
            format!(
                "plugin_incompatible:requires_desktoplab>={}",
                compatibility.required_desktoplab_version
            )
        })
    }
}

fn blocked(reason: &str) -> PluginRouteDecision {
    PluginRouteDecision {
        status: PluginRouteStatus::Blocked,
        reasons: vec![reason.to_string()],
    }
}

fn version_satisfies(current: &str, required: &str) -> bool {
    version_major(current) >= version_major(required)
}

fn version_major(version: &str) -> u64 {
    version
        .split('.')
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}
