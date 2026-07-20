use desktoplab_runtime::{
    HighEndRuntimeHealthState, HighEndRuntimeLifecycle, HighEndRuntimeOwnership,
};
use serde_json::{Value, json};

#[must_use]
pub(crate) fn health_response(runtime: Option<&HighEndRuntimeLifecycle>) -> Value {
    let Some(runtime) = runtime else {
        return json!({
            "source":"runtime_probe",
            "state":"unconfigured",
            "routeEligibility":"blocked"
        });
    };
    let evidence = runtime.evidence();
    let state = evidence.state();
    json!({
        "source":"runtime_probe",
        "runtimeId":runtime.contract().runtime_id().as_str(),
        "runtimeFamily":runtime.contract().family().as_str(),
        "endpoint":runtime.endpoint().base_url(),
        "modelId":runtime.endpoint().model_id(),
        "ownership":runtime.ownership().as_str(),
        "canStop":runtime.ownership() == HighEndRuntimeOwnership::DesktopLabOwned,
        "state":state.as_str(),
        "routeEligibility":if state == HighEndRuntimeHealthState::ModelReady {"verification_required"} else {"blocked"},
        "agentProtocolState":"not_certified",
        "evidence":{
            "endpointCompatible":evidence.endpoint_compatible(),
            "modelLoaded":evidence.model_loaded(),
            "tokenizerReady":evidence.tokenizer_ready(),
            "gpuMemoryPressurePercent":evidence.gpu_memory_pressure_percent(),
            "queueDepth":evidence.queue_depth(),
            "reason":evidence.reason()
        }
    })
}

pub(crate) fn is_high_end_runtime_id(runtime_id: &str) -> bool {
    matches!(
        runtime_id,
        "runtime.nim"
            | "runtime.tensorrt-llm"
            | "runtime.vllm"
            | "runtime.llama-cpp-server"
            | "runtime.openai-compatible-local"
            | "runtime.custom-lan"
    )
}
