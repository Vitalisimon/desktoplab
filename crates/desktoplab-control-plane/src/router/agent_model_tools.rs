use std::path::Path;
use std::time::{Duration, Instant};

use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeLoopState, IterativeLoopStatus, IterativeModelDecision,
};

use crate::agent_model_adapter::{backend_messages, decision_from_backend_output_with_registry};
use crate::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};

use super::LocalApiRouter;
use super::agent_model_execution::apply_model_execution_error;
use super::agent_subagent_tools::RouterAgentToolExecutor;

const IMMEDIATE_LOOP_LIMIT: Duration = Duration::from_secs(300);

impl LocalApiRouter {
    pub(super) fn run_native_iterative_until_pause(
        &mut self,
        state: &mut IterativeLoopState,
        compiled_prompt: &str,
        workspace_id: &str,
        backend_id: &str,
    ) -> Result<(), String> {
        let started_at = Instant::now();
        let agent_loop = IterativeAgentLoop::default();
        while state.status() == IterativeLoopStatus::Running {
            if started_at.elapsed() >= IMMEDIATE_LOOP_LIMIT {
                agent_loop.fail_model_turn(state, "agent_loop_duration_exhausted");
                break;
            }
            if !agent_loop.begin_model_turn(state) {
                break;
            }
            let registry = self.agent_tool_registry()?;
            let output = self.run_selected_backend_messages(
                backend_id,
                backend_messages(compiled_prompt, state, &registry),
            );
            let decision = match output {
                Ok(output) => Some(decision_from_backend_output_with_registry(
                    state, &output, registry,
                )),
                Err(error) => {
                    apply_model_execution_error(state, &agent_loop, error);
                    None
                }
            };
            let Some(decision) = decision else {
                continue;
            };
            match decision {
                Ok(decision) => {
                    state.clear_model_protocol_recovery();
                    self.apply_router_agent_decision(state, workspace_id, decision)?;
                }
                Err(error) => {
                    if !state.request_model_protocol_retry(error.clone()) {
                        agent_loop.fail_model_turn(state, format!("model_protocol_error:{error}"));
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn apply_router_agent_decision(
        &mut self,
        state: &mut IterativeLoopState,
        workspace_id: &str,
        decision: IterativeModelDecision,
    ) -> Result<(), String> {
        let session_id = state.session_id().to_string();
        let workspace = self
            .execution_workspace_record(&session_id)
            .ok_or_else(|| "execution_workspace_unavailable".to_string())?;
        let approval_mode = self
            .agent_execution_bindings
            .get(&session_id)
            .map(|binding| binding.approval_mode())
            .ok_or_else(|| "session_execution_binding_missing".to_string())?;
        let canonical = CanonicalAgentToolExecutor::new(
            Path::new(&workspace.root_path),
            workspace_id,
            &session_id,
            CanonicalExecutionApproval::Pending,
        )
        .with_approval_mode(approval_mode)
        .with_process_registry(self.agent_process_registry.clone())
        .with_mcp_runtime(self.mcp_runtime.clone())?;
        let mut executor = RouterAgentToolExecutor::new(self, canonical, &session_id);
        IterativeAgentLoop::default().apply_model_decision(state, &mut executor, decision);
        Ok(())
    }
}
