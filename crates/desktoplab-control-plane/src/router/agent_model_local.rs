use desktoplab_backends::{BackendMessage, BackendPrompt};

use super::LocalApiRouter;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_model_execution::PreparedAgentModelExecution;

impl LocalApiRouter {
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
        PreparedAgentModelExecution::LmStudio {
            backend: desktoplab_backends::LmStudioExecutionBackend::new(
                desktoplab_backends::LocalEndpoint::available("http://127.0.0.1:1234"),
                desktoplab_backends::BackendModelInventory::available(&[&model]),
            ),
            prompt,
        }
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
mod tests {
    use desktoplab_backends::BackendMessage;

    use super::{AgentExecutionBinding, LocalApiRouter, PreparedAgentModelExecution};

    #[test]
    fn lm_studio_execution_uses_the_model_bound_to_the_session() {
        let mut router = LocalApiRouter::default();
        router.selected_route_id = crate::execution_routes::local_route_id("model.gemma4-12b-q4");
        let binding = AgentExecutionBinding::capture(&router, "backend.lm-studio");
        router.selected_route_id = crate::execution_routes::local_route_id("model.qwen3.5-9b-q4");

        let execution =
            router.prepare_lm_studio_model_execution(&binding, vec![BackendMessage::user("test")]);

        let PreparedAgentModelExecution::LmStudio { prompt, .. } = execution else {
            panic!("session-bound LM Studio execution should be prepared");
        };
        assert_eq!(prompt.model(), "gemma4:12b");
    }

    #[test]
    fn ollama_execution_fails_if_readiness_moved_to_another_model() {
        let mut router = LocalApiRouter::default();
        router.selected_route_id = crate::execution_routes::local_route_id("model.gemma4-12b-q4");
        let binding = AgentExecutionBinding::capture(&router, "backend.ollama");
        router.readiness = router
            .readiness
            .clone()
            .select("runtime.ollama", "model.qwen3.5-9b-q4");
        router.readiness.mark_model_capabilities(
            desktoplab_backends::BackendModelCapabilities::reported(
                "backend.ollama",
                "qwen3.5:9b",
                None,
                Some(32_768),
                ["tools"],
            ),
        );

        let execution =
            router.prepare_ollama_model_execution(&binding, vec![BackendMessage::user("test")]);

        let PreparedAgentModelExecution::Failed(reason) = execution else {
            panic!("changed model readiness must fail closed");
        };
        assert_eq!(reason, "session_model_configuration_changed");
    }

    #[test]
    fn ollama_execution_uses_the_wizard_memory_budget_for_the_bound_model() {
        let mut router = LocalApiRouter::default();
        router.set_host_memory_gb_for_test(36);
        router.selected_route_id = crate::execution_routes::local_route_id("model.gemma4-12b-q4");
        router.readiness = router
            .readiness
            .clone()
            .select("runtime.ollama", "model.gemma4-12b-q4");
        router.readiness.mark_model_capabilities(
            desktoplab_backends::BackendModelCapabilities::reported(
                "backend.ollama",
                "gemma4:12b",
                None,
                Some(256_000),
                ["tools"],
            ),
        );
        let binding = AgentExecutionBinding::capture(&router, "backend.ollama");

        let execution =
            router.prepare_ollama_model_execution(&binding, vec![BackendMessage::user("test")]);

        let PreparedAgentModelExecution::Ollama { prompt, .. } = execution else {
            panic!("configured Ollama execution should be prepared");
        };
        assert_eq!(prompt.context_window_tokens(), Some(65_536));
        assert_eq!(prompt.request_timeout_seconds(), Some(240));
    }
}
