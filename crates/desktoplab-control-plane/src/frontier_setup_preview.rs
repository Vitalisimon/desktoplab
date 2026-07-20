use desktoplab_hardware_wizard::{
    FrontierFeatureState, FrontierHardwareClass, FrontierHardwareClassifier, FrontierHostFacts,
    FrontierHostProbeAdapter, StdFrontierProbeSource,
};
use serde_json::{Value, json};
use std::time::{SystemTime, UNIX_EPOCH};

#[must_use]
pub(crate) fn frontier_setup_preview() -> Value {
    let now = now_epoch_seconds();
    let (facts, source) = setup_facts(now);
    let assessment = FrontierHardwareClassifier::new(300).classify(&facts, now);
    let candidate = assessment.is_high_end_candidate();
    let runtime_id = recommended_runtime(assessment.class());
    let effective_memory = facts
        .coherent_memory_gb()
        .unwrap_or_default()
        .max(facts.accelerator_memory_gb().unwrap_or_default());
    json!({
        "status":if candidate {"candidate"} else {"standard"},
        "source":source,
        "profile":assessment.class().as_str(),
        "profileLabel":profile_label(assessment.class()),
        "hardwareSummary":format!("{} accelerator{} · {} GB effective memory", facts.accelerator_count(), if facts.accelerator_count() == 1 {""} else {"s"}, effective_memory),
        "recommendedRuntimeId":runtime_id,
        "runtimeChoices":runtime_choices(runtime_id),
        "storageTarget":{
            "path":model_store_path(),
            "displayPath":"~/.desktoplab/models",
            "freeGb":facts.storage_available_gb()
        },
        "expectedCapability":if candidate {"High-capacity local coding route; live certification is still required."} else {"Standard local setup is the supported route for the hardware detected here."},
        "claimState":"certification_required",
        "blockingReasons":assessment.blockers(),
        "details":{
            "gpuModels":facts.gpu_models(),
            "driver":facts.cuda_driver_version(),
            "cuda":facts.cuda_runtime_version(),
            "nvlink":feature_state(facts.nvlink()),
            "nvswitch":feature_state(facts.nvswitch()),
            "mig":feature_state(facts.mig())
        }
    })
}

fn setup_facts(now: u64) -> (FrontierHostFacts, &'static str) {
    if std::env::var("DESKTOPLAB_TEST_CONTROLS").as_deref() == Ok("1")
        && std::env::var("DESKTOPLAB_FRONTIER_SETUP_TEST_PROFILE").as_deref()
            == Ok("dgx_station_class")
    {
        return (
            FrontierHostFacts::detected(now)
                .with_accelerators(&["GB300"], 748)
                .with_memory(748, Some(748))
                .with_storage_available_gb(2_048)
                .with_cuda("test-driver", "test-cuda")
                .with_topology(
                    FrontierFeatureState::Detected,
                    FrontierFeatureState::Detected,
                    FrontierFeatureState::Detected,
                ),
            "dev_test_control",
        );
    }
    (
        FrontierHostProbeAdapter::new(StdFrontierProbeSource).probe(),
        "hardware_probe",
    )
}

fn runtime_choices(recommended_runtime_id: &str) -> Vec<Value> {
    let test_endpoint = test_runtime_endpoint();
    [
        (
            "runtime.nim",
            "NVIDIA NIM",
            test_endpoint.as_deref().unwrap_or("http://127.0.0.1:8000"),
        ),
        (
            "runtime.vllm",
            "vLLM",
            test_endpoint.as_deref().unwrap_or("http://127.0.0.1:8000"),
        ),
        (
            "runtime.tensorrt-llm",
            "TensorRT-LLM",
            test_endpoint.as_deref().unwrap_or("http://127.0.0.1:8000"),
        ),
        (
            "runtime.llama-cpp-server",
            "llama.cpp server",
            test_endpoint.as_deref().unwrap_or("http://127.0.0.1:8080"),
        ),
    ]
    .into_iter()
    .map(|(runtime_id, display_name, default_endpoint)| {
        json!({
            "runtimeId":runtime_id,
            "displayName":display_name,
            "defaultEndpoint":default_endpoint,
            "recommended":runtime_id == recommended_runtime_id
        })
    })
    .collect()
}

fn test_runtime_endpoint() -> Option<String> {
    (std::env::var("DESKTOPLAB_TEST_CONTROLS").as_deref() == Ok("1"))
        .then(|| std::env::var("DESKTOPLAB_FRONTIER_RUNTIME_TEST_ENDPOINT").ok())
        .flatten()
}

fn recommended_runtime(class: FrontierHardwareClass) -> &'static str {
    match class {
        FrontierHardwareClass::DgxStationClass => "runtime.nim",
        FrontierHardwareClass::CustomFrontierRig => "runtime.vllm",
        FrontierHardwareClass::DgxSparkClass => "runtime.llama-cpp-server",
        _ => "runtime.vllm",
    }
}

fn profile_label(class: FrontierHardwareClass) -> &'static str {
    match class {
        FrontierHardwareClass::DgxStationClass => "High-memory AI workstation",
        FrontierHardwareClass::CustomFrontierRig => "Multi-accelerator workstation",
        FrontierHardwareClass::DgxSparkClass => "Compact AI workstation",
        FrontierHardwareClass::WorkstationLocal => "Local workstation",
        FrontierHardwareClass::Unclassified => "Standard local computer",
    }
}

fn feature_state(state: FrontierFeatureState) -> &'static str {
    match state {
        FrontierFeatureState::Detected => "detected",
        FrontierFeatureState::NotDetected => "not_detected",
        FrontierFeatureState::Unknown => "unknown",
    }
}

fn model_store_path() -> String {
    std::env::var("HOME")
        .map(|home| format!("{home}/.desktoplab/models"))
        .unwrap_or_else(|_| ".desktoplab/models".to_string())
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
