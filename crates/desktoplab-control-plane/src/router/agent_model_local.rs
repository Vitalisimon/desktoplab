use desktoplab_backends::{BackendMessage, BackendPrompt};

use super::LocalApiRouter;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_model_execution::PreparedAgentModelExecution;

impl LocalApiRouter {
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
            let tools = self.backend_tool_schemas()?;
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

    pub(super) fn prepare_lm_studio_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
    ) -> PreparedAgentModelExecution {
        let discovery = desktoplab_runtime::discover_system_lm_studio();
        self.prepare_lm_studio_model_execution_with_discovery(binding, messages, &discovery)
    }

    fn prepare_lm_studio_model_execution_with_discovery(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        discovery: &desktoplab_runtime::LmStudioExistingDiscovery,
    ) -> PreparedAgentModelExecution {
        let Some(model_id) = binding.model_id().map(str::to_string) else {
            return PreparedAgentModelExecution::Failed("local_model_unavailable".to_string());
        };
        let model = crate::model_routes::model_pull_ref(&model_id).unwrap_or(model_id);
        let tools = match self.backend_tool_schemas() {
            Ok(tools) => tools,
            Err(error) => return PreparedAgentModelExecution::Failed(error),
        };
        let prompt = BackendPrompt::new(model.clone(), "")
            .with_messages(messages)
            .with_tools(tools);
        let backend = match self.lm_studio_backend_for_model_with_discovery(&model, discovery) {
            Ok(backend) => backend,
            Err(error) => return PreparedAgentModelExecution::Failed(error),
        };
        PreparedAgentModelExecution::LmStudio { backend, prompt }
    }

    pub(super) fn lm_studio_backend_for_model(
        &self,
        model: &str,
    ) -> Result<desktoplab_backends::LmStudioExecutionBackend, String> {
        let discovery = desktoplab_runtime::discover_system_lm_studio();
        self.lm_studio_backend_for_model_with_discovery(model, &discovery)
    }

    fn lm_studio_backend_for_model_with_discovery(
        &self,
        model: &str,
        discovery: &desktoplab_runtime::LmStudioExistingDiscovery,
    ) -> Result<desktoplab_backends::LmStudioExecutionBackend, String> {
        let endpoint = discovery
            .endpoint()
            .filter(|_| discovery.ready())
            .ok_or_else(|| format!("lm_studio_{}", discovery.state().as_str()))?;
        if !discovery
            .models()
            .iter()
            .any(|candidate| candidate == model)
        {
            return Err("lm_studio_model_unavailable".to_string());
        }
        let models = discovery
            .models()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        Ok(desktoplab_backends::LmStudioExecutionBackend::new(
            desktoplab_backends::LocalEndpoint::available(endpoint),
            desktoplab_backends::BackendModelInventory::available(&models),
        ))
    }

    pub(super) fn prepare_high_end_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
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
        let tools = match self.backend_tool_schemas() {
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
