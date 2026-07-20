use desktoplab_vault::{DegradedVaultReason, NativeVaultKind, VaultAdapterSelection};
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderAccountRecord {
    provider_id: String,
    account_mode: String,
    status: String,
    vault_ref: Option<String>,
    vault_kind: Option<String>,
    bridge_responder_url: Option<String>,
}

impl ProviderAccountRecord {
    pub(crate) fn from_connection_response(value: &Value) -> Option<Self> {
        if string_field(value, "status") != "connected" {
            return None;
        }
        Some(Self {
            provider_id: string_field(value, "providerId"),
            account_mode: string_field(value, "accountMode"),
            status: "connected".to_string(),
            vault_ref: optional_string_field(value, "vaultRef"),
            vault_kind: optional_string_field(value, "vaultKind"),
            bridge_responder_url: optional_string_field(value, "bridgeResponderUrl"),
        })
    }

    pub(crate) fn removed(provider_id: impl Into<String>, account_mode: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            account_mode: account_mode.into(),
            status: "removed".to_string(),
            vault_ref: None,
            vault_kind: None,
            bridge_responder_url: None,
        }
    }

    pub(crate) fn from_json(value: &Value) -> Self {
        Self {
            provider_id: string_field(value, "providerId"),
            account_mode: string_field(value, "accountMode"),
            status: string_field(value, "status"),
            vault_ref: optional_string_field(value, "vaultRef"),
            vault_kind: optional_string_field(value, "vaultKind"),
            bridge_responder_url: optional_string_field(value, "bridgeResponderUrl"),
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "providerId":self.provider_id,
            "accountMode":self.account_mode,
            "status":self.status,
            "vaultRef":self.vault_ref,
            "vaultKind":self.vault_kind,
            "bridgeResponderUrl":self.bridge_responder_url
        })
    }

    pub(crate) fn provider_id(&self) -> &str {
        &self.provider_id
    }

    pub(crate) fn account_mode(&self) -> &str {
        &self.account_mode
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.status == "connected" && self.vault_ref.is_some()
    }

    pub(crate) fn vault_ref(&self) -> Option<&str> {
        self.vault_ref.as_deref()
    }

    pub(crate) fn vault_kind(&self) -> Option<&str> {
        self.vault_kind.as_deref()
    }

    pub(crate) fn bridge_responder_url(&self) -> Option<&str> {
        self.bridge_responder_url.as_deref()
    }

    pub(crate) fn credential_reference_kind(&self) -> &'static str {
        if self.vault_ref.is_some() {
            "vault_ref"
        } else {
            "none"
        }
    }

    pub(crate) fn is_codex_bridge_ready(&self) -> bool {
        self.provider_id == "provider.openai"
            && matches!(
                self.account_mode.as_str(),
                "subscription_account" | "local_app_session" | "oauth_device"
            )
            && self.is_connected()
            && self.bridge_responder_url.is_some()
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}

pub(crate) fn vault_kind(selection: &VaultAdapterSelection) -> Value {
    match selection {
        VaultAdapterSelection::Available(NativeVaultKind::MacOsKeychain) => {
            json!("macos_keychain")
        }
        VaultAdapterSelection::Available(NativeVaultKind::WindowsCredentialManager) => {
            json!("windows_credential_manager")
        }
        VaultAdapterSelection::Available(NativeVaultKind::LinuxSecretService) => {
            json!("linux_secret_service")
        }
        VaultAdapterSelection::Degraded(_) => Value::Null,
    }
}

pub(crate) fn vault_blocked_reason(selection: &VaultAdapterSelection) -> &'static str {
    match selection {
        VaultAdapterSelection::Degraded(DegradedVaultReason::UnsupportedOperatingSystem(_)) => {
            "unsupported operating system"
        }
        VaultAdapterSelection::Available(_) => "vault unavailable",
    }
}
