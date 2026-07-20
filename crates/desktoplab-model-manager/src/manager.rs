use desktoplab_compatibility::{
    CompatibilityEngine, MatchRequest, ModelLicense, ProductModelSeedCatalog, ProductModelSeedEntry,
};

use crate::{ModelFamilyCatalog, ModelLicenseState, ModelParameterClass, ModelVariant};

#[derive(Clone, Debug, Default)]
pub struct ModelManager;

impl ModelManager {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn recommend(
        &self,
        engine: &CompatibilityEngine,
        request: MatchRequest,
    ) -> ModelRecommendation {
        let model_id = request.model_id().to_string();
        let decision = engine.evaluate(request);
        ModelRecommendation {
            model_id,
            recommended: decision.is_recommended(),
            reason: decision.reason().to_string(),
        }
    }

    #[must_use]
    pub fn rank_variants(
        &self,
        catalog: &ModelFamilyCatalog,
        memory_gb: u32,
        storage_available_mb: u64,
    ) -> Vec<ModelRecommendation> {
        catalog
            .variants()
            .iter()
            .map(|variant| {
                if variant.parameter_class() == ModelParameterClass::Cloud {
                    return ModelRecommendation::blocked(
                        variant.model_id(),
                        "cloud model available after provider connection",
                    );
                }
                if variant.expected_disk_mb() > storage_available_mb {
                    return ModelRecommendation::blocked(
                        variant.model_id(),
                        "not enough free storage",
                    );
                }
                if memory_gb < variant.required_memory_gb() {
                    return ModelRecommendation::blocked(
                        variant.model_id(),
                        "not recommended on this computer",
                    );
                }
                ModelRecommendation::recommended(variant.model_id(), "fits this machine")
            })
            .collect()
    }

    #[must_use]
    pub fn default_family_catalog(&self) -> ModelFamilyCatalog {
        ProductModelSeedCatalog::initial_coding()
            .entries()
            .iter()
            .fold(ModelFamilyCatalog::new(), |catalog, entry| {
                catalog.with_variant(variant_from_seed(entry))
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRecommendation {
    model_id: String,
    recommended: bool,
    reason: String,
}

impl ModelRecommendation {
    fn recommended(model_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            recommended: true,
            reason: reason.into(),
        }
    }

    fn blocked(model_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            recommended: false,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn is_recommended(&self) -> bool {
        self.recommended
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

fn variant_from_seed(entry: &ProductModelSeedEntry) -> ModelVariant {
    ModelVariant::new(
        entry.family_id(),
        entry.family_name(),
        entry.model_id(),
        parameter_class_from_seed(entry.parameter_class()),
        entry.expected_disk_mb(),
        entry.runtime_id(),
        entry.pull_ref(),
        entry.channel().as_str(),
    )
    .with_model_metadata(
        entry.parameters_billion(),
        entry.quantization(),
        entry.context_window_tokens(),
    )
    .with_required_memory_gb(entry.required_memory_gb())
    .with_license_state(license_state_from_seed(entry.license()))
    .with_capabilities(entry.capabilities())
}

fn parameter_class_from_seed(parameter_class: &str) -> ModelParameterClass {
    match parameter_class {
        "cloud" => ModelParameterClass::Cloud,
        "small" => ModelParameterClass::Small,
        "medium" => ModelParameterClass::Medium,
        "large" => ModelParameterClass::Large,
        "workstation" => ModelParameterClass::Workstation,
        _ => ModelParameterClass::Small,
    }
}

fn license_state_from_seed(license: ModelLicense) -> ModelLicenseState {
    match license {
        ModelLicense::Known => ModelLicenseState::Known,
        ModelLicense::Unknown => ModelLicenseState::Unknown,
        ModelLicense::Restricted => ModelLicenseState::Restricted,
    }
}
