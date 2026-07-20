#![forbid(unsafe_code)]

mod backend_prompt;
mod bridge_readiness;
mod claude_bridge;
mod codex_bridge;
mod external_harness;
mod lm_studio_execution;
mod model_capabilities;
mod model_protocol_certification;
mod ollama_capabilities;
mod ollama_execution;
mod ollama_protocol_canary;
mod ollama_stream;
mod openai_codex_device_auth;
mod openai_codex_device_http;
mod openai_codex_local_bridge;
mod openai_compatible_local;
mod openai_compatible_stream;
mod productization;
mod provider_compatibility;
mod tool_calling;
mod tool_response_bridge;

pub use backend_prompt::{BackendMessage, BackendPrompt};
pub use bridge_readiness::{
    BridgeFailureCode, BridgeReadiness, BridgeReadinessProbe, BridgeReadinessService, BridgeStatus,
};
pub use claude_bridge::{ClaudeAgentSdkBridge, ClaudeBridgeConfig};
pub use codex_bridge::{CodexAppServerBridge, CodexBridgeConfig};
pub use external_harness::{ExternalBackendHarness, ExternalBackendManifest, ExternalEvent};
pub use lm_studio_execution::{LmStudioExecutionBackend, LocalEndpoint};
pub use model_capabilities::{BackendModelCapabilities, ModelCapabilityState};
pub use model_protocol_certification::{
    ModelProtocolCertificationState, ModelToolProtocolCertification, ModelToolProtocolKind,
};
pub use ollama_capabilities::OllamaModelCapabilityResolver;
pub use ollama_execution::{BackendExecutionResult, BackendModelInventory, OllamaExecutionBackend};
pub use ollama_protocol_canary::OllamaToolProtocolCanary;
pub use openai_codex_device_auth::{
    OpenAiCodexDeviceAuthorization, OpenAiCodexDeviceAuthorizationPollRequest,
    OpenAiCodexDeviceCodeRequest, OpenAiCodexDevicePollOutcome,
    OpenAiCodexDeviceTokenExchangeRequest, OpenAiCodexResponderCommandOutput,
};
pub use openai_codex_device_http::{
    exchange_openai_codex_device_token, execute_openai_codex_responder_command,
    poll_openai_codex_device_authorization, request_openai_codex_device_authorization,
};
pub use openai_codex_local_bridge::{
    OpenAiCodexCompletionPayload, OpenAiCodexDeviceLoginRequest, OpenAiCodexPkceLogin,
    OpenAiCodexResponderCommandPayload, is_loopback_codex_responder_url,
};
pub use openai_compatible_local::OpenAiCompatibleLocalExecutionBackend;
pub use productization::{BridgeCallFailure, ImportedBridgeEvents};
pub use provider_compatibility::{ProviderCompatibilityProfile, ProviderMessageEnvelope};
pub use tool_calling::{
    BackendToolCall, BackendToolCallEvidence, BackendToolResponse, BackendToolSchema,
    parse_constrained_tool_text, parse_ollama_tool_response, parse_openai_compatible_tool_response,
    provider_tools,
};
pub use tool_response_bridge::backend_response_to_agent_text;
