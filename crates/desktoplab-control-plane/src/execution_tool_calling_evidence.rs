use desktoplab_backends::{BackendModelCapabilities, ModelCapabilityState};
use serde_json::{Value, json};

use crate::execution_tool_calling::{BackendToolProtocolClass, model_tool_protocol_class};

#[must_use]
pub(crate) fn tool_calling_evidence(
    backend_id: &str,
    model_id: Option<&str>,
    capabilities: Option<&BackendModelCapabilities>,
) -> Value {
    let model_id = model_id.unwrap_or("model.unconfigured");
    let class = model_tool_protocol_class(backend_id, capabilities);
    let endpoint = match backend_id {
        "backend.ollama" => json!("http://127.0.0.1:11434/api/chat"),
        "backend.lm-studio" => json!("http://127.0.0.1:1234/v1/chat/completions"),
        _ => Value::Null,
    };
    json!({
        "backendId":backend_id,
        "modelId":model_id,
        "endpoint":endpoint,
        "protocolClass":class.as_str(),
        "nativeToolCalls":class == BackendToolProtocolClass::NativeTool,
        "structuredOutputSupported":class == BackendToolProtocolClass::ConstrainedJson,
        "chatOnly":class == BackendToolProtocolClass::ChatOnly,
        "fullCodingAgentEligible":class.supports_full_coding_agent(),
        "canonicalExecutorPipeline":class.supports_full_coding_agent(),
        "fallbackReason":fallback_reason(backend_id, class, capabilities),
        "capabilityFingerprint":capabilities.map(BackendModelCapabilities::fingerprint),
        "contextWindow":capabilities.and_then(BackendModelCapabilities::context_window),
        "toolCapabilityState":capabilities.map_or("probe_required", |profile| profile.capability_state("tools").as_str()),
        "toolProtocolCertification":capabilities
            .and_then(BackendModelCapabilities::tool_protocol_certification)
            .map(|certification| certification.state().as_str())
            .unwrap_or("not_run"),
        "toolProtocolKind":capabilities
            .and_then(BackendModelCapabilities::tool_protocol_kind)
            .map(|protocol| protocol.as_str())
    })
}

fn fallback_reason(
    backend_id: &str,
    class: BackendToolProtocolClass,
    capabilities: Option<&BackendModelCapabilities>,
) -> Value {
    if class == BackendToolProtocolClass::ConstrainedJson && backend_id == "backend.ollama" {
        return Value::Null;
    }
    if class == BackendToolProtocolClass::ConstrainedJson {
        return json!("native_tool_calls_unavailable");
    }
    if backend_id != "backend.ollama" || class != BackendToolProtocolClass::ChatOnly {
        return Value::Null;
    }
    match capabilities.map(|profile| {
        (
            profile.capability_state("tools"),
            profile.tool_protocol_certified(),
        )
    }) {
        Some((ModelCapabilityState::Unsupported, _)) => json!("model_native_tools_unsupported"),
        Some((ModelCapabilityState::Confirmed, false)) => {
            json!("model_tool_protocol_uncertified")
        }
        _ => json!("model_tool_capability_unverified"),
    }
}
