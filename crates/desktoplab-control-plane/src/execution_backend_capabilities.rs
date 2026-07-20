#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct BackendCapabilityProfile {
    backend_id: &'static str,
    required: &'static [&'static str],
    advertised: &'static [&'static str],
}

impl BackendCapabilityProfile {
    #[must_use]
    pub(crate) const fn backend_id(self) -> &'static str {
        self.backend_id
    }

    #[must_use]
    pub(crate) const fn required(self) -> &'static [&'static str] {
        self.required
    }

    #[must_use]
    pub(crate) const fn advertised(self) -> &'static [&'static str] {
        self.advertised
    }
}

const LOCAL_REQUIRED: &[&str] = &["llm.chat", "tools.filesystem.read"];
const LOCAL_FALLBACK_ADVERTISED: &[&str] = &[
    "llm.chat",
    "tools.filesystem.read",
    "tools.filesystem.write.approval",
    "terminal.command.approval",
    "agent.protocol.strict_json_actions",
];
const OLLAMA_ADVERTISED: &[&str] = &[
    "llm.chat",
    "tools.filesystem.read",
    "tools.filesystem.write.approval",
    "terminal.command.approval",
    "agent.protocol.native_tool_calls",
    "agent.events.stream",
];
const LOCAL_NATIVE_ADVERTISED: &[&str] = &[
    "llm.chat",
    "tools.filesystem.read",
    "tools.filesystem.write.approval",
    "terminal.command.approval",
    "agent.protocol.native_tool_calls",
];
const CODEX_REQUIRED: &[&str] = &["llm.chat", "agent.events.stream"];
const CODEX_ADVERTISED: &[&str] = &[
    "llm.chat",
    "agent.events.stream",
    "external.egress.requires_approval",
];

#[must_use]
pub(crate) fn backend_capability_profile(backend_id: &str) -> BackendCapabilityProfile {
    match backend_id {
        "backend.codex" => BackendCapabilityProfile {
            backend_id: "backend.codex",
            required: CODEX_REQUIRED,
            advertised: CODEX_ADVERTISED,
        },
        "backend.mlx-lm" => BackendCapabilityProfile {
            backend_id: "backend.mlx-lm",
            required: LOCAL_REQUIRED,
            advertised: LOCAL_FALLBACK_ADVERTISED,
        },
        "backend.lm-studio" => BackendCapabilityProfile {
            backend_id: "backend.lm-studio",
            required: LOCAL_REQUIRED,
            advertised: LOCAL_NATIVE_ADVERTISED,
        },
        "backend.high-end-local" => BackendCapabilityProfile {
            backend_id: "backend.high-end-local",
            required: LOCAL_REQUIRED,
            advertised: LOCAL_NATIVE_ADVERTISED,
        },
        _ => BackendCapabilityProfile {
            backend_id: "backend.ollama",
            required: LOCAL_REQUIRED,
            advertised: OLLAMA_ADVERTISED,
        },
    }
}

#[must_use]
pub(crate) fn backend_support_contract(backend_id: &str) -> Value {
    let model_loop_owner = match backend_id {
        "backend.codex" => "external_backend",
        "backend.lm-studio" | "backend.mlx-lm" | "backend.high-end-local" => "local_runtime",
        _ => "desktoplab",
    };
    let transcript_mirror = if backend_id == "backend.codex" {
        "mirrored_from_external_events"
    } else {
        "canonical"
    };
    let unsupported_surfaces = if backend_id == "backend.codex" {
        vec![
            "provider_owned_session",
            "automatic_repository_egress",
            "unapproved_tool_execution",
            "raw_token_ingress",
        ]
    } else {
        vec![
            "provider_owned_session",
            "automatic_repository_egress",
            "raw_token_ingress",
        ]
    };
    json!({
        "backendId":backend_capability_profile(backend_id).backend_id(),
        "supportState":"contract_ready",
        "modelLoopOwner":model_loop_owner,
        "canonicalThreadOwner":"desktoplab",
        "toolOwner":"desktoplab",
        "approvalOwner":"desktoplab",
        "contextOwner":"desktoplab",
        "compactionOwner":"desktoplab",
        "transcriptMirror":transcript_mirror,
        "unsupportedSurfaces":unsupported_surfaces
    })
}
use serde_json::{Value, json};
