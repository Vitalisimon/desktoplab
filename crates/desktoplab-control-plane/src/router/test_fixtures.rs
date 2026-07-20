use desktoplab_vault::FakeVault;

use super::{
    AgentBackendExecutionMode, LocalApiRouter, ModelDownloadExecutionMode,
    OpenAiCodexDeviceFixture, RuntimeVerificationFixture,
};

impl LocalApiRouter {
    pub fn agent_execution_binding_for_test(&self, session_id: &str) -> Option<serde_json::Value> {
        self.agent_execution_bindings
            .get(session_id)
            .map(super::agent_execution_binding::AgentExecutionBinding::to_json)
    }

    pub fn use_fake_openai_codex_native_vault_for_test(&mut self) {
        self.openai_codex_native_vault_for_test = Some(FakeVault::default());
    }

    pub fn store_openai_codex_native_secret_for_test(&mut self, vault_ref: &str, secret: &str) {
        let secret_ref =
            desktoplab_vault::SecretRef::from_uri(vault_ref).expect("vault ref should parse");
        let vault = self
            .openai_codex_native_vault_for_test
            .get_or_insert_with(FakeVault::default);
        desktoplab_vault::Vault::put(
            vault,
            secret_ref,
            desktoplab_vault::SecretValue::new(secret),
        )
        .expect("fake vault should store secret");
    }

    pub fn openai_codex_native_secret_for_test(&self, vault_ref: &str) -> Option<String> {
        let secret_ref = desktoplab_vault::SecretRef::from_uri(vault_ref).ok()?;
        let vault = self.openai_codex_native_vault_for_test.as_ref()?;
        desktoplab_vault::Vault::get(vault, &secret_ref)
            .ok()
            .map(|secret| secret.expose_for_adapter().to_string())
    }

    pub fn authorize_openai_codex_device_for_test(
        &mut self,
        device_auth_id: &str,
        user_code: &str,
        authorization_code: &str,
        code_verifier: &str,
    ) {
        self.openai_codex_device_authorization_for_test = Some(OpenAiCodexDeviceFixture {
            device_auth_id: device_auth_id.to_string(),
            user_code: user_code.to_string(),
            authorization_code: authorization_code.to_string(),
            code_verifier: code_verifier.to_string(),
        });
    }

    pub fn mark_runtime_verified_for_test(&mut self, runtime_id: &str, evidence: &str) {
        self.readiness
            .mark_runtime_verified(runtime_id.to_string(), evidence.to_string());
        self.persist_readiness_state();
    }

    pub fn set_runtime_verification_for_test(&mut self, verified: bool, evidence: &str) {
        self.runtime_verification_for_test = Some(RuntimeVerificationFixture {
            verified,
            evidence: evidence.to_string(),
            blocked_reason: "runtime_not_detected".to_string(),
        });
    }

    pub fn mark_model_verified_for_test(
        &mut self,
        runtime_id: &str,
        model_id: &str,
        evidence: &str,
    ) {
        self.mark_model_verified_without_capabilities_for_test(runtime_id, model_id, evidence);
        if runtime_id == "runtime.ollama"
            && let Some(pull_ref) = crate::model_routes::model_pull_ref(model_id)
        {
            self.mark_ollama_model_capabilities_for_test(&pull_ref, &["completion", "tools"]);
        }
    }

    pub fn mark_model_verified_without_capabilities_for_test(
        &mut self,
        runtime_id: &str,
        model_id: &str,
        evidence: &str,
    ) {
        self.readiness.mark_model_verified(
            runtime_id.to_string(),
            model_id.to_string(),
            evidence.to_string(),
        );
        self.selected_route_id = crate::execution_routes::local_route_id(model_id);
        self.stability.mark_route_decision();
        self.persist_readiness_state();
        self.persist_selected_route_id();
    }

    pub fn mark_ollama_model_capabilities_for_test(
        &mut self,
        runtime_model_id: &str,
        capabilities: &[&str],
    ) {
        self.mark_ollama_model_capabilities_with_protocol_for_test(
            runtime_model_id,
            capabilities,
            desktoplab_backends::ModelToolProtocolKind::NativeTools,
        );
    }

    pub fn mark_ollama_model_capabilities_with_protocol_for_test(
        &mut self,
        runtime_model_id: &str,
        capabilities: &[&str],
        protocol: desktoplab_backends::ModelToolProtocolKind,
    ) {
        let mut profile = desktoplab_backends::BackendModelCapabilities::reported(
            "backend.ollama",
            runtime_model_id,
            Some("test-digest".to_string()),
            Some(32_768),
            capabilities.iter().copied(),
        );
        if capabilities.contains(&"tools") {
            let certification = desktoplab_backends::ModelToolProtocolCertification::certified_as(
                profile.fingerprint(),
                protocol,
            );
            profile = profile.with_tool_protocol_certification(certification);
        }
        self.readiness.mark_model_capabilities(profile);
        self.persist_readiness_state();
    }

    pub fn plan_model_downloads_for_test(&mut self) {
        self.model_download_execution = ModelDownloadExecutionMode::PlanOnlyForTest;
    }

    pub fn complete_model_downloads_for_test(&mut self) {
        self.model_download_execution = ModelDownloadExecutionMode::CompleteForTest;
    }

    pub fn set_local_model_inventory_for_test(&mut self, models: &[&str]) {
        self.local_model_inventory_for_test =
            Some(models.iter().map(ToString::to_string).collect());
    }

    pub fn set_host_memory_gb_for_test(&mut self, memory_gb: u32) {
        self.host_memory_gb_for_test = Some(memory_gb);
    }

    pub fn create_retryable_job_for_test(&mut self, kind: &str) -> String {
        let job = self.jobs.create_job(kind);
        let job_id = job.id().clone();
        let _ = self.jobs.fail(
            &job_id,
            desktoplab_backend_services::JobRetryClass::Retryable,
        );
        job_id.as_str().to_string()
    }

    pub fn complete_agent_backend_for_test(&mut self, output: impl Into<String>) {
        if cfg!(debug_assertions) {
            self.legacy_agent_test_harness_enabled = true;
            self.agent_backend_execution =
                AgentBackendExecutionMode::DeterministicForTest(output.into());
        }
    }

    pub fn complete_agent_backend_sequence_for_test(
        &mut self,
        outputs: impl IntoIterator<Item = impl Into<String>>,
    ) {
        if cfg!(debug_assertions) {
            self.legacy_agent_test_harness_enabled = true;
            self.agent_backend_execution = AgentBackendExecutionMode::DeterministicSequenceForTest(
                outputs.into_iter().map(Into::into).collect(),
            );
        }
    }

    pub fn complete_native_iterative_backend_sequence_for_test(
        &mut self,
        outputs: impl IntoIterator<Item = impl Into<String>>,
    ) {
        if cfg!(debug_assertions) {
            self.legacy_agent_test_harness_enabled = false;
            self.agent_backend_execution =
                AgentBackendExecutionMode::NativeIterativeSequenceForTest(
                    outputs.into_iter().map(Into::into).collect(),
                );
        }
    }

    pub fn fail_agent_backend_for_test(&mut self) {
        if cfg!(debug_assertions) {
            self.legacy_agent_test_harness_enabled = true;
            self.agent_backend_execution = AgentBackendExecutionMode::FailForTest;
        }
    }

    pub fn set_agent_model_delay_for_test(&mut self, delay: std::time::Duration) {
        if cfg!(debug_assertions) {
            self.agent_model_delay_for_test = Some(delay);
        }
    }
}
