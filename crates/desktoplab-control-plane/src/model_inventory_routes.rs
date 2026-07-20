use desktoplab_hardware_wizard::HardwareProbeAdapter;
use desktoplab_model_manager::{ModelManager, ModelParameterClass, ModelVariant};
use serde_json::{Value, json};

#[must_use]
pub(crate) fn models_response_with_state(
    runtime_verified: bool,
    verified_runtime_id: Option<&str>,
    verified_model_id: Option<&str>,
    installed_models: &[String],
    host_memory_gb: Option<u32>,
) -> Value {
    let catalog = ModelManager::new().default_family_catalog();
    let variants = catalog.variants();
    let memory_gb = host_memory_gb.unwrap_or_else(effective_memory_gb);
    let models = variants
        .iter()
        .map(|variant| {
            model_response(
                variant,
                runtime_verified
                    && verified_runtime_id == Some(variant.runtime_compatibility().runtime_id()),
                verified_model_id,
                installed_models,
                memory_gb,
            )
        })
        .collect::<Vec<_>>();
    json!({"source":"service_backed","models":models})
}

fn model_response(
    variant: &ModelVariant,
    runtime_verified: bool,
    verified_model_id: Option<&str>,
    installed_models: &[String],
    memory_gb: u32,
) -> Value {
    let runtime = variant.runtime_compatibility();
    let runtime_supported = runtime_supported_on_host(runtime.runtime_id());
    let runtime_cloud = runtime.runtime_id() == "runtime.ollama-cloud";
    let installed = installed_models
        .iter()
        .any(|model| inventory_entry_has_model(model, runtime.pull_ref()));
    let verified = verified_model_id == Some(variant.model_id());
    let memory_supported = memory_gb >= variant.required_memory_gb();
    let (install_state, compatibility, verification, blocked_reason) = model_state_copy(
        runtime.runtime_id(),
        runtime_cloud,
        runtime_supported,
        runtime_verified,
        installed || verified,
        memory_supported,
        variant.required_memory_gb(),
        memory_gb,
    );
    json!({
        "modelId":variant.model_id(),
        "displayName":display_name(variant),
        "familyId":variant.family_id(),
        "familyName":variant.family_name(),
        "runtimeId":runtime.runtime_id(),
        "pullRef":runtime.pull_ref(),
        "channel":variant.channel(),
        "parameterClass":parameter_class(variant.parameter_class()),
        "parametersBillion":variant.parameters_billion(),
        "quantization":variant.quantization(),
        "requiredMemoryGb":variant.required_memory_gb(),
        "installState":install_state,
        "compatibility":compatibility,
        "backendSelectable":install_state == "installed" && compatibility == "ready",
        "sizeGb":variant.expected_disk_mb().div_ceil(1024),
        "recommended":false,
        "agentQualification":"runtime_validation_required",
        "verification":verification,
        "provenance":{
            "catalogSource":"bundled_seed_catalog",
            "runtimeId":runtime.runtime_id(),
            "pullRef":runtime.pull_ref(),
            "verificationState":verification_state(runtime_verified, installed || verified),
            "localVerification":verification
        },
        "blockedReason":blocked_reason
    })
}

fn verification_state(runtime_verified: bool, installed_or_verified: bool) -> &'static str {
    if installed_or_verified {
        "verified_local_inventory"
    } else if runtime_verified {
        "downloadable_not_installed"
    } else {
        "runtime_verification_required"
    }
}

fn model_state_copy(
    runtime_id: &str,
    runtime_cloud: bool,
    runtime_supported: bool,
    runtime_verified: bool,
    installed_or_verified: bool,
    memory_supported: bool,
    required_memory_gb: u32,
    available_memory_gb: u32,
) -> (&'static str, &'static str, &'static str, Value) {
    if runtime_cloud {
        return (
            "cloud_available",
            "cloud",
            "Requires Ollama cloud account",
            Value::Null,
        );
    }
    if !memory_supported {
        return (
            "blocked",
            "blocked",
            "hardware block",
            json!(format!(
                "Requires {required_memory_gb} GB memory class; this computer reports {available_memory_gb} GB."
            )),
        );
    }
    if !runtime_supported {
        return (
            "blocked",
            "blocked",
            "runtime unavailable",
            json!("Runtime unavailable"),
        );
    }
    if installed_or_verified {
        return (
            "installed",
            "ready",
            runtime_found_copy(runtime_id),
            Value::Null,
        );
    }
    if runtime_verified {
        return (
            "downloadable",
            "compatible",
            "Ready to download through selected local runtime",
            Value::Null,
        );
    }
    (
        "blocked",
        "compatible",
        "runtime inventory required",
        json!("runtime_not_verified"),
    )
}

fn runtime_found_copy(runtime_id: &str) -> &'static str {
    match runtime_id {
        "runtime.mlx-lm" => "Found in MLX-LM",
        _ => "Found in Ollama",
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

fn runtime_supported_on_host(runtime_id: &str) -> bool {
    runtime_id == "runtime.ollama"
        || (runtime_id == "runtime.mlx-lm"
            && cfg!(all(target_os = "macos", target_arch = "aarch64")))
}

fn effective_memory_gb() -> u32 {
    let profile = HardwareProbeAdapter::for_current_host().profile();
    profile
        .ram_gb()
        .value()
        .max(profile.vram_gb().value())
        .max(profile.unified_memory_gb().value())
}

fn display_name(variant: &ModelVariant) -> String {
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
