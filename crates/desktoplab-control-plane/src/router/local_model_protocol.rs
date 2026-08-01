use desktoplab_backends::{BackendModelCapabilities, ModelToolProtocolKind};

use super::LocalApiRouter;

struct RuntimeModelConnection {
    endpoint: String,
    version: String,
    protocol: ModelToolProtocolKind,
    evidence: String,
}

impl LocalApiRouter {
    pub(crate) fn refresh_local_model_capabilities(
        &mut self,
        runtime_id: &str,
        model_id: &str,
        pull_ref: &str,
    ) {
        self.refresh_local_model_capabilities_with_mode(runtime_id, model_id, pull_ref, false);
    }

    pub(crate) fn refresh_local_model_capabilities_fresh(
        &mut self,
        runtime_id: &str,
        model_id: &str,
        pull_ref: &str,
    ) {
        self.refresh_local_model_capabilities_with_mode(runtime_id, model_id, pull_ref, true);
    }

    fn refresh_local_model_capabilities_with_mode(
        &mut self,
        runtime_id: &str,
        model_id: &str,
        pull_ref: &str,
        force_canary: bool,
    ) {
        if runtime_id == "runtime.ollama" {
            if force_canary {
                self.refresh_ollama_model_capabilities_fresh(runtime_id, pull_ref);
            } else {
                self.refresh_ollama_model_capabilities(runtime_id, pull_ref);
            }
            return;
        }
        let Some(connection) = self.runtime_model_connection(runtime_id, pull_ref) else {
            return;
        };
        self.readiness
            .mark_runtime_verified(runtime_id.to_string(), connection.evidence.clone());
        let context_window = crate::model_routes::agent_context_window_tokens(
            model_id,
            self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
        )
        .map(u64::from);
        let backend_id = match runtime_id {
            "runtime.lm-studio" => "backend.lm-studio",
            "runtime.mlx-lm" => "backend.mlx-lm",
            _ => return,
        };
        let mut capabilities = BackendModelCapabilities::reported(
            backend_id,
            pull_ref,
            Some(connection.version),
            context_window,
            ["completion", "tools"],
        );
        let persisted = self
            .readiness
            .model_capabilities()
            .filter(|current| current.fingerprint() == capabilities.fingerprint())
            .and_then(BackendModelCapabilities::tool_protocol_certification)
            .cloned();
        let certification = persisted.filter(|_| !force_canary).unwrap_or_else(|| {
            let timeout = crate::model_routes::agent_request_timeout_seconds(
                model_id,
                self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
            )
            .unwrap_or(240);
            if force_canary {
                self.openai_compatible_tool_protocol_canary.certify_fresh(
                    &connection.endpoint,
                    &capabilities,
                    connection.protocol,
                    timeout,
                )
            } else {
                self.openai_compatible_tool_protocol_canary.certify(
                    &connection.endpoint,
                    &capabilities,
                    connection.protocol,
                    timeout,
                )
            }
        });
        capabilities = capabilities.with_tool_protocol_certification(certification);
        self.readiness.mark_model_capabilities(capabilities);
    }

    fn runtime_model_connection(
        &self,
        runtime_id: &str,
        pull_ref: &str,
    ) -> Option<RuntimeModelConnection> {
        #[cfg(debug_assertions)]
        if self
            .local_model_inventory_for_test
            .as_ref()
            .is_some_and(|models| models.iter().any(|model| model == pull_ref))
        {
            return test_connection(runtime_id);
        }
        match runtime_id {
            "runtime.lm-studio" => lm_studio_connection(pull_ref),
            "runtime.mlx-lm" => mlx_connection(pull_ref),
            _ => None,
        }
    }
}

fn lm_studio_connection(pull_ref: &str) -> Option<RuntimeModelConnection> {
    if let Some((endpoint, models, version)) =
        crate::runtime_routes::lm_studio_managed::capability_connection()
        && models.iter().any(|model| model == pull_ref)
    {
        return Some(RuntimeModelConnection {
            version: format!("llmster-{version}@{endpoint}"),
            evidence: format!(
                "runtime.lm-studio desktoplab_managed ready; endpoint={endpoint}; version={version}"
            ),
            endpoint,
            protocol: ModelToolProtocolKind::NativeTools,
        });
    }
    let discovery = desktoplab_runtime::discover_system_lm_studio();
    let endpoint = discovery.endpoint().filter(|_| discovery.ready())?;
    discovery
        .models()
        .iter()
        .any(|model| model == pull_ref)
        .then(|| RuntimeModelConnection {
            version: format!("existing@{endpoint}"),
            evidence: format!("runtime.lm-studio user_owned ready; endpoint={endpoint}"),
            endpoint: endpoint.to_string(),
            protocol: ModelToolProtocolKind::NativeTools,
        })
}

fn mlx_connection(pull_ref: &str) -> Option<RuntimeModelConnection> {
    let status = crate::runtime_routes::mlx_lm_managed::status()
        .filter(desktoplab_runtime::MlxLmManagedStatus::ready)?;
    status
        .models()
        .iter()
        .any(|model| model == pull_ref)
        .then(|| RuntimeModelConnection {
            version: format!("{}@{}", status.model_revision(), status.endpoint()),
            evidence: format!(
                "runtime.mlx-lm desktoplab_managed ready; endpoint={}; revision={}",
                status.endpoint(),
                status.model_revision()
            ),
            endpoint: status.endpoint().to_string(),
            protocol: ModelToolProtocolKind::ConstrainedJson,
        })
}

#[cfg(debug_assertions)]
fn test_connection(runtime_id: &str) -> Option<RuntimeModelConnection> {
    match runtime_id {
        "runtime.lm-studio" => Some(RuntimeModelConnection {
            endpoint: "http://127.0.0.1:12345".to_string(),
            version: "test-lm-studio@127.0.0.1:12345".to_string(),
            evidence: "runtime.lm-studio test ready".to_string(),
            protocol: ModelToolProtocolKind::NativeTools,
        }),
        "runtime.mlx-lm" => Some(RuntimeModelConnection {
            endpoint: "http://127.0.0.1:18080".to_string(),
            version: "test-mlx@127.0.0.1:18080".to_string(),
            evidence: "runtime.mlx-lm test ready".to_string(),
            protocol: ModelToolProtocolKind::ConstrainedJson,
        }),
        _ => None,
    }
}
