use desktoplab_backend_services::{
    CatalogChannel, SetupCatalogEntry, SetupRecommendation, SetupRecommendationRole,
    SetupWizardApiService, SetupWizardPolicy, SetupWizardRegistryState,
};
use desktoplab_hardware_wizard::{
    AcceleratorKind, Confidence, HardwareObservation, HardwareProbeAdapter, HardwareProfile,
    HardwareWizard, WarningCode,
};
use desktoplab_model_manager::{
    AgentContextWindowPolicy, AgentRequestTimeoutPolicy, ModelDownloadPlan, ModelFamilyCatalog,
    ModelManager, ModelParameterClass, ModelRecommendation, ModelVariant,
};
use desktoplab_runtime::{ProcessCommand, ProcessRunner, SystemProcessRunner};
use serde_json::{Value, json};

#[must_use]
pub fn setup_preview_response() -> Value {
    let snapshot = HardwareProbeAdapter::for_current_host().snapshot();
    let profile = HardwareWizard::v1().profile(snapshot.clone());
    let service = SetupWizardApiService::new();
    let model_manager = ModelManager::new();
    let model_catalog = model_manager.default_family_catalog();
    let available_memory_gb = effective_memory_gb(&profile);
    let host = HostSetupInventory::detect();
    let high_end_local = crate::frontier_setup_preview::frontier_setup_preview();
    let model_fit = model_manager.rank_variants(
        &model_catalog,
        available_memory_gb,
        storage_available_mb(&profile),
    );
    let preview = service.preview(
        snapshot,
        SetupWizardRegistryState::Ready,
        SetupWizardPolicy::allow_experimental(),
        setup_entries(&model_catalog, &model_fit),
    );
    let hidden_reasons = preview
        .hidden_reasons()
        .iter()
        .cloned()
        .chain(hidden_model_reasons(&model_catalog, &model_fit))
        .collect::<Vec<_>>();

    json!({
        "source":"service_backed",
        "catalogSource":"bundled_seed_catalog",
        "registryState":"ready",
        "hardware":{
            "cpu":string_observation("CPU", profile.cpu()),
            "ramGb":number_observation("RAM", profile.ram_gb()),
            "gpu":string_observation("GPU", profile.gpu()),
            "acceleratorKind":accelerator_kind_observation(profile.accelerator_kind()),
            "vramGb":number_observation("VRAM", profile.vram_gb()),
            "unifiedMemoryGb":number_observation("Unified memory", profile.unified_memory_gb()),
            "operatingSystem":fact("OS", std::env::consts::OS),
            "architecture":fact("Architecture", std::env::consts::ARCH),
            "storageAvailableGb":number_observation("Storage", profile.storage_available_gb())
        },
        "highEndLocal":high_end_local,
        "runtimeRecommendations":preview.runtime_recommendations().iter().map(|recommendation| recommendation_json(recommendation, &host)).collect::<Vec<_>>(),
        "modelRecommendations":preview.model_recommendations().iter().map(|recommendation| model_recommendation_json(recommendation, &model_catalog, &model_fit, &host, available_memory_gb)).collect::<Vec<_>>(),
        "warnings":preview.warnings().iter().map(warning_code).collect::<Vec<_>>(),
        "expectedLimitations":preview.expected_limitations(),
        "hiddenReasons":hidden_reasons
    })
}

#[must_use]
pub fn catalog_refresh_status_response() -> Value {
    let service = SetupWizardApiService::new();
    let status = service.catalog_refresh_status(SetupWizardRegistryState::Ready, true, Vec::new());
    json!({
        "source":"service_backed",
        "state":registry_state(status.state),
        "lastKnownGoodAvailable":status.last_known_good_available,
        "degradedReasons":status.degraded_reasons,
        "manualRefresh":{
            "available":status.manual_refresh.blocked_reason.is_none(),
            "jobId":status.manual_refresh.job_id,
            "blockedReason":status.manual_refresh.blocked_reason
        }
    })
}

fn setup_entries(
    model_catalog: &ModelFamilyCatalog,
    model_fit: &[ModelRecommendation],
) -> Vec<SetupCatalogEntry> {
    let mut entries = vec![
        SetupCatalogEntry::runtime("runtime.ollama", "Ollama", CatalogChannel::Stable),
        SetupCatalogEntry::runtime("runtime.lm-studio", "LM Studio", CatalogChannel::Stable),
    ];
    if host_supports_mlx_lm() {
        entries.push(SetupCatalogEntry::runtime(
            "runtime.mlx-lm",
            "MLX-LM Server",
            CatalogChannel::Stable,
        ));
    }
    entries.extend(
        model_catalog
            .variants()
            .iter()
            .filter(|variant| setup_runtime_available(variant.runtime_compatibility().runtime_id()))
            .filter(|variant| model_fits_host(variant.model_id(), model_fit))
            .map(model_entry),
    );
    entries
}

fn model_fits_host(model_id: &str, model_fit: &[ModelRecommendation]) -> bool {
    model_fit
        .iter()
        .find(|fit| fit.model_id() == model_id)
        .is_some_and(ModelRecommendation::is_recommended)
}

fn hidden_model_reasons<'a>(
    model_catalog: &'a ModelFamilyCatalog,
    model_fit: &'a [ModelRecommendation],
) -> impl Iterator<Item = String> + 'a {
    model_fit
        .iter()
        .filter(|fit| !fit.is_recommended())
        .filter_map(|fit| {
            model_catalog
                .variants()
                .iter()
                .find(|variant| variant.model_id() == fit.model_id())
                .map(|variant| {
                    format!(
                        "{}:hidden_hardware:{}",
                        variant.model_id(),
                        parameter_class(variant.parameter_class())
                    )
                })
        })
}

fn setup_runtime_available(runtime_id: &str) -> bool {
    matches!(
        runtime_id,
        "runtime.ollama" | "runtime.ollama-cloud" | "runtime.lm-studio"
    ) || (runtime_id == "runtime.mlx-lm" && host_supports_mlx_lm())
}

fn host_supports_mlx_lm() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

fn model_entry(variant: &ModelVariant) -> SetupCatalogEntry {
    SetupCatalogEntry::model(
        variant.model_id(),
        model_display_name(variant),
        channel_from_str(variant.channel()),
    )
    .for_runtime(variant.runtime_compatibility().runtime_id())
}

fn model_display_name(variant: &ModelVariant) -> String {
    format!(
        "{} {}",
        variant.family_name(),
        parameter_class(variant.parameter_class())
    )
}

fn parameter_class(parameter_class: ModelParameterClass) -> &'static str {
    match parameter_class {
        ModelParameterClass::Cloud => "cloud",
        ModelParameterClass::Small => "small",
        ModelParameterClass::Medium => "medium",
        ModelParameterClass::Large => "large",
        ModelParameterClass::Workstation => "workstation",
    }
}

fn channel_from_str(channel: &str) -> CatalogChannel {
    match channel {
        "experimental" => CatalogChannel::Experimental,
        "beta" => CatalogChannel::Beta,
        _ => CatalogChannel::Stable,
    }
}

fn recommendation_json(recommendation: &SetupRecommendation, host: &HostSetupInventory) -> Value {
    let mut value = json!({
        "manifestId":recommendation.manifest_id(),
        "displayName":recommendation.display_name(),
        "channel":recommendation.channel(),
        "role":role_value(recommendation.role())
    });
    if let Some(object) = value.as_object_mut() {
        if recommendation.manifest_id().starts_with("runtime.") {
            object.insert(
                "installMode".to_string(),
                json!(runtime_install_mode(recommendation.manifest_id())),
            );
            if recommendation.manifest_id() == "runtime.ollama" && host.ollama_installed {
                object.insert("hostInstallState".to_string(), json!("installed"));
                object.insert("defaultSetupChoice".to_string(), json!("use_existing"));
                object.insert("setupChoiceRequired".to_string(), json!(true));
            }
        }
    }
    value
}

fn model_recommendation_json(
    recommendation: &SetupRecommendation,
    model_catalog: &ModelFamilyCatalog,
    model_fit: &[ModelRecommendation],
    host: &HostSetupInventory,
    available_memory_gb: u32,
) -> Value {
    let Some(variant) = model_catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == recommendation.manifest_id())
    else {
        return recommendation_json(recommendation, host);
    };
    let mut value = recommendation_json(recommendation, host);
    if let Some(object) = value.as_object_mut() {
        object.insert("familyId".to_string(), json!(variant.family_id()));
        object.insert("familyName".to_string(), json!(variant.family_name()));
        object.insert(
            "parameterClass".to_string(),
            json!(parameter_class_json(variant.parameter_class())),
        );
        object.insert(
            "parametersBillion".to_string(),
            json!(variant.parameters_billion()),
        );
        object.insert("quantization".to_string(), json!(variant.quantization()));
        object.insert(
            "contextWindowTokens".to_string(),
            json!(variant.context_window_tokens()),
        );
        object.insert(
            "agentContextWindowTokens".to_string(),
            json!(AgentContextWindowPolicy::for_variant(
                variant,
                available_memory_gb
            )),
        );
        object.insert(
            "agentRequestTimeoutSeconds".to_string(),
            json!(AgentRequestTimeoutPolicy::for_variant(
                variant,
                available_memory_gb
            )),
        );
        object.insert(
            "requiredMemoryGb".to_string(),
            json!(variant.required_memory_gb()),
        );
        object.insert(
            "expectedDiskMb".to_string(),
            json!(variant.expected_disk_mb()),
        );
        object.insert(
            "runtimeId".to_string(),
            json!(variant.runtime_compatibility().runtime_id()),
        );
        object.insert(
            "licenseState".to_string(),
            json!(variant.license_state().as_str()),
        );
        object.insert(
            "trustLabel".to_string(),
            json!(variant.license_state().trust_label()),
        );
        object.insert(
            "agentQualification".to_string(),
            json!("runtime_validation_required"),
        );
        if let Some(fit) = model_fit
            .iter()
            .find(|fit| fit.model_id() == recommendation.manifest_id())
        {
            object.insert("compatibilityReason".to_string(), json!(fit.reason()));
        }
        let pull_ref = ModelDownloadPlan::from_variant(variant, true)
            .pull_ref()
            .to_string();
        if host.has_model(&pull_ref) {
            object.insert("hostInstallState".to_string(), json!("installed"));
            object.insert("defaultSetupChoice".to_string(), json!("use_existing"));
            object.insert("setupChoiceRequired".to_string(), json!(true));
        }
    }
    value
}

fn parameter_class_json(parameter_class: ModelParameterClass) -> &'static str {
    match parameter_class {
        ModelParameterClass::Cloud => "cloud",
        ModelParameterClass::Small => "small",
        ModelParameterClass::Medium => "medium",
        ModelParameterClass::Large => "large",
        ModelParameterClass::Workstation => "workstation",
    }
}

struct HostSetupInventory {
    ollama_installed: bool,
    ollama_models: Vec<String>,
}

impl HostSetupInventory {
    fn detect() -> Self {
        let version = <SystemProcessRunner as ProcessRunner>::run(
            &SystemProcessRunner,
            ProcessCommand::new("ollama").arg("--version"),
        );
        if !version.succeeded() {
            return Self {
                ollama_installed: false,
                ollama_models: Vec::new(),
            };
        }
        let list = <SystemProcessRunner as ProcessRunner>::run(
            &SystemProcessRunner,
            ProcessCommand::new("ollama").arg("list"),
        );
        Self {
            ollama_installed: true,
            ollama_models: if list.succeeded() {
                list.stdout()
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToString::to_string)
                    .collect()
            } else {
                Vec::new()
            },
        }
    }

    fn has_model(&self, pull_ref: &str) -> bool {
        self.ollama_models
            .iter()
            .any(|model| inventory_entry_has_model(model, pull_ref))
    }
}

fn inventory_entry_has_model(entry: &str, pull_ref: &str) -> bool {
    let entry = entry.trim();
    entry == pull_ref
        || entry.starts_with(pull_ref)
        || entry
            .split_whitespace()
            .any(|token| token == pull_ref || token.starts_with(pull_ref))
}

fn effective_memory_gb(profile: &HardwareProfile) -> u32 {
    profile
        .ram_gb()
        .value()
        .max(profile.vram_gb().value())
        .max(profile.unified_memory_gb().value())
}

fn storage_available_mb(profile: &HardwareProfile) -> u64 {
    u64::from(profile.storage_available_gb().value()) * 1024
}

fn role_value(role: SetupRecommendationRole) -> &'static str {
    match role {
        SetupRecommendationRole::Recommended => "recommended",
        SetupRecommendationRole::Alternative => "alternative",
    }
}

fn runtime_install_mode(runtime_id: &str) -> &'static str {
    match runtime_id {
        "runtime.lm-studio" => "external_guided",
        "runtime.mlx-lm" => "python_environment",
        _ => "automatic",
    }
}

fn registry_state(state: SetupWizardRegistryState) -> &'static str {
    match state {
        SetupWizardRegistryState::Ready => "ready",
        SetupWizardRegistryState::Degraded => "degraded",
        SetupWizardRegistryState::Blocked => "blocked",
    }
}

fn fact(label: &str, value: &str) -> Value {
    json!({"label":label,"value":value,"confidence":"confirmed"})
}

fn string_observation(label: &str, observation: &HardwareObservation<String>) -> Value {
    let confidence = confidence_value(observation.confidence());
    if observation.confidence() == Confidence::Unknown {
        return json!({"label":label,"value":Value::Null,"confidence":confidence});
    }
    json!({"label":label,"value":observation.value(),"confidence":confidence})
}

fn number_observation(label: &str, observation: &HardwareObservation<u32>) -> Value {
    let confidence = confidence_value(observation.confidence());
    if observation.confidence() == Confidence::Unknown {
        return json!({"label":label,"value":Value::Null,"confidence":confidence});
    }
    json!({"label":label,"value":observation.value(),"confidence":confidence})
}

fn accelerator_kind_observation(observation: &HardwareObservation<AcceleratorKind>) -> Value {
    let confidence = confidence_value(observation.confidence());
    let value = match observation.value() {
        AcceleratorKind::Integrated => "integrated",
        AcceleratorKind::Discrete => "discrete",
        AcceleratorKind::UnifiedMemory => "unified_memory",
        AcceleratorKind::Unknown => {
            return json!({"label":"Accelerator type","value":Value::Null,"confidence":confidence});
        }
    };
    json!({"label":"Accelerator type","value":value,"confidence":confidence})
}

fn confidence_value(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Confirmed => "confirmed",
        Confidence::Probable => "probable",
        Confidence::Unknown => "unknown",
        Confidence::Conflicting => "conflicting",
        Confidence::Unsupported => "unsupported",
    }
}

fn warning_code(warning: &WarningCode) -> &'static str {
    match warning {
        WarningCode::DriverProbeDeferredToV2 => "driver_probe_deferred_to_v2",
        WarningCode::GpuProbeUnavailable => "gpu_probe_unavailable",
        WarningCode::LimitedMemory => "limited_memory",
        WarningCode::LowStorage => "low_storage",
        WarningCode::UnsupportedArchitecture => "unsupported_architecture",
        WarningCode::UnsupportedOperatingSystem => "unsupported_operating_system",
        WarningCode::VramProbeUnavailable => "vram_probe_unavailable",
    }
}
