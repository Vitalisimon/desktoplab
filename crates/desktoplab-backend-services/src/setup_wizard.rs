use desktoplab_hardware_wizard::{HardwareWizard, ProbeSnapshot, WarningCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogChannel {
    Stable,
    Beta,
    Experimental,
}

impl CatalogChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogEntryKind {
    Runtime,
    Model,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupWizardRegistryState {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetupWizardPolicy {
    max_channel: CatalogChannel,
}

impl SetupWizardPolicy {
    #[must_use]
    pub fn stable_only() -> Self {
        Self {
            max_channel: CatalogChannel::Stable,
        }
    }

    #[must_use]
    pub fn allow_beta() -> Self {
        Self {
            max_channel: CatalogChannel::Beta,
        }
    }

    #[must_use]
    pub fn allow_experimental() -> Self {
        Self {
            max_channel: CatalogChannel::Experimental,
        }
    }

    fn allows(self, channel: CatalogChannel) -> bool {
        channel_rank(channel) <= channel_rank(self.max_channel)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupCatalogEntry {
    manifest_id: String,
    display_name: String,
    kind: CatalogEntryKind,
    channel: CatalogChannel,
    runtime_id: Option<String>,
}

impl SetupCatalogEntry {
    #[must_use]
    pub fn runtime(
        manifest_id: impl Into<String>,
        display_name: impl Into<String>,
        channel: CatalogChannel,
    ) -> Self {
        Self::new(
            manifest_id,
            display_name,
            CatalogEntryKind::Runtime,
            channel,
        )
    }

    #[must_use]
    pub fn model(
        manifest_id: impl Into<String>,
        display_name: impl Into<String>,
        channel: CatalogChannel,
    ) -> Self {
        Self::new(manifest_id, display_name, CatalogEntryKind::Model, channel)
    }

    #[must_use]
    pub fn for_runtime(mut self, runtime_id: impl Into<String>) -> Self {
        self.runtime_id = Some(runtime_id.into());
        self
    }

    fn new(
        manifest_id: impl Into<String>,
        display_name: impl Into<String>,
        kind: CatalogEntryKind,
        channel: CatalogChannel,
    ) -> Self {
        Self {
            manifest_id: manifest_id.into(),
            display_name: display_name.into(),
            kind,
            channel,
            runtime_id: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupRecommendation {
    manifest_id: String,
    display_name: String,
    channel: CatalogChannel,
    role: SetupRecommendationRole,
}

impl SetupRecommendation {
    #[must_use]
    pub fn manifest_id(&self) -> &str {
        &self.manifest_id
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub fn channel(&self) -> &'static str {
        self.channel.as_str()
    }

    #[must_use]
    pub fn role(&self) -> SetupRecommendationRole {
        self.role
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupRecommendationRole {
    Recommended,
    Alternative,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupPlanPreview {
    pub(crate) registry_state: SetupWizardRegistryState,
    pub(crate) runtime_recommendations: Vec<SetupRecommendation>,
    pub(crate) model_recommendations: Vec<SetupRecommendation>,
    warnings: Vec<WarningCode>,
    expected_limitations: Vec<String>,
    hidden_reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogRefreshRequestState {
    Available,
    BlockedNoSafeCatalog,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogRefreshRequestResult {
    pub job_id: Option<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogRefreshStatus {
    pub state: SetupWizardRegistryState,
    pub last_known_good_available: bool,
    pub degraded_reasons: Vec<String>,
    pub manual_refresh: CatalogRefreshRequestResult,
}

impl SetupPlanPreview {
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.registry_state != SetupWizardRegistryState::Blocked
            && !self.runtime_recommendations.is_empty()
    }

    #[must_use]
    pub fn runtime_recommendations(&self) -> &[SetupRecommendation] {
        &self.runtime_recommendations
    }

    #[must_use]
    pub fn model_recommendations(&self) -> &[SetupRecommendation] {
        &self.model_recommendations
    }

    #[must_use]
    pub fn warnings(&self) -> &[WarningCode] {
        &self.warnings
    }

    #[must_use]
    pub fn expected_limitations(&self) -> &[String] {
        &self.expected_limitations
    }

    #[must_use]
    pub fn hidden_reasons(&self) -> &[String] {
        &self.hidden_reasons
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SetupWizardApiService;

impl SetupWizardApiService {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn preview(
        &self,
        snapshot: ProbeSnapshot,
        registry_state: SetupWizardRegistryState,
        policy: SetupWizardPolicy,
        entries: Vec<SetupCatalogEntry>,
    ) -> SetupPlanPreview {
        let wizard = HardwareWizard::v1();
        let profile = wizard.profile(snapshot);
        let recommendation_inputs = wizard.recommendation_inputs(&profile);
        let mut runtime_recommendations = Vec::new();
        let mut model_recommendations = Vec::new();
        let mut hidden_reasons = Vec::new();

        for entry in entries {
            if !policy.allows(entry.channel) {
                hidden_reasons.push(format!(
                    "{}:hidden_channel:{}",
                    entry.manifest_id,
                    entry.channel.as_str()
                ));
                continue;
            }
            match entry.kind {
                CatalogEntryKind::Runtime => {
                    let role = role_for_next(runtime_recommendations.is_empty());
                    runtime_recommendations.push(recommendation_from_entry(entry, role));
                }
                CatalogEntryKind::Model => {
                    let role = role_for_next(model_recommendations.is_empty());
                    model_recommendations.push(recommendation_from_entry(entry, role));
                }
            }
        }

        let mut expected_limitations = recommendation_inputs.expected_limitations().to_vec();
        match registry_state {
            SetupWizardRegistryState::Ready => {}
            SetupWizardRegistryState::Degraded => expected_limitations.push(
                "compatibility catalog refresh unavailable; using last-known-good catalog"
                    .to_string(),
            ),
            SetupWizardRegistryState::Blocked => {
                expected_limitations.push("no safe compatibility catalog is available".to_string())
            }
        }

        SetupPlanPreview {
            registry_state,
            runtime_recommendations,
            model_recommendations,
            warnings: profile.warnings().to_vec(),
            expected_limitations,
            hidden_reasons,
        }
    }

    #[must_use]
    pub fn catalog_refresh_status(
        &self,
        state: SetupWizardRegistryState,
        last_known_good_available: bool,
        degraded_reasons: Vec<String>,
    ) -> CatalogRefreshStatus {
        let request_state =
            if state == SetupWizardRegistryState::Blocked && !last_known_good_available {
                CatalogRefreshRequestState::BlockedNoSafeCatalog
            } else {
                CatalogRefreshRequestState::Available
            };
        CatalogRefreshStatus {
            state,
            last_known_good_available,
            degraded_reasons,
            manual_refresh: self.request_catalog_refresh(request_state),
        }
    }

    #[must_use]
    pub fn request_catalog_refresh(
        &self,
        state: CatalogRefreshRequestState,
    ) -> CatalogRefreshRequestResult {
        match state {
            CatalogRefreshRequestState::Available => CatalogRefreshRequestResult {
                job_id: Some("registry.refresh.manual".to_string()),
                blocked_reason: None,
            },
            CatalogRefreshRequestState::BlockedNoSafeCatalog => CatalogRefreshRequestResult {
                job_id: None,
                blocked_reason: Some("No safe compatibility catalog is available.".to_string()),
            },
        }
    }
}

fn role_for_next(is_first: bool) -> SetupRecommendationRole {
    if is_first {
        SetupRecommendationRole::Recommended
    } else {
        SetupRecommendationRole::Alternative
    }
}

fn recommendation_from_entry(
    entry: SetupCatalogEntry,
    role: SetupRecommendationRole,
) -> SetupRecommendation {
    SetupRecommendation {
        manifest_id: entry.manifest_id,
        display_name: entry.display_name,
        channel: entry.channel,
        role,
    }
}

fn channel_rank(channel: CatalogChannel) -> u8 {
    match channel {
        CatalogChannel::Stable => 0,
        CatalogChannel::Beta => 1,
        CatalogChannel::Experimental => 2,
    }
}
