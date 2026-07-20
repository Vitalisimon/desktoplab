use serde_json::json;

use super::{AgentBackendExecutionMode, ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub fn enable_test_controls_for_dev_server(&mut self) {
        self.test_controls_enabled = cfg!(debug_assertions);
    }

    pub fn inject_state_journal_fault_for_test(&mut self, message: impl Into<String>) {
        assert!(
            cfg!(debug_assertions),
            "journal fault injection is debug-only"
        );
        self.state_journal_fault = Some(message.into());
    }

    pub fn clear_state_journal_fault_for_test(&mut self) {
        assert!(
            cfg!(debug_assertions),
            "journal fault injection is debug-only"
        );
        self.state_journal_fault = None;
    }

    pub(crate) fn reset_for_test_control(&mut self) -> ApiRouteResponse {
        if !self.test_controls_enabled {
            return ApiRouteResponse::not_found();
        }
        let agent_backend_execution = self.agent_backend_execution.clone();
        let runtime_verification = self.runtime_verification_for_test.clone();
        let local_model_inventory = self.local_model_inventory_for_test.clone();
        let host_memory_gb = self.host_memory_gb_for_test;
        let model_download_execution = self.model_download_execution;
        *self = Self::default();
        self.agent_backend_execution = preserve_agent_backend(agent_backend_execution);
        self.legacy_agent_test_harness_enabled = matches!(
            self.agent_backend_execution,
            AgentBackendExecutionMode::DeterministicForTest(_)
                | AgentBackendExecutionMode::DeterministicSequenceForTest(_)
                | AgentBackendExecutionMode::FailForTest
        );
        self.runtime_verification_for_test = runtime_verification;
        self.local_model_inventory_for_test = local_model_inventory;
        self.host_memory_gb_for_test = host_memory_gb;
        self.model_download_execution = model_download_execution;
        self.test_controls_enabled = true;
        ApiRouteResponse::ok(json!({
            "source":"dev_test_control",
            "state":"reset"
        }))
    }

    pub(crate) fn agent_backend_for_test_control(&mut self, body: &str) -> ApiRouteResponse {
        if !self.test_controls_enabled {
            return ApiRouteResponse::not_found();
        }
        let value = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
        if value
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|mode| mode == "fail")
        {
            self.legacy_agent_test_harness_enabled = true;
            self.agent_backend_execution = AgentBackendExecutionMode::FailForTest;
            return ApiRouteResponse::ok(json!({"source":"dev_test_control","state":"fail"}));
        }
        if value
            .get("mode")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|mode| mode == "native_iterative")
        {
            let outputs = value
                .get("outputs")
                .and_then(serde_json::Value::as_array)
                .map(|outputs| {
                    outputs
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            self.agent_backend_execution =
                AgentBackendExecutionMode::NativeIterativeSequenceForTest(outputs);
            self.legacy_agent_test_harness_enabled = false;
            return ApiRouteResponse::ok(json!({
                "source":"dev_test_control",
                "state":"native_iterative"
            }));
        }
        let output = value
            .get("output")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("Deterministic agent response.");
        self.agent_backend_execution =
            AgentBackendExecutionMode::DeterministicForTest(output.to_string());
        self.legacy_agent_test_harness_enabled = true;
        ApiRouteResponse::ok(json!({
            "source":"dev_test_control",
            "state":"deterministic"
        }))
    }

    pub(crate) fn model_protocol_for_test_control(&mut self, body: &str) -> ApiRouteResponse {
        if !self.test_controls_enabled {
            return ApiRouteResponse::not_found();
        }
        let value = serde_json::from_str::<serde_json::Value>(body).unwrap_or_default();
        let Some(model_id) = value.get("modelId").and_then(serde_json::Value::as_str) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"MODEL_ID_REQUIRED",
                "message":"modelId is required for the dev protocol control."
            }));
        };
        let Some(pull_ref) = crate::model_routes::model_pull_ref(model_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"UNKNOWN_MODEL",
                "message":"The dev protocol control only accepts catalog models."
            }));
        };
        self.mark_ollama_model_capabilities_for_test(&pull_ref, &["completion", "tools"]);
        ApiRouteResponse::ok(json!({
            "source":"dev_test_control",
            "state":"certified",
            "modelId":model_id,
            "runtimeModelId":pull_ref
        }))
    }
}

fn preserve_agent_backend(mode: AgentBackendExecutionMode) -> AgentBackendExecutionMode {
    match mode {
        AgentBackendExecutionMode::Execute => AgentBackendExecutionMode::Execute,
        AgentBackendExecutionMode::DeterministicForTest(output) => {
            AgentBackendExecutionMode::DeterministicForTest(output)
        }
        AgentBackendExecutionMode::DeterministicSequenceForTest(outputs) => {
            AgentBackendExecutionMode::DeterministicSequenceForTest(outputs)
        }
        AgentBackendExecutionMode::NativeIterativeSequenceForTest(outputs) => {
            AgentBackendExecutionMode::NativeIterativeSequenceForTest(outputs)
        }
        AgentBackendExecutionMode::FailForTest => AgentBackendExecutionMode::FailForTest,
    }
}

#[cfg(test)]
mod tests {
    use super::super::{AgentBackendExecutionMode, LocalApiRouter, ModelDownloadExecutionMode};

    #[test]
    fn reset_preserves_test_executor_configuration_without_preserving_product_state() {
        let mut router = LocalApiRouter::default();
        router.enable_test_controls_for_dev_server();
        router.set_runtime_verification_for_test(false, "fixture runtime evidence");
        router.set_local_model_inventory_for_test(&["fixture/model:latest"]);
        router.set_host_memory_gb_for_test(384);
        router.complete_model_downloads_for_test();
        router.complete_agent_backend_for_test("fixture agent output");
        router.mark_runtime_verified_for_test("runtime.ollama", "product readiness");

        let response = router.reset_for_test_control();

        assert_eq!(response.status(), "200 OK");
        assert!(!router.readiness.runtime_verified());
        assert_eq!(
            router
                .runtime_verification_for_test
                .as_ref()
                .unwrap()
                .evidence,
            "fixture runtime evidence"
        );
        assert_eq!(
            router.local_model_inventory_for_test,
            Some(vec!["fixture/model:latest".to_string()])
        );
        assert_eq!(router.host_memory_gb_for_test, Some(384));
        assert_eq!(
            router.model_download_execution,
            ModelDownloadExecutionMode::CompleteForTest
        );
        assert_eq!(
            router.agent_backend_execution,
            AgentBackendExecutionMode::DeterministicForTest("fixture agent output".to_string())
        );
    }

    #[test]
    fn dev_control_can_select_the_native_iterative_loop() {
        let mut router = LocalApiRouter::default();
        router.enable_test_controls_for_dev_server();

        let response = router.agent_backend_for_test_control(
            r#"{"mode":"native_iterative","outputs":["{\"tool\":\"desktoplab.complete\",\"arguments\":{\"message\":\"Done.\",\"outcome\":\"answered\",\"evidenceCallIds\":[]}}"]}"#,
        );

        assert_eq!(response.status(), "200 OK");
        assert!(!router.legacy_agent_test_harness_enabled);
        assert!(matches!(
            router.agent_backend_execution,
            AgentBackendExecutionMode::NativeIterativeSequenceForTest(ref outputs)
                if outputs.len() == 1
        ));
    }

    #[test]
    fn dev_control_certifies_only_catalog_model_protocols_after_explicit_opt_in() {
        let catalog = desktoplab_model_manager::ModelManager::new().default_family_catalog();
        let variant = catalog
            .variants()
            .first()
            .expect("default agent catalog should contain a model");
        let body = serde_json::json!({"modelId":variant.model_id()}).to_string();
        let mut disabled = LocalApiRouter::default();
        assert_eq!(
            disabled
                .model_protocol_for_test_control(&body)
                .status(),
            "404 Not Found"
        );

        let mut enabled = LocalApiRouter::default();
        enabled.enable_test_controls_for_dev_server();
        let unknown =
            enabled.model_protocol_for_test_control(r#"{"modelId":"model.not-in-catalog"}"#);
        assert_eq!(unknown.status(), "400 Bad Request");
        let certified = enabled.model_protocol_for_test_control(&body);

        assert_eq!(certified.status(), "200 OK");
        let capabilities = enabled
            .readiness
            .model_capabilities()
            .expect("dev control should record model capabilities");
        assert_eq!(
            capabilities.model_id(),
            variant.runtime_compatibility().pull_ref()
        );
        assert!(capabilities.tool_protocol_certified());
    }
}
