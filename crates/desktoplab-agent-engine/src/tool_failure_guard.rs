use desktoplab_tool_gateway::ToolIntent;

use crate::ToolLoopStopReason;
use crate::loop_events::{tool_evidence, tool_source};

#[derive(Default)]
pub(crate) struct RepeatedFailureDetector {
    last_signature: Option<String>,
    count: usize,
}

impl RepeatedFailureDetector {
    pub(crate) fn observed(
        &mut self,
        intent: &ToolIntent,
        observation: Option<&str>,
    ) -> Option<usize> {
        let signature = failure_signature(intent, observation)?;
        if self.last_signature.as_deref() == Some(signature.as_str()) {
            self.count += 1;
        } else {
            self.last_signature = Some(signature);
            self.count = 1;
        }
        Some(self.count)
    }
}

fn failure_signature(intent: &ToolIntent, observation: Option<&str>) -> Option<String> {
    let observation = observation?;
    let lower = observation.to_ascii_lowercase();
    if !["error", "failed", "failure", "not found", "denied"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        return None;
    }
    Some(format!("{}:{}", tool_source(intent), tool_evidence(intent)))
}

pub(crate) fn blocked_reason(reason: ToolLoopStopReason) -> &'static str {
    match reason {
        ToolLoopStopReason::ApprovalRequired => "waiting for approval",
        ToolLoopStopReason::ApprovalDenied => "approval denied",
        ToolLoopStopReason::MaxSteps => "max_steps_exceeded",
        ToolLoopStopReason::MaxToolCalls => "max_tool_calls_exceeded",
        ToolLoopStopReason::MaxDuration => "max_duration_exceeded",
        ToolLoopStopReason::PolicyBlocked => "policy_blocked",
        ToolLoopStopReason::RepeatedToolFailure => "repeated_identical_tool_failure",
    }
}
