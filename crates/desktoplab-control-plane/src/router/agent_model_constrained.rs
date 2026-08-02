use desktoplab_backends::{BackendMessage, OpenAiCodexResponderCommandPayload};

use super::LocalApiRouter;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_model_execution::PreparedAgentModelExecution;

impl LocalApiRouter {
    pub(super) fn prepare_codex_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> PreparedAgentModelExecution {
        match self.codex_agent_execution_request_from_binding(binding, messages, suppressed_tool) {
            Ok((responder_url, payload)) => PreparedAgentModelExecution::Codex {
                responder_url,
                payload,
            },
            Err(error) => PreparedAgentModelExecution::Failed(error),
        }
    }

    fn codex_agent_execution_request_from_binding(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> Result<(String, OpenAiCodexResponderCommandPayload), String> {
        if binding.provider_id() != Some("provider.openai")
            || !matches!(
                binding.account_mode(),
                Some("subscription_account" | "local_app_session" | "oauth_device")
            )
        {
            return Err("session_codex_account_binding_missing".to_string());
        }
        let vault_ref = binding
            .vault_ref()
            .ok_or_else(|| "session_codex_vault_binding_missing".to_string())?;
        if !self.codex_credential_available(vault_ref) {
            return Err("codex_credential_unavailable".to_string());
        }
        let responder_url = binding
            .endpoint()
            .ok_or_else(|| "session_codex_responder_binding_missing".to_string())?;
        let payload = OpenAiCodexResponderCommandPayload::for_agent_turn(
            messages,
            self.backend_tool_schemas_excluding(suppressed_tool)?,
            vault_ref,
            binding.vault_kind().unwrap_or("native_vault"),
        )?;
        Ok((responder_url.to_string(), payload))
    }

    pub(super) fn codex_agent_execution_request(
        &self,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> Result<(String, OpenAiCodexResponderCommandPayload), String> {
        let account = self
            .provider_accounts
            .get("provider.openai")
            .filter(|account| account.is_codex_bridge_ready())
            .ok_or_else(|| "codex_bridge_not_ready".to_string())?;
        let vault_ref = account
            .vault_ref()
            .ok_or_else(|| "codex_vault_reference_missing".to_string())?;
        if !self.codex_credential_available(vault_ref) {
            return Err("codex_credential_unavailable".to_string());
        }
        let responder_url = account
            .bridge_responder_url()
            .ok_or_else(|| "codex_responder_unavailable".to_string())?
            .to_string();
        let payload = OpenAiCodexResponderCommandPayload::for_agent_turn(
            messages,
            self.backend_tool_schemas_excluding(suppressed_tool)?,
            vault_ref,
            account.vault_kind().unwrap_or("native_vault"),
        )?;
        Ok((responder_url, payload))
    }

    pub(super) fn prepare_mlx_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> PreparedAgentModelExecution {
        let result = (|| -> Result<_, String> {
            let bound_model_id = binding
                .model_id()
                .ok_or_else(|| "session_model_binding_missing".to_string())?;
            crate::model_routes::model_pull_ref(&bound_model_id)
                .ok_or_else(|| "local_model_pull_reference_missing".to_string())?;
            let (backend, served_model) = self.mlx_lm_backend_and_model()?;
            let prompt = self.local_openai_compatible_prompt(
                bound_model_id,
                served_model,
                messages,
                suppressed_tool,
            )?;
            Ok(PreparedAgentModelExecution::Mlx { backend, prompt })
        })();
        result.unwrap_or_else(PreparedAgentModelExecution::Failed)
    }
}
