use desktoplab_backends::{BackendMessage, BackendPrompt};

use super::LocalApiRouter;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_model_execution::PreparedAgentModelExecution;

impl LocalApiRouter {
    pub(super) fn prepare_lm_studio_model_execution(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
    ) -> PreparedAgentModelExecution {
        if let Some((endpoint, models)) = crate::runtime_routes::managed_lm_studio_connection() {
            return self.prepare_lm_studio_model_execution_with_inventory(
                binding, messages, &endpoint, &models,
            );
        }
        let discovery = desktoplab_runtime::discover_system_lm_studio();
        self.prepare_lm_studio_model_execution_with_discovery(binding, messages, &discovery)
    }

    pub(super) fn prepare_lm_studio_model_execution_with_discovery(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        discovery: &desktoplab_runtime::LmStudioExistingDiscovery,
    ) -> PreparedAgentModelExecution {
        let Some(endpoint) = discovery.endpoint().filter(|_| discovery.ready()) else {
            return PreparedAgentModelExecution::Failed(format!(
                "lm_studio_{}",
                discovery.state().as_str()
            ));
        };
        self.prepare_lm_studio_model_execution_with_inventory(
            binding,
            messages,
            endpoint,
            discovery.models(),
        )
    }

    pub(super) fn prepare_lm_studio_model_execution_with_inventory(
        &self,
        binding: &AgentExecutionBinding,
        messages: Vec<BackendMessage>,
        endpoint: &str,
        models: &[String],
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
        let backend = match self.lm_studio_backend_for_inventory(&model, endpoint, models) {
            Ok(backend) => backend,
            Err(error) => return PreparedAgentModelExecution::Failed(error),
        };
        PreparedAgentModelExecution::LmStudio { backend, prompt }
    }

    pub(super) fn lm_studio_backend_for_model(
        &self,
        model: &str,
    ) -> Result<desktoplab_backends::LmStudioExecutionBackend, String> {
        if let Some((endpoint, models)) = crate::runtime_routes::managed_lm_studio_connection() {
            return self.lm_studio_backend_for_inventory(model, &endpoint, &models);
        }
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
        self.lm_studio_backend_for_inventory(model, endpoint, discovery.models())
    }

    fn lm_studio_backend_for_inventory(
        &self,
        model: &str,
        endpoint: &str,
        models: &[String],
    ) -> Result<desktoplab_backends::LmStudioExecutionBackend, String> {
        if !models.iter().any(|candidate| candidate == model) {
            return Err("lm_studio_model_unavailable".to_string());
        }
        let models = models.iter().map(String::as_str).collect::<Vec<_>>();
        Ok(desktoplab_backends::LmStudioExecutionBackend::new(
            desktoplab_backends::LocalEndpoint::available(endpoint),
            desktoplab_backends::BackendModelInventory::available(&models),
        ))
    }
}
