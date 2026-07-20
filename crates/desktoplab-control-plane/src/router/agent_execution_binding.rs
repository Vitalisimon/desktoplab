use serde_json::{Value, json};

use super::LocalApiRouter;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentExecutionBinding {
    route_id: String,
    backend_id: String,
    runtime_id: Option<String>,
    model_id: Option<String>,
    endpoint: Option<String>,
    provider_id: Option<String>,
    account_mode: Option<String>,
    vault_ref: Option<String>,
    vault_kind: Option<String>,
    approval_mode: String,
}

impl AgentExecutionBinding {
    pub(crate) fn capture(router: &LocalApiRouter, backend_id: &str) -> Self {
        let (runtime_id, model_id, endpoint) = if backend_id == "backend.high-end-local" {
            router
                .high_end_runtime
                .as_ref()
                .map_or((None, None, None), |runtime| {
                    (
                        Some("runtime.high-end-local".to_string()),
                        Some(runtime.endpoint().model_id().to_string()),
                        Some(runtime.endpoint().base_url().to_string()),
                    )
                })
        } else if backend_id == "backend.codex" {
            (
                None,
                None,
                router
                    .provider_accounts
                    .get("provider.openai")
                    .and_then(|account| account.bridge_responder_url())
                    .map(str::to_string),
            )
        } else if backend_id.starts_with("backend.") {
            (
                router.selected_local_runtime_id(),
                router.selected_local_model_id().ok(),
                None,
            )
        } else {
            (None, None, None)
        };
        let account = (backend_id == "backend.codex")
            .then(|| router.provider_accounts.get("provider.openai"))
            .flatten();
        Self {
            route_id: router.selected_route_id.clone(),
            backend_id: backend_id.to_string(),
            runtime_id,
            model_id,
            endpoint,
            provider_id: account.map(|account| account.provider_id().to_string()),
            account_mode: account.map(|account| account.account_mode().to_string()),
            vault_ref: account.and_then(|account| account.vault_ref().map(str::to_string)),
            vault_kind: account.and_then(|account| account.vault_kind().map(str::to_string)),
            approval_mode: router.session_approval_mode.as_str().to_string(),
        }
    }

    pub(crate) fn backend_id(&self) -> &str {
        &self.backend_id
    }

    pub(crate) fn model_id(&self) -> Option<&str> {
        self.model_id.as_deref()
    }

    pub(crate) fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    pub(crate) fn provider_id(&self) -> Option<&str> {
        self.provider_id.as_deref()
    }

    pub(crate) fn account_mode(&self) -> Option<&str> {
        self.account_mode.as_deref()
    }

    pub(crate) fn vault_ref(&self) -> Option<&str> {
        self.vault_ref.as_deref()
    }

    pub(crate) fn vault_kind(&self) -> Option<&str> {
        self.vault_kind.as_deref()
    }

    pub(crate) fn approval_mode(&self) -> desktoplab_policy::ApprovalMode {
        desktoplab_policy::ApprovalMode::from_stable_str(&self.approval_mode).unwrap_or_default()
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "routeId":self.route_id,
            "backendId":self.backend_id,
            "runtimeId":self.runtime_id,
            "modelId":self.model_id,
            "endpoint":self.endpoint,
            "providerId":self.provider_id,
            "accountMode":self.account_mode,
            "vaultRef":self.vault_ref,
            "vaultKind":self.vault_kind,
            "approvalMode":self.approval_mode
        })
    }

    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            route_id: value.get("routeId")?.as_str()?.to_string(),
            backend_id: value.get("backendId")?.as_str()?.to_string(),
            runtime_id: optional_string(value, "runtimeId"),
            model_id: optional_string(value, "modelId"),
            endpoint: optional_string(value, "endpoint"),
            provider_id: optional_string(value, "providerId"),
            account_mode: optional_string(value, "accountMode"),
            vault_ref: optional_string(value, "vaultRef"),
            vault_kind: optional_string(value, "vaultKind"),
            approval_mode: value
                .get("approvalMode")
                .and_then(Value::as_str)
                .unwrap_or("require_approval")
                .to_string(),
        })
    }
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AgentExecutionBinding, LocalApiRouter};
    use crate::provider_accounts::ProviderAccountRecord;

    #[test]
    fn execution_binding_round_trips_without_secret_material() {
        let router = LocalApiRouter::default();
        let binding = AgentExecutionBinding::capture(&router, "backend.ollama");
        let payload = binding.to_json();

        assert_eq!(AgentExecutionBinding::from_json(&payload), Some(binding));
        assert!(payload.get("credential").is_none());
        assert!(payload.get("prompt").is_none());
    }

    #[test]
    fn codex_binding_freezes_account_references_without_secret_material() {
        let mut router = LocalApiRouter::default();
        router.provider_accounts.insert(
            "provider.openai".to_string(),
            ProviderAccountRecord::from_json(&json!({
                "providerId":"provider.openai",
                "accountMode":"oauth_device",
                "status":"connected",
                "vaultRef":"vault://desktoplab/codex/profile/one",
                "vaultKind":"macos_keychain",
                "bridgeResponderUrl":"http://127.0.0.1:43123"
            })),
        );

        let binding = AgentExecutionBinding::capture(&router, "backend.codex");
        let payload = binding.to_json();

        assert_eq!(payload["accountMode"], "oauth_device");
        assert_eq!(payload["vaultRef"], "vault://desktoplab/codex/profile/one");
        assert_eq!(payload["endpoint"], "http://127.0.0.1:43123");
        assert!(payload.get("secret").is_none());
        assert_eq!(AgentExecutionBinding::from_json(&payload), Some(binding));
    }
}
