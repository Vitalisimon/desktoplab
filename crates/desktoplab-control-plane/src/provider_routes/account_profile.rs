use serde_json::{Value, json};

use crate::provider_accounts::ProviderAccountRecord;

pub(super) fn supported_account_modes() -> [&'static str; 5] {
    [
        "api_key_billing",
        "subscription_account",
        "oauth_device",
        "local_app_session",
        "custom_endpoint",
    ]
}

pub(super) fn auth_profile_health(account: Option<&ProviderAccountRecord>) -> Value {
    let account_mode = account
        .map(ProviderAccountRecord::account_mode)
        .unwrap_or("api_key_billing");
    json!({
        "authMode":account_mode,
        "credentialReferenceKind":account
            .map(ProviderAccountRecord::credential_reference_kind)
            .unwrap_or("none"),
        "credentialRef":account.and_then(ProviderAccountRecord::vault_ref),
        "lastHealthState":if account.is_some_and(ProviderAccountRecord::is_connected) { "probe_required" } else { "missing_credential" },
        "cooldownState":if account.is_some() { "not_probed" } else { "none" },
        "fallbackOrder":fallback_order(account_mode),
        "fallbackApproval":"explicit_user_approval_required",
        "degradedReason":if account.is_some() { json!("provider_probe_required") } else { json!("credential_reference_missing") }
    })
}

fn fallback_order(account_mode: &str) -> Vec<&'static str> {
    if account_mode == "subscription_account" {
        vec!["subscription_account", "api_key_billing"]
    } else {
        vec![account_mode_label(account_mode)]
    }
}

fn account_mode_label(account_mode: &str) -> &'static str {
    match account_mode {
        "subscription_account" => "subscription_account",
        "oauth_device" => "oauth_device",
        "local_app_session" => "local_app_session",
        "custom_endpoint" => "custom_endpoint",
        _ => "api_key_billing",
    }
}
