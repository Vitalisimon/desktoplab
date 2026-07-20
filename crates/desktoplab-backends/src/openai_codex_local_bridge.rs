use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{BackendMessage, BackendPrompt, BackendToolSchema, provider_tools};

const OPENAI_CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const OPENAI_CODEX_OAUTH_AUTHORIZE_URL: &str = "https://auth.openai.com/oauth/authorize";
const OPENAI_CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
const OPENAI_CODEX_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
const OPENAI_CODEX_OAUTH_SCOPE: &str = "openid profile email offline_access";
const OPENAI_CODEX_DEVICE_USER_CODE_URL: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
const OPENAI_CODEX_DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexPkceLogin {
    authorization_url: String,
    code_challenge: String,
    code_verifier: String,
}

impl OpenAiCodexPkceLogin {
    pub fn from_verifier_source(verifier_source: &[u8], state: &str) -> Result<Self, String> {
        require_text(state, "state")?;
        if verifier_source.is_empty() {
            return Err("verifier source is required".to_string());
        }
        let code_verifier = base64_url(verifier_source);
        let code_challenge = pkce_challenge(&code_verifier);
        let authorization_url = format!(
            "{OPENAI_CODEX_OAUTH_AUTHORIZE_URL}?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
            url_encode(OPENAI_CODEX_CLIENT_ID),
            url_encode(OPENAI_CODEX_REDIRECT_URI),
            url_encode(OPENAI_CODEX_OAUTH_SCOPE),
            url_encode(state),
            url_encode(&code_challenge)
        );
        Ok(Self {
            authorization_url,
            code_challenge,
            code_verifier,
        })
    }

    #[must_use]
    pub fn authorization_url(&self) -> &str {
        &self.authorization_url
    }

    #[must_use]
    pub fn code_challenge(&self) -> &str {
        &self.code_challenge
    }

    #[must_use]
    pub fn code_verifier(&self) -> &str {
        &self.code_verifier
    }

    #[must_use]
    pub fn redirect_uri(&self) -> &'static str {
        OPENAI_CODEX_REDIRECT_URI
    }

    #[must_use]
    pub fn token_url(&self) -> &'static str {
        OPENAI_CODEX_OAUTH_TOKEN_URL
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexDeviceLoginRequest;

impl OpenAiCodexDeviceLoginRequest {
    #[must_use]
    pub fn to_json() -> Value {
        json!({
            "url":OPENAI_CODEX_DEVICE_USER_CODE_URL,
            "method":"POST",
            "headers":{"content-type":"application/json","originator":"desktoplab","user-agent":"desktoplab-local-provider-bridge"},
            "body":{"client_id":OPENAI_CODEX_CLIENT_ID},
            "verificationUrl":OPENAI_CODEX_DEVICE_VERIFICATION_URL
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexCompletionPayload {
    bridge_instance_id: String,
    local_credential_ref: String,
    pairing_code: String,
    pairing_id: String,
    provider_account_label: String,
}

impl OpenAiCodexCompletionPayload {
    pub fn new(
        bridge_instance_id: &str,
        local_credential_ref: &str,
        pairing_code: &str,
        pairing_id: &str,
        provider_account_label: &str,
    ) -> Result<Self, String> {
        require_text(bridge_instance_id, "bridgeInstanceId")?;
        require_text(local_credential_ref, "localCredentialRef")?;
        require_text(pairing_code, "pairingCode")?;
        require_text(pairing_id, "pairingId")?;
        require_text(provider_account_label, "providerAccountLabel")?;
        if !is_openai_codex_vault_ref(local_credential_ref) {
            return Err(
                "localCredentialRef must use the DesktopLab native Codex vault".to_string(),
            );
        }
        Ok(Self {
            bridge_instance_id: bridge_instance_id.to_string(),
            local_credential_ref: local_credential_ref.to_string(),
            pairing_code: pairing_code.to_string(),
            pairing_id: pairing_id.to_string(),
            provider_account_label: provider_account_label.to_string(),
        })
    }

    #[must_use]
    pub fn local_credential_ref(&self) -> &str {
        &self.local_credential_ref
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "bridgeCapabilities":["chat.completions","drafts.compose","context.read_after_consent"],
            "bridgeInstanceId":self.bridge_instance_id,
            "localCredentialRef":self.local_credential_ref,
            "pairingCode":self.pairing_code,
            "pairingId":self.pairing_id,
            "providerAccountLabel":self.provider_account_label
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexResponderCommandPayload {
    prompt: String,
    agent_request: Option<Value>,
    local_credential_ref: String,
    vault_kind: String,
}

impl OpenAiCodexResponderCommandPayload {
    pub fn new(prompt: &str, local_credential_ref: &str, vault_kind: &str) -> Result<Self, String> {
        require_text(prompt, "prompt")?;
        require_text(local_credential_ref, "localCredentialRef")?;
        require_text(vault_kind, "vaultKind")?;
        if !is_openai_codex_vault_ref(local_credential_ref) {
            return Err(
                "localCredentialRef must use the DesktopLab native Codex vault".to_string(),
            );
        }
        Ok(Self {
            prompt: prompt.to_string(),
            agent_request: None,
            local_credential_ref: local_credential_ref.to_string(),
            vault_kind: vault_kind.to_string(),
        })
    }

    pub fn for_agent_turn(
        messages: Vec<BackendMessage>,
        tools: Vec<BackendToolSchema>,
        local_credential_ref: &str,
        vault_kind: &str,
    ) -> Result<Self, String> {
        if messages.is_empty() || tools.is_empty() {
            return Err("Codex agent turns require messages and canonical tools".to_string());
        }
        let backend_prompt = BackendPrompt::new("openai-codex", "")
            .with_messages(messages)
            .with_tools(tools);
        let agent_request = json!({
            "protocol":"desktoplab.canonical-tools.v1",
            "messages":backend_prompt.openai_messages(),
            "tools":provider_tools(backend_prompt.tools()),
            "toolChoice":"required"
        });
        let encoded = serde_json::to_string(&agent_request)
            .map_err(|_| "Codex canonical agent request could not be encoded".to_string())?;
        let prompt = format!(
            "Execute this DesktopLab agent turn. Return exactly one canonical tool call from the supplied registry and no prose outside it.\n{encoded}"
        );
        let mut payload = Self::new(&prompt, local_credential_ref, vault_kind)?;
        payload.agent_request = Some(agent_request);
        Ok(payload)
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut payload = json!({
            "connection":{
                "connectedVia":"local_provider_bridge",
                "entitlementMode":"subscription_account",
                "providerCallMode":"local_provider_bridge",
                "providerCredentialRef":self.local_credential_ref,
                "providerId":"openai",
                "requestedScopes":["repository.context","agent.response"],
                "tokenStorage":"vault_ref_only",
                "vaultKind":self.vault_kind
            },
            "prompt":self.prompt,
            "requestedScopes":["repository.context","agent.response"],
            "targetAccountIds":["desktoplab-local-workspace"],
            "targetDisplayNames":["DesktopLab workspace"]
        });
        if let Some(agent_request) = &self.agent_request {
            payload["agentRequest"] = agent_request.clone();
        }
        payload
    }
}

fn is_openai_codex_vault_ref(value: &str) -> bool {
    value.starts_with("vault://desktoplab/external-backend/openai-codex/")
}

#[must_use]
pub fn is_loopback_codex_responder_url(url: &str) -> bool {
    let candidate = url.trim();
    candidate.starts_with("http://127.0.0.1:")
        || candidate.starts_with("http://localhost:")
        || candidate.starts_with("http://[::1]:")
}

fn require_text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(())
    }
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64_url(&digest)
}

fn base64_url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;
        output.push(ALPHABET[((triple >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((triple >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((triple >> 6) & 0x3f) as usize] as char);
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(triple & 0x3f) as usize] as char);
        }
    }
    output
}

fn url_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "%20".to_string(),
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
