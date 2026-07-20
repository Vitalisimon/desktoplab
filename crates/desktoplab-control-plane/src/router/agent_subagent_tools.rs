use desktoplab_agent_engine::{
    AgentToolExecutionOwner, DesktopLabToolRegistry, IterativeToolCall, IterativeToolExecutor,
    ProviderToolCallNormalizer, ToolObservation,
};
use desktoplab_backend_services::AuditAction;
use desktoplab_policy::{Action, DecisionOutcome, PolicyEngine};
use serde_json::{Value, json};

use crate::CanonicalAgentToolExecutor;

use super::{ApiRouteResponse, LocalApiRouter};

pub(super) struct RouterAgentToolExecutor<'a> {
    router: &'a mut LocalApiRouter,
    canonical: CanonicalAgentToolExecutor,
    parent_session_id: String,
}

impl<'a> RouterAgentToolExecutor<'a> {
    pub(super) fn new(
        router: &'a mut LocalApiRouter,
        canonical: CanonicalAgentToolExecutor,
        parent_session_id: impl Into<String>,
    ) -> Self {
        Self {
            router,
            canonical,
            parent_session_id: parent_session_id.into(),
        }
    }
}

impl IterativeToolExecutor for RouterAgentToolExecutor<'_> {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        let owner = execution_owner(call.name())?;
        let observation = match owner {
            AgentToolExecutionOwner::CanonicalGateway => self.canonical.execute(call),
            AgentToolExecutionOwner::RouterControl if call.name() == "desktoplab.update_plan" => {
                self.router
                    .execute_agent_plan_tool(&self.parent_session_id, call)
            }
            AgentToolExecutionOwner::RouterControl => self
                .router
                .execute_subagent_tool(&self.parent_session_id, call),
            AgentToolExecutionOwner::LoopControl => Err("loop_control_tool_not_executable".into()),
        }?;
        validate_router_output(owner, call, observation)
    }

    fn execute_approved(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        match execution_owner(call.name())? {
            AgentToolExecutionOwner::CanonicalGateway => self.canonical.execute_approved(call),
            AgentToolExecutionOwner::RouterControl => self.execute(call),
            AgentToolExecutionOwner::LoopControl => Err("loop_control_tool_not_executable".into()),
        }
    }
}

fn validate_router_output(
    owner: AgentToolExecutionOwner,
    call: &IterativeToolCall,
    observation: ToolObservation,
) -> Result<ToolObservation, String> {
    if owner == AgentToolExecutionOwner::RouterControl && observation.error().is_none() {
        ProviderToolCallNormalizer::default()
            .validate_output(call.name(), observation.output())
            .map_err(|error| format!("tool_output_contract_violation:{error}"))?;
    }
    Ok(observation)
}

impl LocalApiRouter {
    fn execute_subagent_tool(
        &mut self,
        parent_session_id: &str,
        call: &IterativeToolCall,
    ) -> Result<ToolObservation, String> {
        let decision = PolicyEngine::default_conservative().evaluate(Action::AgentControl);
        self.audit.record(
            AuditAction::PolicyDecision,
            format!("{} {:?}", call.name(), decision.outcome()),
        );
        if decision.outcome() != DecisionOutcome::AllowedAutomatic {
            return Err("subagent_policy_denied".to_string());
        }
        let response = match call.name() {
            "desktoplab.spawn_subagent" => self.spawn_subagent(
                &json!({
                    "parentSessionId":parent_session_id,
                    "prompt":required_string(call, "prompt")?,
                    "intent":required_string(call, "intent")?
                })
                .to_string(),
            ),
            name => {
                let child_id = required_string(call, "subagentId")?;
                if !self.subagent_belongs_to(child_id, parent_session_id) {
                    return Err("subagent_not_owned_by_parent".to_string());
                }
                let path = match name {
                    "desktoplab.send_subagent" => {
                        let response = self.subagent_route(
                            "POST",
                            &format!("/v1/agent/subagents/{child_id}/messages"),
                            &json!({"prompt":required_string(call, "prompt")?}).to_string(),
                        );
                        return self.observe_subagent_response(call, response);
                    }
                    "desktoplab.get_subagent" => {
                        format!("/v1/agent/subagents/{child_id}")
                    }
                    "desktoplab.cancel_subagent" => {
                        format!("/v1/agent/subagents/{child_id}/cancel")
                    }
                    "desktoplab.close_subagent" => {
                        format!("/v1/agent/subagents/{child_id}/close")
                    }
                    _ => return Err("unsupported_subagent_tool".to_string()),
                };
                let method = if name == "desktoplab.get_subagent" {
                    "GET"
                } else {
                    "POST"
                };
                self.subagent_route(method, &path, "{}")
            }
        };
        self.observe_subagent_response(call, response)
    }

    fn subagent_belongs_to(&self, child_id: &str, parent_session_id: &str) -> bool {
        self.subagents
            .get(child_id)
            .is_some_and(|record| record.belongs_to(parent_session_id))
    }

    fn observe_subagent_response(
        &mut self,
        call: &IterativeToolCall,
        response: ApiRouteResponse,
    ) -> Result<ToolObservation, String> {
        let output = serde_json::from_str::<Value>(response.body())
            .unwrap_or_else(|_| json!({"code":"SUBAGENT_RESPONSE_INVALID"}));
        if response.status() == "200 OK" {
            self.audit.record(
                AuditAction::ToolExecution,
                format!("{} completed", call.name()),
            );
            Ok(ToolObservation::success(call, output))
        } else {
            let reason = output
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("subagent_operation_failed")
                .to_ascii_lowercase();
            Ok(ToolObservation::failure_with_output(call, output, reason))
        }
    }
}

fn execution_owner(name: &str) -> Result<AgentToolExecutionOwner, String> {
    DesktopLabToolRegistry::default()
        .get(name)
        .map(|tool| tool.execution_owner())
        .or_else(|| {
            name.starts_with("mcp.")
                .then_some(AgentToolExecutionOwner::CanonicalGateway)
        })
        .ok_or_else(|| "unsupported_agent_tool".to_string())
}

fn required_string<'a>(call: &'a IterativeToolCall, name: &str) -> Result<&'a str, String> {
    call.arguments()
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing_argument:{name}"))
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::{AgentToolExecutionOwner, DesktopLabToolRegistry};

    use super::execution_owner;

    #[test]
    fn runtime_dispatch_accepts_every_catalog_owner_and_dynamic_mcp() {
        for tool in DesktopLabToolRegistry::default().tools() {
            assert_eq!(execution_owner(tool.id()), Ok(tool.execution_owner()));
        }
        assert_eq!(
            execution_owner("mcp.server.tool"),
            Ok(AgentToolExecutionOwner::CanonicalGateway)
        );
        assert_eq!(
            execution_owner("desktoplab.unknown"),
            Err("unsupported_agent_tool".to_string())
        );
    }
}
