use desktoplab_vault::{
    FakeVault, SecretRef, SecretScope, SecretValue, Vault, VaultAdapterSelection, VaultError,
};
use serde_json::{Value, json};

use crate::provider_accounts::{vault_blocked_reason, vault_kind};

pub(super) fn connect_api_key(
    provider_id: &str,
    account_mode: &str,
    body: &str,
    native_vault_for_test: Option<&mut FakeVault>,
) -> Value {
    let selection = VaultAdapterSelection::current();
    if !selection.can_save_credentials() {
        return blocked_connect(provider_id, account_mode, &selection);
    }
    let secret_ref = secret_ref(provider_id, account_mode);
    let Some(api_key) = body_field(body, "apiKey").filter(|value| !value.trim().is_empty()) else {
        return failure(
            provider_id,
            account_mode,
            "credential_missing",
            "An API key is required and was not stored.",
        );
    };
    let result = if let Some(vault) = native_vault_for_test {
        vault.put(secret_ref.clone(), SecretValue::new(api_key))
    } else {
        desktoplab_vault::put_current_native_secret(secret_ref.clone(), SecretValue::new(api_key))
    };
    if let Err(error) = result {
        return failure(
            provider_id,
            account_mode,
            "native_vault_write_failed",
            &error_message(error),
        );
    }
    json!({
        "source":"service_backed",
        "providerId":provider_id,
        "status":"connected",
        "vaultRef":secret_ref.as_uri(),
        "accountMode":account_mode,
        "vaultKind":vault_kind(&selection),
        "plaintextFallbackAllowed":selection.allows_plaintext_fallback(),
        "message":"Credential stored in the operating-system vault.",
        "diagnostic":{
            "state":"degraded",
            "message":"The credential is stored. Live provider execution remains disabled until its remote route is certified.",
            "redactedEvidence":"credential=[REDACTED]; token_storage=vault_ref_only"
        }
    })
}

pub(super) fn test_api_key(
    provider_id: &str,
    account_mode: &str,
    native_vault_for_test: Option<&FakeVault>,
) -> Value {
    let secret_ref = secret_ref(provider_id, account_mode);
    let result = if let Some(vault) = native_vault_for_test {
        vault.get(&secret_ref)
    } else {
        desktoplab_vault::get_current_native_secret(&secret_ref)
    };
    match result {
        Ok(_) => json!({
            "source":"service_backed",
            "providerId":provider_id,
            "state":"degraded",
            "accountMode":account_mode,
            "vaultRef":secret_ref.as_uri(),
            "message":"The credential is readable from the operating-system vault. Remote authentication was not attempted.",
            "redactedEvidence":"credential=[REDACTED]; vault_read=verified; remote_call=not_run",
            "repairActions":[{"id":"provider.live.certify","label":"Certify provider","description":"Run live account certification before enabling provider execution."}]
        }),
        Err(error) => failure(
            provider_id,
            account_mode,
            "native_vault_read_failed",
            &error_message(error),
        ),
    }
}

pub(super) fn disconnect_api_key(
    provider_id: &str,
    account_mode: &str,
    persisted_vault_ref: Option<&str>,
    native_vault_for_test: Option<&mut FakeVault>,
) -> Value {
    let secret_ref = match persisted_vault_ref {
        Some(vault_ref) => match SecretRef::from_uri(vault_ref) {
            Ok(secret_ref) => secret_ref,
            Err(_) => {
                return failure(
                    provider_id,
                    account_mode,
                    "invalid_persisted_vault_ref",
                    "The stored credential reference is invalid; no credential was removed.",
                );
            }
        },
        None => {
            return failure(
                provider_id,
                account_mode,
                "credential_reference_missing",
                "No stored credential reference is available to remove.",
            );
        }
    };
    let result = if let Some(vault) = native_vault_for_test {
        vault.delete(&secret_ref)
    } else {
        desktoplab_vault::delete_current_native_secret(&secret_ref)
    };
    if let Err(error) = result {
        return failure(
            provider_id,
            account_mode,
            "native_vault_delete_failed",
            &error_message(error),
        );
    }
    json!({
        "source":"service_backed",
        "providerId":provider_id,
        "status":"removed",
        "accountMode":account_mode,
        "vaultRef":secret_ref.as_uri(),
        "message":"Credential removed from the operating-system vault and DesktopLab provider state."
    })
}

fn blocked_connect(
    provider_id: &str,
    account_mode: &str,
    selection: &VaultAdapterSelection,
) -> Value {
    json!({
        "source":"service_backed",
        "providerId":provider_id,
        "status":"blocked",
        "accountMode":account_mode,
        "vaultRef":Value::Null,
        "plaintextFallbackAllowed":selection.allows_plaintext_fallback(),
        "blockedReason":vault_blocked_reason(selection),
        "message":"Native credential storage is unavailable.",
        "diagnostic":{"state":"blocked","message":"Native credential storage is unavailable.","redactedEvidence":"credential=[NOT STORED]"}
    })
}

fn failure(provider_id: &str, account_mode: &str, reason: &str, message: &str) -> Value {
    json!({
        "source":"service_backed",
        "providerId":provider_id,
        "status":"blocked",
        "state":"blocked",
        "accountMode":account_mode,
        "vaultRef":Value::Null,
        "blockedReason":reason,
        "message":message,
        "diagnostic":{"state":"blocked","message":message,"redactedEvidence":"credential=[NOT STORED]"}
    })
}

fn secret_ref(provider_id: &str, account_mode: &str) -> SecretRef {
    SecretRef::new(
        SecretScope::Provider,
        format!("{provider_id}:{account_mode}"),
    )
}

fn error_message(error: VaultError) -> String {
    match error {
        VaultError::SecretNotFound(_) => {
            "Credential was not found in the operating-system vault.".to_string()
        }
        VaultError::Unavailable(reason) => format!("Operating-system vault unavailable: {reason}"),
    }
}

fn body_field(body: &str, field: &str) -> Option<String> {
    serde_json::from_str::<Value>(body)
        .ok()?
        .get(field)?
        .as_str()
        .map(str::to_string)
}
