use desktoplab_backends::BackendMessage;

#[cfg(debug_assertions)]
use super::AgentBackendExecutionMode;
use super::LocalApiRouter;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_model_execution::PreparedAgentModelExecution;

impl LocalApiRouter {
    pub(super) fn prepare_agent_model_execution(
        &mut self,
        backend_id: &str,
        binding: Option<&AgentExecutionBinding>,
        messages: Vec<BackendMessage>,
        suppressed_tool: Option<&str>,
    ) -> PreparedAgentModelExecution {
        #[cfg(debug_assertions)]
        if let AgentBackendExecutionMode::NativeIterativeSequenceForTest(outputs) =
            &mut self.agent_backend_execution
        {
            return outputs
                .is_empty()
                .then(|| PreparedAgentModelExecution::Failed("test_backend_exhausted".to_string()))
                .unwrap_or_else(|| PreparedAgentModelExecution::Fixture {
                    output: outputs.remove(0),
                    delay: self.agent_model_delay_for_test.unwrap_or_default(),
                });
        }
        let Some(binding) = binding else {
            return PreparedAgentModelExecution::Failed(
                "session_execution_binding_missing".to_string(),
            );
        };
        if binding.backend_id() != backend_id {
            return PreparedAgentModelExecution::Failed(
                "session_execution_binding_mismatch".to_string(),
            );
        }
        match backend_id {
            "backend.ollama" => {
                self.prepare_ollama_model_execution(binding, messages, suppressed_tool)
            }
            "backend.codex" => {
                self.prepare_codex_model_execution(binding, messages, suppressed_tool)
            }
            "backend.mlx-lm" => {
                self.prepare_mlx_model_execution(binding, messages, suppressed_tool)
            }
            "backend.lm-studio" => {
                self.prepare_lm_studio_model_execution(binding, messages, suppressed_tool)
            }
            "backend.high-end-local" => {
                self.prepare_high_end_model_execution(binding, messages, suppressed_tool)
            }
            _ => PreparedAgentModelExecution::Failed(
                "backend_native_tool_history_unsupported".to_string(),
            ),
        }
    }
}
