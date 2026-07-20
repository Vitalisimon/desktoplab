use desktoplab_backends::{
    OpenAiCodexCompletionPayload, OpenAiCodexDeviceAuthorization, OpenAiCodexDeviceLoginRequest,
    OpenAiCodexDevicePollOutcome, OpenAiCodexPkceLogin, exchange_openai_codex_device_token,
    is_loopback_codex_responder_url, poll_openai_codex_device_authorization,
    request_openai_codex_device_authorization,
};
use desktoplab_vault::{FakeVault, NativeVaultKind, SecretRef, SecretScope, SecretValue, Vault};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OpenAiCodexPairingRecord {
    pairing_id: String,
    pairing_code: String,
    account_mode: String,
    device_auth_id: String,
    user_code: String,
}

impl OpenAiCodexPairingRecord {
    #[must_use]
    pub(crate) fn pairing_id(&self) -> &str {
        &self.pairing_id
    }

    #[must_use]
    pub(crate) fn pairing_code(&self) -> &str {
        &self.pairing_code
    }

    #[must_use]
    pub(crate) fn account_mode(&self) -> &str {
        &self.account_mode
    }

    #[must_use]
    pub(crate) fn device_auth_id(&self) -> &str {
        &self.device_auth_id
    }

    #[must_use]
    pub(crate) fn user_code(&self) -> &str {
        &self.user_code
    }
}

pub(crate) fn start_openai_codex_pairing(
    body: &str,
    device_authorization: Option<OpenAiCodexDeviceAuthorization>,
) -> Result<(OpenAiCodexPairingRecord, Value), Value> {
    let account_mode =
        body_field(body, "accountMode").unwrap_or_else(|| "subscription_account".to_string());
    if !matches!(
        account_mode.as_str(),
        "subscription_account" | "local_app_session" | "oauth_device"
    ) {
        return Err(json!({
            "code":"UNSUPPORTED_ACCOUNT_MODE",
            "message":"OpenAI Codex bridge supports subscription, browser/device and local app session account modes."
        }));
    }
    let seed = runtime_seed();
    let digest = sha256_hex(&format!("openai-codex:{account_mode}:{seed}"));
    let pairing_id = format!("desktoplab_bridge_pair_{}", &digest[..16]);
    let pairing_code = format!("DL-{}", &digest[16..24].to_ascii_uppercase());
    let login = OpenAiCodexPkceLogin::from_verifier_source(digest.as_bytes(), &pairing_id)
        .map_err(|message| json!({"code":"CODEX_BRIDGE_START_INVALID","message":message}))?;
    let device = match device_authorization {
        Some(device) => device,
        None => request_openai_codex_device_authorization()
            .map_err(|message| json!({"code":"CODEX_DEVICE_AUTH_UNAVAILABLE","message":message}))?,
    };
    let record = OpenAiCodexPairingRecord {
        pairing_id: pairing_id.clone(),
        pairing_code: pairing_code.clone(),
        account_mode: account_mode.to_string(),
        device_auth_id: device.device_auth_id().to_string(),
        user_code: device.user_code().to_string(),
    };
    Ok((
        record,
        json!({
            "source":"service_backed",
            "bridgeId":"bridge.openai-codex",
            "providerId":"provider.openai",
            "accountMode":account_mode,
            "status":"authorization_required",
            "pairingId":pairing_id,
            "pairingCode":pairing_code,
            "authorizationUrl":device.verification_url(),
            "redirectUri":login.redirect_uri(),
            "tokenUrl":login.token_url(),
            "codeChallengeMethod":"S256",
            "tokenStorage":"vault_ref_only",
            "deviceLogin":{
                "request":OpenAiCodexDeviceLoginRequest::to_json(),
                "deviceAuthId":device.device_auth_id(),
                "userCode":device.user_code(),
                "verificationUrl":device.verification_url(),
                "intervalSeconds":device.interval_seconds()
            },
            "completionPath":"/v1/provider-bridges/openai-codex/pairing/complete",
            "pollPath":"/v1/provider-bridges/openai-codex/pairing/poll",
            "message":"Sign in with OpenAI Codex. DesktopLab will store only a local credential reference after consent."
        }),
    ))
}

pub(crate) fn complete_openai_codex_pairing(
    body: &str,
    pairing: Option<&OpenAiCodexPairingRecord>,
) -> Result<Value, Value> {
    if body.contains("access_token") || body.contains("refresh_token") {
        return Err(json!({
            "code":"RAW_PROVIDER_TOKEN_REJECTED",
            "message":"Raw provider tokens are never accepted by DesktopLab bridge routes."
        }));
    }
    let Some(pairing) = pairing else {
        return Err(
            json!({"code":"PAIRING_NOT_FOUND","message":"OpenAI Codex pairing was not found or expired."}),
        );
    };
    if body_field(body, "pairingId").as_deref() != Some(pairing.pairing_id()) {
        return Err(
            json!({"code":"PAIRING_ID_MISMATCH","message":"OpenAI Codex pairing id does not match."}),
        );
    }
    if body_field(body, "pairingCode").as_deref() != Some(pairing.pairing_code()) {
        return Err(
            json!({"code":"PAIRING_CODE_MISMATCH","message":"OpenAI Codex pairing code does not match."}),
        );
    }
    let responder_url = body_field(body, "responderUrl").unwrap_or_default();
    if !is_loopback_codex_responder_url(&responder_url) {
        return Err(json!({
            "code":"RESPONDER_MUST_BE_LOOPBACK",
            "message":"OpenAI Codex responder must be a loopback URL owned by this machine."
        }));
    }
    let payload = OpenAiCodexCompletionPayload::new(
        &body_field(body, "bridgeInstanceId").unwrap_or_default(),
        &body_field(body, "localCredentialRef").unwrap_or_default(),
        pairing.pairing_code(),
        pairing.pairing_id(),
        &body_field(body, "providerAccountLabel").unwrap_or_else(|| "OpenAI Codex".to_string()),
    )
    .map_err(|message| json!({"code":"INVALID_CODEX_COMPLETION","message":message}))?;
    Ok(json!({
        "source":"service_backed",
        "providerId":"provider.openai",
        "status":"connected",
        "accountMode":pairing.account_mode(),
        "vaultRef":payload.local_credential_ref(),
        "vaultKind":current_native_vault_kind(),
        "bridgeId":"bridge.openai-codex",
        "bridgeResponderUrl":responder_url,
        "bridgeCapabilities":payload.to_json()["bridgeCapabilities"].clone(),
        "message":"OpenAI Codex subscription bridge connected through local credential reference."
    }))
}

pub(crate) fn poll_openai_codex_pairing(
    body: &str,
    pairing: Option<&OpenAiCodexPairingRecord>,
    _credential_dir: &Path,
    test_authorization: Option<(&str, &str)>,
    native_vault_for_test: Option<&mut FakeVault>,
) -> Result<Value, Value> {
    reject_raw_tokens(body)?;
    let Some(pairing) = pairing else {
        return Err(
            json!({"code":"PAIRING_NOT_FOUND","message":"OpenAI Codex pairing was not found or expired."}),
        );
    };
    if body_field(body, "pairingId").as_deref() != Some(pairing.pairing_id()) {
        return Err(
            json!({"code":"PAIRING_ID_MISMATCH","message":"OpenAI Codex pairing id does not match."}),
        );
    }
    let outcome = match test_authorization {
        Some((authorization_code, code_verifier)) => OpenAiCodexDevicePollOutcome::Authorized {
            authorization_code: authorization_code.to_string(),
            code_verifier: code_verifier.to_string(),
        },
        None => {
            poll_openai_codex_device_authorization(pairing.device_auth_id(), pairing.user_code())
                .map_err(|message| json!({"code":"CODEX_DEVICE_POLL_FAILED","message":message}))?
        }
    };
    let OpenAiCodexDevicePollOutcome::Authorized {
        authorization_code,
        code_verifier,
    } = outcome
    else {
        return Ok(json!({
            "source":"service_backed",
            "providerId":"provider.openai",
            "status":"authorization_pending",
            "pairingId":pairing.pairing_id(),
            "message":"Waiting for OpenAI consent."
        }));
    };
    let token_payload = if test_authorization.is_some() {
        json!({"token_type":"Bearer","access_token":"[test-redacted]","refresh_token":"[test-redacted]"})
    } else {
        exchange_openai_codex_device_token(&authorization_code, &code_verifier)
            .map_err(|message| json!({"code":"CODEX_TOKEN_EXCHANGE_FAILED","message":message}))?
    };
    let stored = store_openai_codex_credential(pairing, &token_payload, native_vault_for_test)
        .map_err(|message| json!({"code":"CODEX_CREDENTIAL_STORE_FAILED","message":message}))?;
    Ok(json!({
        "source":"service_backed",
        "providerId":"provider.openai",
        "status":"connected",
        "accountMode":pairing.account_mode(),
        "vaultRef":stored.vault_ref,
        "vaultKind":stored.vault_kind,
        "bridgeId":"bridge.openai-codex",
        "bridgeResponderUrl":Value::Null,
        "bridgeCapabilities":["account.consent","credential.native_vault_ref"],
        "diagnostic":{
            "state":"ready",
            "message":"OpenAI Codex consent completed and the credential was stored in the native vault.",
            "redactedEvidence":"credential=[REDACTED]"
        },
        "message":"OpenAI Codex account connected. Execution stays blocked until a local responder is available."
    }))
}

pub(crate) fn codex_responder_reachable(responder_url: &str) -> bool {
    if !is_loopback_codex_responder_url(responder_url) {
        return false;
    }
    let authority = responder_url
        .trim()
        .strip_prefix("http://")
        .unwrap_or(responder_url)
        .split('/')
        .next()
        .unwrap_or_default();
    authority.to_socket_addrs().is_ok_and(|addresses| {
        addresses
            .into_iter()
            .any(|address| TcpStream::connect_timeout(&address, Duration::from_millis(120)).is_ok())
    })
}

pub(crate) fn certify_openai_codex_bridge(body: &str) -> Result<Value, Value> {
    reject_raw_tokens(body)?;
    let vault_ref = body_field(body, "vaultRef").unwrap_or_default();
    let responder_url = body_field(body, "responderUrl").unwrap_or_default();
    let responder_state = body_field(body, "responderState").unwrap_or_default();
    let egress_state = body_field(body, "repositoryContextEgressApproval").unwrap_or_default();
    let capabilities = string_array_field(body, "capabilities");
    let mut blocked_reasons = Vec::new();

    if !vault_ref.starts_with("vault://desktoplab/external-backend/openai-codex/") {
        blocked_reasons.push("credential_vault_ref_missing");
    }
    if !is_loopback_codex_responder_url(&responder_url) {
        blocked_reasons.push("responder_must_be_loopback");
    } else if responder_state != "healthy" && !codex_responder_reachable(&responder_url) {
        blocked_reasons.push("responder_health_missing");
    }
    if egress_state != "approved" {
        blocked_reasons.push("repository_context_egress_approval_missing");
    }
    for required in [
        "account.consent",
        "credential.native_vault_ref",
        "event_stream.normalized",
        "tool_request.delegated",
    ] {
        if !capabilities.iter().any(|capability| capability == required) {
            blocked_reasons.push("capability_mapping_incomplete");
            break;
        }
    }

    let status = if blocked_reasons.is_empty() {
        "certified_private_dev"
    } else {
        "blocked"
    };
    Ok(json!({
        "source":"service_backed",
        "bridgeId":"bridge.openai-codex",
        "providerId":"provider.openai",
        "certificationScope":"private_dev_only",
        "status":status,
        "publicClaim":"not_supported",
        "vaultRef":vault_ref,
        "responderUrl":responder_url,
        "requiredCapabilities":[
            "account.consent",
            "credential.native_vault_ref",
            "event_stream.normalized",
            "tool_request.delegated"
        ],
        "blockedReasons":blocked_reasons
    }))
}

fn reject_raw_tokens(body: &str) -> Result<(), Value> {
    if body.contains("access_token") || body.contains("refresh_token") {
        return Err(json!({
            "code":"RAW_PROVIDER_TOKEN_REJECTED",
            "message":"Raw provider tokens are never accepted by DesktopLab bridge routes."
        }));
    }
    Ok(())
}

struct StoredOpenAiCodexCredential {
    vault_ref: String,
    vault_kind: &'static str,
}

fn store_openai_codex_credential(
    pairing: &OpenAiCodexPairingRecord,
    token_payload: &Value,
    native_vault_for_test: Option<&mut FakeVault>,
) -> Result<StoredOpenAiCodexCredential, String> {
    let secret_ref = SecretRef::new(
        SecretScope::ExternalBackend,
        format!("openai-codex/{}", pairing.pairing_id()),
    );
    if let Some(vault) = native_vault_for_test {
        vault
            .put(
                secret_ref.clone(),
                SecretValue::new(token_payload.to_string()),
            )
            .map_err(|error| format!("{error:?}"))?;
        return Ok(StoredOpenAiCodexCredential {
            vault_ref: secret_ref.as_uri(),
            vault_kind: "macos_keychain",
        });
    }
    desktoplab_vault::put_current_native_secret(
        secret_ref.clone(),
        SecretValue::new(token_payload.to_string()),
    )
    .map_err(|error| format!("{error:?}"))?;
    Ok(StoredOpenAiCodexCredential {
        vault_ref: secret_ref.as_uri(),
        vault_kind: current_native_vault_kind(),
    })
}

fn current_native_vault_kind() -> &'static str {
    match desktoplab_vault::VaultAdapterSelection::current() {
        desktoplab_vault::VaultAdapterSelection::Available(NativeVaultKind::MacOsKeychain) => {
            "macos_keychain"
        }
        desktoplab_vault::VaultAdapterSelection::Available(
            NativeVaultKind::WindowsCredentialManager,
        ) => "windows_credential_manager",
        desktoplab_vault::VaultAdapterSelection::Available(NativeVaultKind::LinuxSecretService) => {
            "linux_secret_service"
        }
        desktoplab_vault::VaultAdapterSelection::Degraded(_) => "native_vault",
    }
}

fn body_field(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(str::to_string)
}

fn string_array_field(body: &str, field: &str) -> Vec<String> {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get(field).cloned())
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn runtime_seed() -> String {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).expect("OS random source should be available");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sha256_hex(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
