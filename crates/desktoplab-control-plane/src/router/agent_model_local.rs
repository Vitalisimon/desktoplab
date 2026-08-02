use desktoplab_backends::{BackendMessage, BackendPrompt};

use super::LocalApiRouter;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_model_execution::PreparedAgentModelExecution;

impl LocalApiRouter {
    pub(super) fn local_openai_compatible_prompt(
        &self,
        model_id: &str,
        served_model: impl Into<String>,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> Result<BackendPrompt, String> {
        let available_memory_gb = self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb);
        let context_window_tokens =
            crate::model_routes::agent_context_window_tokens(model_id, available_memory_gb)
                .ok_or_else(|| "local_model_context_window_unavailable".to_string())?;
        let request_timeout_seconds =
            crate::model_routes::agent_request_timeout_seconds(model_id, available_memory_gb)
                .ok_or_else(|| "local_model_request_timeout_unavailable".to_string())?;
        Ok(BackendPrompt::new(served_model, "")
            .with_messages(messages)
            .with_tools(self.backend_tool_schemas_excluding(suppressed_tool)?)
            .with_context_window_tokens(context_window_tokens)
            .with_request_timeout_seconds(request_timeout_seconds))
    }

    pub(super) fn mlx_lm_backend_and_model(
        &self,
    ) -> Result<(desktoplab_backends::LmStudioExecutionBackend, String), String> {
        let status = crate::runtime_routes::mlx_lm_managed::status()
            .filter(desktoplab_runtime::MlxLmManagedStatus::ready)
            .ok_or_else(|| "mlx_lm_managed_runtime_unavailable".to_string())?;
        let model = status
            .models()
            .first()
            .cloned()
            .ok_or_else(|| "mlx_lm_model_unavailable".to_string())?;
        let backend = desktoplab_backends::LmStudioExecutionBackend::new(
            desktoplab_backends::LocalEndpoint::available(status.endpoint()),
            desktoplab_backends::BackendModelInventory::available(&[model.as_str()]),
        );
        Ok((backend, model))
    }

    pub(super) fn prepare_ollama_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> PreparedAgentModelExecution {
        let result = (|| -> Result<_, String> {
            let model_id = binding
                .model_id()
                .ok_or_else(|| "session_model_binding_missing".to_string())?;
            let model = crate::model_routes::model_pull_ref(&model_id)
                .ok_or_else(|| "local_model_pull_reference_missing".to_string())?;
            let expected = self
                .readiness
                .model_capabilities()
                .filter(|_| self.readiness.model_id() == Some(model_id))
                .cloned()
                .ok_or_else(|| "session_model_configuration_changed".to_string())?;
            let tools = self.backend_tool_schemas_excluding(suppressed_tool)?;
            let context_window_tokens = crate::model_routes::agent_context_window_tokens(
                &model_id,
                self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
            )
            .ok_or_else(|| "local_model_context_window_unavailable".to_string())?;
            let request_timeout_seconds = crate::model_routes::agent_request_timeout_seconds(
                &model_id,
                self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
            )
            .ok_or_else(|| "local_model_request_timeout_unavailable".to_string())?;
            let prompt = BackendPrompt::new(model, "")
                .with_messages(messages)
                .with_tools(tools)
                .with_context_window_tokens(context_window_tokens)
                .with_request_timeout_seconds(request_timeout_seconds);
            Ok(PreparedAgentModelExecution::Ollama {
                resolver: self.ollama_model_capabilities.clone(),
                expected,
                prompt,
            })
        })();
        result.unwrap_or_else(PreparedAgentModelExecution::Failed)
    }

    pub(super) fn prepare_high_end_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> PreparedAgentModelExecution {
        let Some(runtime) = self.high_end_runtime.as_ref() else {
            return PreparedAgentModelExecution::Failed("high_end_runtime_unavailable".to_string());
        };
        let Some(model) = binding.model_id().map(str::to_string) else {
            return PreparedAgentModelExecution::Failed(
                "session_model_binding_missing".to_string(),
            );
        };
        let Some(endpoint) = binding.endpoint() else {
            return PreparedAgentModelExecution::Failed(
                "session_endpoint_binding_missing".to_string(),
            );
        };
        if runtime.endpoint().model_id() != model || runtime.endpoint().base_url() != endpoint {
            return PreparedAgentModelExecution::Failed(
                "session_model_configuration_changed".to_string(),
            );
        }
        let tools = match self.backend_tool_schemas_excluding(suppressed_tool) {
            Ok(tools) => tools,
            Err(error) => return PreparedAgentModelExecution::Failed(error),
        };
        let prompt = BackendPrompt::new(&model, "")
            .with_messages(messages)
            .with_tools(tools);
        PreparedAgentModelExecution::HighEnd {
            backend: desktoplab_backends::OpenAiCompatibleLocalExecutionBackend::new(
                "backend.high-end-local",
                desktoplab_backends::LocalEndpoint::available(endpoint),
                desktoplab_backends::BackendModelInventory::available(&[&model]),
            ),
            prompt,
        }
    }
}

#[cfg(test)]
#[path = "agent_model_local_tests.rs"]
mod tests;
