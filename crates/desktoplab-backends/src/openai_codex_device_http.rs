use serde_json::{Value, json};

use crate::openai_codex_device_auth::{
    OPENAI_CODEX_CLIENT_ID, OPENAI_CODEX_DEVICE_USER_CODE_URL, OPENAI_CODEX_OAUTH_TOKEN_URL,
    OpenAiCodexDeviceAuthorization, OpenAiCodexDeviceAuthorizationPollRequest,
    OpenAiCodexDevicePollOutcome, OpenAiCodexDeviceTokenExchangeRequest,
    OpenAiCodexResponderCommandOutput,
};
use crate::{OpenAiCodexResponderCommandPayload, is_loopback_codex_responder_url};

pub fn request_openai_codex_device_authorization() -> Result<OpenAiCodexDeviceAuthorization, String>
{
    let response = reqwest::blocking::Client::new()
        .post(OPENAI_CODEX_DEVICE_USER_CODE_URL)
        .headers(reqwest_headers("application/json")?)
        .json(&json!({"client_id":OPENAI_CODEX_CLIENT_ID}))
        .send()
        .map_err(|error| format!("OpenAI Codex device authorization failed: {error}"))?;
    parse_device_authorization_response(response)
}

pub fn poll_openai_codex_device_authorization(
    device_auth_id: &str,
    user_code: &str,
) -> Result<OpenAiCodexDevicePollOutcome, String> {
    let request = OpenAiCodexDeviceAuthorizationPollRequest::new(device_auth_id, user_code)?;
    let response = reqwest::blocking::Client::new()
        .post(crate::openai_codex_device_auth::OPENAI_CODEX_DEVICE_TOKEN_URL)
        .headers(reqwest_headers("application/json")?)
        .json(&request.to_json()["body"])
        .send()
        .map_err(|error| format!("OpenAI Codex device authorization poll failed: {error}"))?;
    parse_device_poll_response(response)
}

pub fn exchange_openai_codex_device_token(
    authorization_code: &str,
    code_verifier: &str,
) -> Result<Value, String> {
    let request = OpenAiCodexDeviceTokenExchangeRequest::new(authorization_code, code_verifier)?;
    let body = request.to_json()["body"].clone();
    let response = reqwest::blocking::Client::new()
        .post(OPENAI_CODEX_OAUTH_TOKEN_URL)
        .headers(reqwest_headers("application/x-www-form-urlencoded")?)
        .form(&[
            ("client_id", body["client_id"].as_str().unwrap_or_default()),
            ("code", body["code"].as_str().unwrap_or_default()),
            (
                "code_verifier",
                body["code_verifier"].as_str().unwrap_or_default(),
            ),
            (
                "grant_type",
                body["grant_type"].as_str().unwrap_or_default(),
            ),
            (
                "redirect_uri",
                body["redirect_uri"].as_str().unwrap_or_default(),
            ),
        ])
        .send()
        .map_err(|error| format!("OpenAI Codex token exchange failed: {error}"))?;
    parse_success_json(response, "OpenAI Codex token exchange")
}

pub fn execute_openai_codex_responder_command(
    responder_url: &str,
    payload: &OpenAiCodexResponderCommandPayload,
) -> Result<OpenAiCodexResponderCommandOutput, String> {
    if !is_loopback_codex_responder_url(responder_url) {
        return Err("OpenAI Codex responder must be a loopback URL.".to_string());
    }
    let response = reqwest::blocking::Client::new()
        .post(responder_url)
        .headers(reqwest_headers("application/json")?)
        .json(&payload.to_json())
        .send()
        .map_err(|error| format!("OpenAI Codex responder request failed: {error}"))?;
    let value = parse_success_json(response, "OpenAI Codex responder")?;
    OpenAiCodexResponderCommandOutput::parse(&value.to_string())
}

fn reqwest_headers(content_type: &str) -> Result<reqwest::header::HeaderMap, String> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        reqwest::header::HeaderValue::from_str(content_type)
            .map_err(|error| format!("invalid content type: {error}"))?,
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        reqwest::header::HeaderValue::from_static("desktoplab-local-provider-bridge"),
    );
    headers.insert(
        "originator",
        reqwest::header::HeaderValue::from_static("desktoplab"),
    );
    Ok(headers)
}

fn parse_device_authorization_response(
    response: reqwest::blocking::Response,
) -> Result<OpenAiCodexDeviceAuthorization, String> {
    let value = parse_success_json(response, "OpenAI Codex device authorization")?;
    OpenAiCodexDeviceAuthorization::new(
        string_value(&value, "device_auth_id", "deviceAuthId")?,
        string_value(&value, "user_code", "userCode")?,
        value.get("interval").and_then(Value::as_u64).unwrap_or(5),
    )
}

fn parse_device_poll_response(
    response: reqwest::blocking::Response,
) -> Result<OpenAiCodexDevicePollOutcome, String> {
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if !status.is_success() && text.contains("authorization_pending") {
        return Ok(OpenAiCodexDevicePollOutcome::Pending(
            "Waiting for OpenAI consent.".to_string(),
        ));
    }
    if !status.is_success() {
        return Err(format!(
            "OpenAI Codex device authorization poll failed: {text}"
        ));
    }
    let value: Value = serde_json::from_str(&text).map_err(|error| {
        format!("OpenAI Codex device authorization poll returned invalid JSON: {error}")
    })?;
    Ok(OpenAiCodexDevicePollOutcome::Authorized {
        authorization_code: string_value(&value, "authorization_code", "authorizationCode")?
            .to_string(),
        code_verifier: string_value(&value, "code_verifier", "codeVerifier")?.to_string(),
    })
}

fn parse_success_json(response: reqwest::blocking::Response, label: &str) -> Result<Value, String> {
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("{label} failed: {text}"));
    }
    serde_json::from_str(&text).map_err(|error| format!("{label} returned invalid JSON: {error}"))
}

fn string_value<'a>(value: &'a Value, snake: &str, camel: &str) -> Result<&'a str, String> {
    value
        .get(snake)
        .or_else(|| value.get(camel))
        .and_then(Value::as_str)
        .filter(|entry| !entry.trim().is_empty())
        .ok_or_else(|| format!("{camel} is required"))
}
