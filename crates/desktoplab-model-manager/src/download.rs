use desktoplab_runtime::RuntimeId;

use crate::ModelVariant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DownloadPolicy {
    AutomaticAfterAccept,
    ManualOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupSelection {
    runtime_id: RuntimeId,
    model_id: String,
    expected_disk_mb: u64,
    accepted: bool,
}

impl SetupSelection {
    #[must_use]
    pub fn accepted(
        runtime_id: RuntimeId,
        model_id: impl Into<String>,
        expected_disk_mb: u64,
    ) -> Self {
        Self {
            runtime_id,
            model_id: model_id.into(),
            expected_disk_mb,
            accepted: true,
        }
    }

    #[must_use]
    pub fn preview(runtime_id: RuntimeId, model_id: impl Into<String>) -> Self {
        Self {
            runtime_id,
            model_id: model_id.into(),
            expected_disk_mb: 0,
            accepted: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDownloadPlan {
    runtime_id: RuntimeId,
    model_id: String,
    family_id: String,
    variant_id: String,
    pull_ref: String,
    expected_disk_mb: u64,
    verification: String,
    starts_automatically: bool,
}

impl ModelDownloadPlan {
    #[must_use]
    pub fn from_selection(selection: &SetupSelection, policy: DownloadPolicy) -> Self {
        Self {
            runtime_id: selection.runtime_id.clone(),
            model_id: selection.model_id.clone(),
            family_id: selection.model_id.clone(),
            variant_id: selection.model_id.clone(),
            pull_ref: selection.model_id.clone(),
            expected_disk_mb: selection.expected_disk_mb,
            verification: "selection metadata".to_string(),
            starts_automatically: selection.accepted
                && policy == DownloadPolicy::AutomaticAfterAccept,
        }
    }

    #[must_use]
    pub fn from_variant(variant: &ModelVariant, accepted: bool) -> Self {
        let compatibility = variant.runtime_compatibility();
        Self {
            runtime_id: RuntimeId::new(compatibility.runtime_id()),
            model_id: variant.model_id().to_string(),
            family_id: variant.family_id().to_string(),
            variant_id: variant.model_id().to_string(),
            pull_ref: compatibility.pull_ref().to_string(),
            expected_disk_mb: variant.expected_disk_mb(),
            verification: "runtime manifest plus local runtime inventory".to_string(),
            starts_automatically: accepted,
        }
    }

    #[must_use]
    pub fn starts_automatically(&self) -> bool {
        self.starts_automatically
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn family_id(&self) -> &str {
        &self.family_id
    }

    #[must_use]
    pub fn variant_id(&self) -> &str {
        &self.variant_id
    }

    #[must_use]
    pub fn pull_ref(&self) -> &str {
        &self.pull_ref
    }

    #[must_use]
    pub fn verification(&self) -> &str {
        &self.verification
    }

    #[must_use]
    pub fn expected_disk_mb(&self) -> u64 {
        self.expected_disk_mb
    }
}
