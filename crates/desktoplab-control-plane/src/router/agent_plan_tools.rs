use desktoplab_agent_engine::{IterativeToolCall, ToolObservation};
use desktoplab_agent_session::SessionEvent;
use desktoplab_backend_services::AuditAction;
use desktoplab_policy::{Action, DecisionOutcome, PolicyEngine};
use serde_json::{Value, json};

use super::LocalApiRouter;

impl LocalApiRouter {
    pub(super) fn execute_agent_plan_tool(
        &mut self,
        session_id: &str,
        call: &IterativeToolCall,
    ) -> Result<ToolObservation, String> {
        let decision = PolicyEngine::default_conservative().evaluate(Action::AgentControl);
        self.audit.record(
            AuditAction::PolicyDecision,
            format!("{} {:?}", call.name(), decision.outcome()),
        );
        if decision.outcome() != DecisionOutcome::AllowedAutomatic {
            return Err("plan_policy_denied".to_string());
        }
        let steps = parse_steps(call.arguments())?;
        let rendered = steps
            .iter()
            .map(|step| format!("[{}] {}", step.status, step.text))
            .collect::<Vec<_>>()
            .join("\n");
        self.sessions
            .append_events(session_id, &[SessionEvent::planning_started(&rendered)]);
        self.audit.record(
            AuditAction::ToolExecution,
            format!("{} recorded {} steps", call.name(), steps.len()),
        );
        Ok(ToolObservation::success(
            call,
            json!({
                "status":"recorded",
                "steps":steps.iter().map(|step| json!({
                    "step":step.text,
                    "status":step.status
                })).collect::<Vec<_>>()
            }),
        ))
    }
}

#[derive(Debug)]
struct PlanStep<'a> {
    text: &'a str,
    status: &'a str,
}

fn parse_steps(arguments: &Value) -> Result<Vec<PlanStep<'_>>, String> {
    let values = arguments
        .get("steps")
        .and_then(Value::as_array)
        .filter(|steps| !steps.is_empty() && steps.len() <= 20)
        .ok_or_else(|| "invalid_argument:steps".to_string())?;
    let steps = values
        .iter()
        .map(|value| {
            let text = value
                .get("step")
                .and_then(Value::as_str)
                .filter(|step| !step.trim().is_empty())
                .ok_or_else(|| "invalid_argument:step".to_string())?;
            let status = value
                .get("status")
                .and_then(Value::as_str)
                .filter(|status| matches!(status, &"pending" | &"in_progress" | &"completed"))
                .ok_or_else(|| "invalid_argument:status".to_string())?;
            Ok(PlanStep { text, status })
        })
        .collect::<Result<Vec<_>, String>>()?;
    if steps
        .iter()
        .filter(|step| step.status == "in_progress")
        .count()
        > 1
    {
        return Err("multiple_in_progress_plan_steps".to_string());
    }
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_steps;

    #[test]
    fn plan_rejects_multiple_active_steps_and_empty_copy() {
        assert_eq!(
            parse_steps(&json!({"steps":[
                {"step":"one","status":"in_progress"},
                {"step":"two","status":"in_progress"}
            ]}))
            .unwrap_err(),
            "multiple_in_progress_plan_steps"
        );
        assert_eq!(
            parse_steps(&json!({"steps":[{"step":" ","status":"pending"}]})).unwrap_err(),
            "invalid_argument:step"
        );
    }
}
