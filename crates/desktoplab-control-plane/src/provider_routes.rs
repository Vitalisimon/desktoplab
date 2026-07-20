use desktoplab_agent_engine::{OpenAiCompatibleEndpoint, OpenAiCompatibleEndpointPolicy};
use desktoplab_vault::FakeVault;
use serde_json::{Value, json};

use crate::provider_accounts::ProviderAccountRecord;

mod account_profile;
mod credentials;

use account_profile::{auth_profile_health, supported_account_modes};

#[must_use]
pub(crate) fn providers_response(account: Option<&ProviderAccountRecord>) -> Value {
    let connected = account.filter(|account| account.is_connected());
    json!({
        "source":"service_backed",
        "providers":[{
            "providerId":"provider.openai",
            "displayName":"OpenAI",
            "status":if connected.is_some() { "connected" } else { "missing_credential" },
            "trust":"verified",
            "egress":"requires_approval",
            "capabilities":["llm.chat"],
            "supportedAccountModes":supported_account_modes(),
            "activeAccountMode":connected.map(ProviderAccountRecord::account_mode).unwrap_or("api_key_billing"),
            "vaultRef":connected.and_then(ProviderAccountRecord::vault_ref),
            "vaultKind":connected.and_then(ProviderAccountRecord::vault_kind),
            "bridgeResponderUrl":connected.and_then(ProviderAccountRecord::bridge_responder_url),
            "authProfileHealth":auth_profile_health(connected),
            "diagnostic":{
                "state":if connected.is_some() { "degraded" } else { "missing_credential" },
                "message":if connected.is_some_and(ProviderAccountRecord::is_codex_bridge_ready) { "OpenAI Codex subscription bridge is connected through a local responder." } else if connected.is_some() { "Credential reference is stored; live provider execution remains blocked until certified." } else { "Connect an account before cloud routing." },
                "redactedEvidence":if connected.is_some() { "credential=[REDACTED]; token_storage=vault_ref_only" } else { "No credential stored." },
                "repairActions":[{"id":"provider.connect","label":"Connect","description":"Store credentials in the native vault."}]
            }
        }]
    })
}

#[must_use]
pub fn connect_provider_response(
    path: &str,
    body: &str,
    native_vault_for_test: Option<&mut FakeVault>,
) -> Value {
    let provider_id = segment(path, 2);
    let account_mode = account_mode(body);
    if account_mode == "custom_endpoint" {
        return custom_endpoint_response(&provider_id, body);
    }
    if account_mode != "api_key_billing" {
        return blocked_account_mode_response(&provider_id, &account_mode);
    }
    credentials::connect_api_key(&provider_id, &account_mode, body, native_vault_for_test)
}

#[must_use]
pub(crate) fn provider_diagnostics_response(
    path: &str,
    account: Option<&ProviderAccountRecord>,
) -> Value {
    let provider_id = segment(path, 2);
    if account.is_some_and(|account| account.provider_id() == provider_id && account.is_connected())
    {
        return json!({
            "source":"service_backed",
            "providerId":provider_id,
            "state":"degraded",
            "authProfileHealth":auth_profile_health(account),
            "message":"Credential reference is stored; live provider execution remains blocked until certified.",
            "redactedEvidence":"credential=[REDACTED]; live_call=blocked",
            "repairActions":[{"id":"provider.live.certify","label":"Certify provider","description":"Run live account certification before enabling provider execution."}]
        });
    }
    json!({
        "source":"service_backed",
        "providerId":provider_id,
        "state":"missing_credential",
        "authProfileHealth":auth_profile_health(None),
        "message":"No provider credential evidence is stored yet.",
        "redactedEvidence":"credential=[NOT STORED]",
        "repairActions":[{"id":"provider.connect","label":"Connect","description":"Store credentials in the native vault."}]
    })
}

#[must_use]
pub fn test_provider_response(
    path: &str,
    body: &str,
    native_vault_for_test: Option<&FakeVault>,
) -> Value {
    credentials::test_api_key(
        &segment(path, 2),
        &account_mode(body),
        native_vault_for_test,
    )
}

#[must_use]
pub fn disconnect_provider_response(
    path: &str,
    body: &str,
    persisted_vault_ref: Option<&str>,
    native_vault_for_test: Option<&mut FakeVault>,
) -> Value {
    credentials::disconnect_api_key(
        &segment(path, 2),
        &account_mode(body),
        persisted_vault_ref,
        native_vault_for_test,
    )
}

fn custom_endpoint_response(provider_id: &str, body: &str) -> Value {
    let endpoint_url = body_field(body, "endpointUrl").unwrap_or_default();
    let policy = if body_bool(body, "allowRemoteHttps") {
        OpenAiCompatibleEndpointPolicy::allow_remote_https()
    } else {
        OpenAiCompatibleEndpointPolicy::local_only()
    };
    match OpenAiCompatibleEndpoint::validate(&endpoint_url, policy) {
        Ok(endpoint) => json!({
            "source":"service_backed",
            "providerId":provider_id,
            "status":"blocked",
            "accountMode":"custom_endpoint",
            "vaultRef":Value::Null,
            "endpointClass":format!("{:?}", endpoint.class()).to_ascii_lowercase(),
            "blockedReason":"custom_endpoint_health_check_not_certified",
            "message":"Endpoint syntax is valid, but DesktopLab has not certified health/model-listing execution for this endpoint yet.",
            "redactedEvidence":"endpoint=[VALIDATED]; credential=[NOT STORED]"
        }),
        Err(error) => json!({
            "source":"service_backed",
            "providerId":provider_id,
            "status":"blocked",
            "accountMode":"custom_endpoint",
            "vaultRef":Value::Null,
            "blockedReason":format!("{error:?}").to_ascii_lowercase(),
            "message":"Custom endpoint is not usable yet.",
            "redactedEvidence":"endpoint=[REDACTED]; credential=[NOT STORED]"
        }),
    }
}

fn blocked_account_mode_response(provider_id: &str, account_mode: &str) -> Value {
    json!({
        "source":"service_backed",
        "providerId":provider_id,
        "status":"blocked",
        "accountMode":account_mode,
        "vaultRef":Value::Null,
        "blockedReason":if account_mode == "custom_endpoint" { "custom_endpoint_not_validated" } else { "account_bridge_not_certified" },
        "message":"This account bridge is not executable yet. DesktopLab keeps it separate from API-key billing until a bridge is certified.",
        "diagnostic":{
            "state":"blocked",
            "message":"Account bridge is future/guided until DesktopLab can verify it.",
            "redactedEvidence":"credential=[NOT STORED]",
            "repairActions":[{"id":"provider.bridge.certify","label":"Certify bridge","description":"Add bridge discovery, account ownership and capability evidence before enabling this mode."}]
        }
    })
}

fn account_mode(body: &str) -> String {
    body_field(body, "accountMode").unwrap_or_else(|| "api_key_billing".to_string())
}

fn body_field(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(str::to_string)
}

fn body_bool(body: &str, field: &str) -> bool {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get(field).and_then(Value::as_bool).map(bool::from))
        .unwrap_or(false)
}

fn segment(path: &str, index: usize) -> String {
    path.split('/')
        .nth(index + 1)
        .unwrap_or_default()
        .to_string()
}
