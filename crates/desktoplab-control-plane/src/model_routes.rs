use desktoplab_hardware_wizard::HardwareProbeAdapter;
use desktoplab_model_manager::{
    AgentContextWindowPolicy, AgentRequestTimeoutPolicy, ModelDownloadCapacity,
    ModelDownloadExecutionPolicy, ModelDownloadExecutor, ModelDownloadPlan, ModelManager,
    ModelVariant, ModelVerification,
};
use serde_json::{Value, json};

pub use crate::model_route_errors::ModelRouteError;

#[must_use]
pub fn models_response_with_state(
    runtime_verified: bool,
    verified_runtime_id: Option<&str>,
    verified_model_id: Option<&str>,
    installed_models: &[String],
    host_memory_gb: Option<u32>,
) -> Value {
    crate::model_inventory_routes::models_response_with_state(
        runtime_verified,
        verified_runtime_id,
        verified_model_id,
        installed_models,
        host_memory_gb,
    )
}

pub fn plan_model_download(
    path: &str,
    body: &str,
    host_memory_gb: Option<u32>,
) -> Result<ModelDownloadStart, ModelRouteError> {
    let model_id = segment(path, 2);
    let variant = find_variant(&model_id)?;
    if variant.runtime_compatibility().runtime_id() == "runtime.ollama-cloud" {
        return Err(ModelRouteError::CloudProviderRequired);
    }
    let memory_gb = host_memory_gb.unwrap_or_else(effective_memory_gb);
    if memory_gb < variant.required_memory_gb() {
        return Err(ModelRouteError::InsufficientMemory {
            required_gb: variant.required_memory_gb(),
            available_gb: memory_gb,
        });
    }
    let setup_choice = setup_choice_from_body(body)?;
    let capacity = ModelDownloadCapacity::new(number_body_field(body, "diskAvailableMb"))
        .with_network_available(bool_body_field(body, "networkAvailable").unwrap_or(true));
    let plan = ModelDownloadPlan::from_variant(
        &variant,
        bool_body_field(body, "setupAccepted").unwrap_or(true),
    );
    let pull_ref_override = string_body_field(body, "pullRef");
    if let Some(pull_ref) = pull_ref_override.as_deref() {
        crate::model_pull_ref_validation::validate(plan.runtime_id().as_str(), pull_ref)?;
    }
    let pull_ref = plan.pull_ref().to_string();
    let runtime_id = plan.runtime_id().as_str().to_string();
    let executor = ModelDownloadExecutor::new(capacity);
    executor
        .start(plan, ModelDownloadExecutionPolicy::resumable())
        .map(|_job| ModelDownloadStart {
            model_id,
            #[cfg(debug_assertions)]
            family_id: variant.family_id().to_string(),
            #[cfg(debug_assertions)]
            variant_id: variant.model_id().to_string(),
            runtime_id,
            pull_ref,
            #[cfg(debug_assertions)]
            command_evidence: _job.command().evidence(),
            setup_choice,
        })
        .map_err(ModelRouteError::Download)
}

#[must_use]
#[cfg(debug_assertions)]
pub fn model_download_response(start: &ModelDownloadStart, job_id: &str) -> Value {
    json!({
        "source":"service_backed",
        "jobId":job_id,
        "modelId":start.model_id,
        "familyId":start.family_id,
        "variantId":start.variant_id,
        "runtimeId":start.runtime_id,
        "state":"running",
        "progressPercent":5,
        "retryClass":"retryable",
        "failureReason":Value::Null,
        "setupChoice":start.setup_choice.as_str(),
        "executionEvidence":start.command_evidence
    })
}

#[must_use]
pub fn model_download_runtime_blocked_response(
    path: &str,
    job_id: &str,
    runtime_id: &str,
) -> Value {
    json!({
        "source":"service_backed",
        "jobId":job_id,
        "modelId":segment(path, 2),
        "runtimeId":runtime_id,
        "state":"blocked",
        "progressPercent":0,
        "retryClass":"non_retryable",
        "blockedReason":"runtime_not_verified",
        "executionEvidence":"runtime readiness is required before model pull"
    })
}

#[must_use]
pub fn model_download_blocked_response(path: &str, error: ModelRouteError) -> Value {
    let mut response = json!({
        "source":"service_backed",
        "jobId":Value::Null,
        "modelId":segment(path, 2),
        "runtimeId":error.runtime_id(),
        "state":"blocked",
        "retryClass":error.retry_class(),
        "blockedReason":error.reason()
    });
    if let Some((required_mb, available_mb)) = error.disk_details()
        && let Some(object) = response.as_object_mut()
    {
        object.insert("requiredDiskMb".to_string(), json!(required_mb));
        object.insert("availableDiskMb".to_string(), json!(available_mb));
    }
    if let Some((required_gb, available_gb)) = error.memory_details()
        && let Some(object) = response.as_object_mut()
    {
        object.insert("requiredMemoryGb".to_string(), json!(required_gb));
        object.insert("availableMemoryGb".to_string(), json!(available_gb));
    }
    response
}

pub struct ModelDownloadStart {
    model_id: String,
    #[cfg(debug_assertions)]
    family_id: String,
    #[cfg(debug_assertions)]
    variant_id: String,
    runtime_id: String,
    pull_ref: String,
    #[cfg(debug_assertions)]
    command_evidence: String,
    setup_choice: SetupChoice,
}

impl ModelDownloadStart {
    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn pull_ref(&self) -> &str {
        &self.pull_ref
    }

    #[must_use]
    pub fn should_use_existing(&self) -> bool {
        self.setup_choice == SetupChoice::UseExisting
    }

    #[must_use]
    pub fn setup_choice(&self) -> &'static str {
        self.setup_choice.as_str()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupChoice {
    UseExisting,
    Replace,
}

impl SetupChoice {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UseExisting => "use_existing",
            Self::Replace => "replace",
        }
    }
}

fn setup_choice_from_body(body: &str) -> Result<SetupChoice, ModelRouteError> {
    match string_body_field(body, "setupChoice").as_deref() {
        None | Some("") | Some("use_existing") | Some("install") => Ok(SetupChoice::UseExisting),
        Some("replace") => Ok(SetupChoice::Replace),
        Some(_) => Err(ModelRouteError::UnknownSetupChoice),
    }
}

#[must_use]
pub fn model_pull_ref(model_id: &str) -> Option<String> {
    find_variant(model_id).ok().map(|variant| {
        ModelDownloadPlan::from_variant(&variant, true)
            .pull_ref()
            .to_string()
    })
}

#[must_use]
pub fn model_runtime_id(model_id: &str) -> Option<String> {
    find_variant(model_id)
        .ok()
        .map(|variant| variant.runtime_compatibility().runtime_id().to_string())
}

#[must_use]
pub fn agent_context_window_tokens(model_id: &str, available_memory_gb: u32) -> Option<u32> {
    let variant = find_variant(model_id).ok()?;
    Some(AgentContextWindowPolicy::for_variant(
        &variant,
        available_memory_gb,
    ))
}

#[must_use]
pub fn agent_request_timeout_seconds(model_id: &str, available_memory_gb: u32) -> Option<u64> {
    let variant = find_variant(model_id).ok()?;
    Some(AgentRequestTimeoutPolicy::for_variant(
        &variant,
        available_memory_gb,
    ))
}

#[must_use]
pub fn agent_request_timeout_seconds_for_pull_ref(
    pull_ref: &str,
    available_memory_gb: u32,
) -> Option<u64> {
    let catalog = ModelManager::new().default_family_catalog();
    let variant = catalog
        .variants()
        .iter()
        .find(|variant| ModelDownloadPlan::from_variant(variant, true).pull_ref() == pull_ref)?;
    Some(AgentRequestTimeoutPolicy::for_variant(
        variant,
        available_memory_gb,
    ))
}

#[must_use]
pub fn verify_model_inventory(pull_ref: &str, inventory_output: &str) -> ModelVerification {
    ModelVerification::from_runtime_inventory(pull_ref, inventory_output)
}

fn find_variant(model_id: &str) -> Result<ModelVariant, ModelRouteError> {
    ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .find(|variant| variant.model_id() == model_id)
        .cloned()
        .ok_or(ModelRouteError::UnknownModel)
}

fn bool_body_field(body: &str, field: &str) -> Option<bool> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_bool()
}

fn number_body_field(body: &str, field: &str) -> u64 {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|body| body.get(field).and_then(Value::as_u64))
        .unwrap_or(64_000)
}

pub(crate) fn effective_memory_gb() -> u32 {
    let profile = HardwareProbeAdapter::for_current_host().profile();
    profile
        .ram_gb()
        .value()
        .max(profile.vram_gb().value())
        .max(profile.unified_memory_gb().value())
}

fn string_body_field(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(ToString::to_string)
}

fn segment(path: &str, index: usize) -> String {
    path.split('/')
        .nth(index + 1)
        .unwrap_or_default()
        .to_string()
}
