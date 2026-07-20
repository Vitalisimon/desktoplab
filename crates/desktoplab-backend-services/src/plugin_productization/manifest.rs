use crate::PluginTrustState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginPermissionKind {
    LlmChat,
    FilesystemWrite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginRuntimeState {
    Enabled,
    Disabled,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginDistributionKind {
    LocalLoaded,
    RegistryInstallable,
    MarketplaceFuture,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PluginExecutionBoundaryKind {
    DisplayOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginExecutionBoundary {
    kind: PluginExecutionBoundaryKind,
    reason: String,
}

impl PluginExecutionBoundary {
    #[must_use]
    pub fn kind(&self) -> PluginExecutionBoundaryKind {
        self.kind
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginInstallBoundary {
    available: bool,
    reason: String,
}

impl PluginInstallBoundary {
    #[must_use]
    pub fn available(&self) -> bool {
        self.available
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginProductManifest {
    plugin_id: String,
    contract_version: String,
    source: String,
    trust: PluginTrustState,
    install_policy: String,
    auth_policy: String,
    category: String,
    capabilities: Vec<String>,
    requires: Vec<String>,
    permissions: Vec<PluginPermissionKind>,
    hooks: Vec<String>,
    runtime_state: PluginRuntimeState,
    distribution_kind: PluginDistributionKind,
    install_boundary: PluginInstallBoundary,
    execution_boundary: PluginExecutionBoundary,
}

impl PluginProductManifest {
    #[must_use]
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    #[must_use]
    pub fn trust(&self) -> PluginTrustState {
        self.trust
    }

    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    #[must_use]
    pub fn install_policy(&self) -> &str {
        &self.install_policy
    }

    #[must_use]
    pub fn auth_policy(&self) -> &str {
        &self.auth_policy
    }

    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    #[must_use]
    pub fn requires(&self) -> &[String] {
        &self.requires
    }

    #[must_use]
    pub fn permissions(&self) -> &[PluginPermissionKind] {
        &self.permissions
    }

    #[must_use]
    pub fn has_permission(&self, permission: PluginPermissionKind) -> bool {
        self.permissions.contains(&permission)
    }

    #[must_use]
    pub fn runtime_state(&self) -> PluginRuntimeState {
        self.runtime_state
    }

    #[must_use]
    pub fn distribution_kind(&self) -> PluginDistributionKind {
        self.distribution_kind
    }

    #[must_use]
    pub fn install_boundary(&self) -> &PluginInstallBoundary {
        &self.install_boundary
    }

    #[must_use]
    pub fn execution_boundary(&self) -> &PluginExecutionBoundary {
        &self.execution_boundary
    }

    #[must_use]
    pub fn contract_version(&self) -> &str {
        &self.contract_version
    }

    pub(super) fn set_trust(&mut self, trust: PluginTrustState) {
        self.trust = trust;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PluginManifestLoader {
    supported_contract_version: String,
    executed_plugin_code_count: usize,
}

impl PluginManifestLoader {
    #[must_use]
    pub fn new(_desktoplab_version: &str) -> Self {
        Self {
            supported_contract_version: "1".to_string(),
            executed_plugin_code_count: 0,
        }
    }

    pub fn load_from_str(&self, source: &str) -> Result<PluginProductManifest, String> {
        let plugin_id = extract_string(source, "plugin_id").ok_or("plugin_id_required")?;
        if plugin_id.is_empty() {
            return Err("plugin_id_required".to_string());
        }
        let contract_version =
            extract_string(source, "contract_version").ok_or("contract_version_required")?;
        if contract_version != self.supported_contract_version {
            return Err("unsupported_contract_version".to_string());
        }
        let trust = match extract_string(source, "trust").as_deref() {
            Some("verified") => PluginTrustState::Verified,
            Some("unverified") => PluginTrustState::Unverified,
            _ => PluginTrustState::Unverified,
        };
        let permissions = extract_array(source, "permissions")
            .into_iter()
            .map(|permission| match permission.as_str() {
                "llm.chat" => Ok(PluginPermissionKind::LlmChat),
                "tool.filesystem.write" => Ok(PluginPermissionKind::FilesystemWrite),
                _ => Err("unknown_permission".to_string()),
            })
            .collect::<Result<Vec<_>, _>>()?;
        if permissions.is_empty() {
            return Err("permissions_required".to_string());
        }
        let distribution_kind = distribution_kind(source);
        Ok(PluginProductManifest {
            plugin_id,
            contract_version,
            source: extract_string(source, "source")
                .unwrap_or_else(|| "local_manifest".to_string()),
            trust,
            install_policy: extract_string(source, "install_policy")
                .unwrap_or_else(|| "manual_review_required".to_string()),
            auth_policy: extract_string(source, "auth_policy")
                .unwrap_or_else(|| "no_auth_requested".to_string()),
            category: extract_string(source, "category").unwrap_or_else(|| "general".to_string()),
            capabilities: extract_array(source, "capabilities"),
            requires: extract_array(source, "requires"),
            permissions,
            hooks: extract_array(source, "hooks"),
            runtime_state: runtime_state(source),
            distribution_kind,
            install_boundary: install_boundary(distribution_kind),
            execution_boundary: execution_boundary(),
        })
    }

    #[must_use]
    pub fn executed_plugin_code_count(&self) -> usize {
        self.executed_plugin_code_count
    }
}

fn execution_boundary() -> PluginExecutionBoundary {
    PluginExecutionBoundary {
        kind: PluginExecutionBoundaryKind::DisplayOnly,
        reason:
            "Community plugins are display-only until signed package verification and out-of-process execution are implemented."
                .to_string(),
    }
}

fn distribution_kind(source: &str) -> PluginDistributionKind {
    match extract_string(source, "distribution").as_deref() {
        Some("registry_installable") => PluginDistributionKind::RegistryInstallable,
        Some("marketplace_future") => PluginDistributionKind::MarketplaceFuture,
        _ => PluginDistributionKind::LocalLoaded,
    }
}

fn install_boundary(distribution_kind: PluginDistributionKind) -> PluginInstallBoundary {
    match distribution_kind {
        PluginDistributionKind::LocalLoaded => PluginInstallBoundary {
            available: false,
            reason: "Plugin is already loaded from a local manifest.".to_string(),
        },
        PluginDistributionKind::RegistryInstallable => PluginInstallBoundary {
            available: false,
            reason: "Plugin registry download is not available before marketplace distribution is designed.".to_string(),
        },
        PluginDistributionKind::MarketplaceFuture => PluginInstallBoundary {
            available: false,
            reason: "Plugin marketplace distribution is not available in this phase.".to_string(),
        },
    }
}

fn runtime_state(source: &str) -> PluginRuntimeState {
    match extract_string(source, "state").as_deref() {
        Some("disabled") => PluginRuntimeState::Disabled,
        Some("blocked") => PluginRuntimeState::Blocked,
        _ => PluginRuntimeState::Enabled,
    }
}

fn extract_string(source: &str, key: &str) -> Option<String> {
    let marker = format!(r#""{key}":"#);
    let after = source.split(&marker).nth(1)?.trim_start();
    let after = after.strip_prefix('"')?;
    Some(after.split('"').next()?.to_string())
}

fn extract_array(source: &str, key: &str) -> Vec<String> {
    let marker = format!(r#""{key}":["#);
    let Some(after) = source.split(&marker).nth(1) else {
        return Vec::new();
    };
    let Some(raw) = after.split(']').next() else {
        return Vec::new();
    };
    raw.split(',')
        .filter_map(|item| item.trim().trim_matches('"').split('"').next())
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}
