use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub(crate) const OPENAI_CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub(crate) const OPENAI_CODEX_OAUTH_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub(crate) const OPENAI_CODEX_DEVICE_USER_CODE_URL: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
pub(crate) const OPENAI_CODEX_DEVICE_TOKEN_URL: &str =
    "https://auth.openai.com/api/accounts/deviceauth/token";
pub(crate) const OPENAI_CODEX_DEVICE_VERIFICATION_URL: &str =
    "https://auth.openai.com/codex/device";
pub(crate) const OPENAI_CODEX_DEVICE_CALLBACK_URL: &str =
    "https://auth.openai.com/deviceauth/callback";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexDeviceCodeRequest;

impl OpenAiCodexDeviceCodeRequest {
    #[must_use]
    pub fn to_json() -> Value {
        json!({
            "url":OPENAI_CODEX_DEVICE_USER_CODE_URL,
            "method":"POST",
            "headers":openai_codex_headers("application/json"),
            "body":{"client_id":OPENAI_CODEX_CLIENT_ID},
            "verificationUrl":OPENAI_CODEX_DEVICE_VERIFICATION_URL
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexDeviceAuthorization {
    device_auth_id: String,
    user_code: String,
    interval_seconds: u64,
}

impl OpenAiCodexDeviceAuthorization {
    pub fn new(
        device_auth_id: &str,
        user_code: &str,
        interval_seconds: u64,
    ) -> Result<Self, String> {
        require_text(device_auth_id, "deviceAuthId")?;
        require_text(user_code, "userCode")?;
        Ok(Self {
            device_auth_id: device_auth_id.to_string(),
            user_code: user_code.to_string(),
            interval_seconds: interval_seconds.max(1),
        })
    }

    #[must_use]
    pub fn device_auth_id(&self) -> &str {
        &self.device_auth_id
    }

    #[must_use]
    pub fn user_code(&self) -> &str {
        &self.user_code
    }

    #[must_use]
    pub fn interval_seconds(&self) -> u64 {
        self.interval_seconds
    }

    #[must_use]
    pub fn verification_url(&self) -> &'static str {
        OPENAI_CODEX_DEVICE_VERIFICATION_URL
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "deviceAuthId":self.device_auth_id,
            "userCode":self.user_code,
            "verificationUrl":OPENAI_CODEX_DEVICE_VERIFICATION_URL,
            "intervalSeconds":self.interval_seconds
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OpenAiCodexDevicePollOutcome {
    Pending(String),
    Authorized {
        authorization_code: String,
        code_verifier: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexDeviceAuthorizationPollRequest {
    device_auth_id: String,
    user_code: String,
}

impl OpenAiCodexDeviceAuthorizationPollRequest {
    pub fn new(device_auth_id: &str, user_code: &str) -> Result<Self, String> {
        require_text(device_auth_id, "deviceAuthId")?;
        require_text(user_code, "userCode")?;
        Ok(Self {
            device_auth_id: device_auth_id.to_string(),
            user_code: user_code.to_string(),
        })
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "url":OPENAI_CODEX_DEVICE_TOKEN_URL,
            "method":"POST",
            "headers":openai_codex_headers("application/json"),
            "body":{
                "device_auth_id":self.device_auth_id,
                "user_code":self.user_code,
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexDeviceTokenExchangeRequest {
    authorization_code: String,
    code_verifier: String,
}

impl OpenAiCodexDeviceTokenExchangeRequest {
    pub fn new(authorization_code: &str, code_verifier: &str) -> Result<Self, String> {
        require_text(authorization_code, "authorizationCode")?;
        require_text(code_verifier, "codeVerifier")?;
        Ok(Self {
            authorization_code: authorization_code.to_string(),
            code_verifier: code_verifier.to_string(),
        })
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "url":OPENAI_CODEX_OAUTH_TOKEN_URL,
            "method":"POST",
            "headers":openai_codex_headers("application/x-www-form-urlencoded"),
            "body":{
                "client_id":OPENAI_CODEX_CLIENT_ID,
                "code":self.authorization_code,
                "code_verifier":self.code_verifier,
                "grant_type":"authorization_code",
                "redirect_uri":OPENAI_CODEX_DEVICE_CALLBACK_URL,
            }
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OpenAiCodexResponderCommandOutput {
    body: String,
    provider_response_id: String,
}

impl OpenAiCodexResponderCommandOutput {
    pub fn parse(raw_output: &str) -> Result<Self, String> {
        let parsed: Value = serde_json::from_str(raw_output)
            .map_err(|error| format!("OpenAI Codex responder output must be JSON: {error}"))?;
        let Some(object) = parsed.as_object() else {
            return Err("OpenAI Codex responder output must be a JSON object".to_string());
        };
        let body = object
            .get("body")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "OpenAI Codex responder output body is required".to_string())?;
        let provider_response_id = object
            .get("providerResponseId")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map_or_else(|| response_id_for_body(body), ToString::to_string);
        Ok(Self {
            body: body.to_string(),
            provider_response_id,
        })
    }

    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    #[must_use]
    pub fn provider_response_id(&self) -> &str {
        &self.provider_response_id
    }
}

fn openai_codex_headers(content_type: &str) -> Value {
    json!({
        "content-type":content_type,
        "originator":"desktoplab",
        "user-agent":"desktoplab-local-provider-bridge"
    })
}

fn response_id_for_body(body: &str) -> String {
    let digest = Sha256::digest(body.as_bytes());
    let mut hex = String::with_capacity(24);
    for byte in digest.iter().take(12) {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("openai_codex_response_{hex}")
}

fn require_text(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} is required"))
    } else {
        Ok(())
    }
}
