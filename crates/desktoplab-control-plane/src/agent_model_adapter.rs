use desktoplab_agent_engine::{
    DesktopLabToolRegistry, IterativeLoopEvent, IterativeLoopState, IterativeModelDecision,
    ProviderToolCallNormalizer,
};
use desktoplab_backends::BackendMessage;
use serde_json::{Value, json};

use crate::agent_completion_grounding::validate_inspection_message;
use crate::agent_execution_obligations::{
    validate_file_change_completion, validate_tool_prerequisites,
};

#[cfg(test)]
mod completion_tests;

#[cfg(test)]
mod execution_obligations_tests;

#[cfg(test)]
pub(crate) struct BackendDecisionAdapter<F> {
    initial_prompt: String,
    execute: F,
    registry: DesktopLabToolRegistry,
    normalizer: ProviderToolCallNormalizer,
}

#[cfg(test)]
impl<F> BackendDecisionAdapter<F> {
    #[cfg(test)]
    pub(crate) fn new(initial_prompt: impl Into<String>, execute: F) -> Self {
        let registry = DesktopLabToolRegistry::default();
        Self {
            initial_prompt: initial_prompt.into(),
            execute,
            normalizer: ProviderToolCallNormalizer::new(registry.clone()),
            registry,
        }
    }
}

#[cfg(test)]
impl<F> desktoplab_agent_engine::IterativeModelAdapter for BackendDecisionAdapter<F>
where
    F: FnMut(Vec<BackendMessage>) -> Result<String, String>,
{
    fn decide(&mut self, state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        let output = (self.execute)(backend_messages(
            &self.initial_prompt,
            state,
            &self.registry,
        ))?;
        decision_from_output(&self.normalizer, state, &output)
    }
}

pub(crate) fn backend_messages(
    initial_prompt: &str,
    state: &IterativeLoopState,
    registry: &DesktopLabToolRegistry,
) -> Vec<BackendMessage> {
    let mut messages = vec![BackendMessage::user(initial_prompt)];
    for event in state.events() {
        match event {
            IterativeLoopEvent::ToolRequested { call } => {
                messages.push(BackendMessage::assistant_tool_call(
                    call.id(),
                    call.name(),
                    call.arguments().clone(),
                ));
            }
            IterativeLoopEvent::ToolObserved { observation } => {
                let output = match observation.error() {
                    Some(error) => json!({
                        "error":error,
                        "output":observation.output(),
                        "provenance":observation.provenance()
                    }),
                    None => observation.output().clone(),
                };
                messages.push(BackendMessage::tool_result(
                    observation.call_id(),
                    observation.tool_name(),
                    output,
                ));
            }
            _ => {}
        }
    }
    let evidence_ledger = state
        .observations()
        .iter()
        .map(|observation| {
            json!({
                "id": observation.call_id(),
                "tool": observation.tool_name(),
                "status": if observation.error().is_none() { "success" } else { "failed" }
            })
        })
        .collect::<Vec<_>>();
    if !evidence_ledger.is_empty() {
        let failed_evidence_guidance = failure_recovery_guidance(state);
        messages.push(BackendMessage::user(format!(
            "Executor evidence ledger: {}. In desktoplab.complete evidenceCallIds, use only exact ids whose status is success. Classify outcome as answered for read-only findings, including reports about existing Git changes; executed for a successful non-mutation action; changed only when the agent applied a mutation with changed=true; or verified only with passing test evidence. Do not repeat a successful tool call with unchanged arguments while its observation is still current; select the next missing evidence or complete the task.{failed_evidence_guidance}",
            Value::Array(evidence_ledger),
        )));
    }
    if let Some(reason) = state.model_protocol_recovery() {
        let available_tools = registry
            .tools()
            .iter()
            .map(|tool| tool.id())
            .collect::<Vec<_>>()
            .join(", ");
        let recovery_guidance = protocol_recovery_guidance(reason);
        messages.push(BackendMessage::user(format!(
            "The previous response was rejected by the canonical tool protocol ({reason}). Retry exactly once. {recovery_guidance} A completion message must directly answer the current user request with concrete resource names or results from cited executor evidence; merely saying tools ran is not sufficient. Return exactly one JSON tool call using an exact canonical name and arguments that satisfy its schema. Canonical names available for this turn: {available_tools}. Never shorten, alias, or invent a tool name. Use an object named arguments for tool parameters. Do not include prose or Markdown fences."
        )));
    }
    messages
}

fn failure_recovery_guidance(state: &IterativeLoopState) -> &'static str {
    let observations = state.observations();
    if observations.iter().any(|observation| {
        observation.tool_name() == "desktoplab.run_tests" && observation.error().is_some()
    }) {
        return " Do not repeat a failed tool call with unchanged arguments when no new evidence or corrective action can change its result. A failing test is diagnostic evidence: inspect the implicated implementation and test files, correct the root cause without weakening the test, and rerun only after a corrective change.";
    }
    if observations
        .iter()
        .any(|observation| observation.error().is_some())
    {
        return " Do not repeat a failed tool call with unchanged arguments when no new evidence or corrective action can change its result. Inspect the failure, obtain missing evidence, or change the relevant state before retrying.";
    }
    ""
}

fn protocol_recovery_guidance(reason: &str) -> &'static str {
    match reason {
        "completion_test_evidence_required" => {
            "The verified outcome is reserved for a cited executor observation proving that tests passed; use answered for read-only findings or executed for a completed non-test action."
        }
        "completion_change_evidence_required" => {
            "The changed outcome means the agent applied a mutation and requires cited evidence with changed=true. A Git status or diff report is a read-only finding even when it describes existing changes; use answered for that report, or executed for another completed non-mutation action."
        }
        "completion_execution_evidence_required" => {
            "The executed outcome requires at least one successful cited executor observation; use answered only when reporting information rather than an executed action."
        }
        "completion_post_change_test_evidence_required" => {
            "A failed test and a workspace mutation create a pending validation obligation. Run the relevant tests again after the latest mutation and complete only after a passing executor observation."
        }
        "completion_post_change_inspection_required" => {
            "A file content mutation must be inspected after it executes. Read the changed file, or inspect the Git diff after a patch, then cite that successful observation in desktoplab.complete."
        }
        "patch_requires_prior_read" => {
            "Read the exact target file before applying a localized patch so the replacement is grounded in current workspace contents."
        }
        "completion_message_missing_evidence_anchor" => {
            "Name concrete resources or findings from the cited inspection evidence in the completion message."
        }
        "completion_message_missing_status_entries" => {
            "The completion message must account for every path in the bounded Git status evidence, including untracked paths."
        }
        "repeated_failed_tool_call_without_progress" => {
            "The same tool and arguments already failed and cannot run again without progress. Choose a different canonical tool or arguments to inspect the failure, obtain new evidence, or change the relevant state before retrying."
        }
        "missing_argument:message"
        | "missing_argument:outcome"
        | "missing_argument:evidenceCallIds"
        | "invalid_argument:outcome"
        | "invalid_arguments:desktoplab.complete" => {
            "Use desktoplab.complete with an arguments object containing exactly message, outcome, and evidenceCallIds. Set outcome to answered, executed, changed, or verified, and cite successful executor call ids."
        }
        _ => "Correct the rejected field according to the canonical tool schema.",
    }
}

pub(crate) fn decision_from_backend_output_with_registry(
    state: &IterativeLoopState,
    output: &str,
    registry: DesktopLabToolRegistry,
) -> Result<IterativeModelDecision, String> {
    decision_from_output(&ProviderToolCallNormalizer::new(registry), state, output)
}

fn decision_from_output(
    normalizer: &ProviderToolCallNormalizer,
    state: &IterativeLoopState,
    output: &str,
) -> Result<IterativeModelDecision, String> {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return Err("provider_response_missing_content".to_string());
    }
    let Ok(mut value) = serde_json::from_str::<Value>(trimmed) else {
        return Err("provider_canonical_tool_call_required".to_string());
    };
    let Some(object) = value.as_object_mut() else {
        return Err("provider_tool_call_must_be_object".to_string());
    };
    if object.contains_key("tool") && object.contains_key("function") {
        return Err("malformed_tool_call_envelope".to_string());
    }
    let name = object.get("tool").and_then(Value::as_str).or_else(|| {
        object
            .get("function")
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
    });
    let Some(name) = name.map(ToString::to_string) else {
        return Err("provider_tool_call_missing_name".to_string());
    };
    let arguments = object
        .get("arguments")
        .or_else(|| {
            object
                .get("function")
                .and_then(|function| function.get("arguments"))
        })
        .and_then(provider_arguments)
        .ok_or_else(|| "provider_tool_arguments_must_be_object".to_string())?;
    match name.as_str() {
        "desktoplab.complete" => {
            let call = normalizer
                .normalize(runtime_call_id(state), &name, arguments)
                .map_err(|error| error.to_string())?;
            completion_decision(state, call.arguments())
        }
        "desktoplab.clarify" => {
            let call = normalizer
                .normalize(runtime_call_id(state), &name, arguments)
                .map_err(|error| error.to_string())?;
            let question = call
                .arguments()
                .get("question")
                .and_then(Value::as_str)
                .filter(|question| !question.trim().is_empty())
                .ok_or_else(|| "missing_argument:question".to_string())?;
            let blocked_on = call
                .arguments()
                .get("blockedOn")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing_argument:blockedOn".to_string())?;
            Ok(IterativeModelDecision::clarification(question, blocked_on))
        }
        _ => {
            if object
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .is_none()
            {
                object.insert("id".to_string(), Value::String(runtime_call_id(state)));
            }
            if !object.contains_key("tool") && !object.contains_key("function") {
                object.insert("tool".to_string(), Value::String(name));
            }
            object.insert("arguments".to_string(), arguments);
            let call = normalizer
                .from_provider_value(&value)
                .map_err(|error| error.to_string())?;
            validate_tool_prerequisites(state, &call).map_err(ToString::to_string)?;
            if state
                .observations()
                .last()
                .is_some_and(|observation| observation.is_failed_repeat_of(&call))
            {
                return Err("repeated_failed_tool_call_without_progress".to_string());
            }
            Ok(IterativeModelDecision::tool_call(call))
        }
    }
}

fn completion_decision(
    state: &IterativeLoopState,
    arguments: &Value,
) -> Result<IterativeModelDecision, String> {
    let object = arguments
        .as_object()
        .ok_or_else(|| "provider_tool_arguments_must_be_object".to_string())?;
    if object.len() != 3
        || !object.contains_key("message")
        || !object.contains_key("outcome")
        || !object.contains_key("evidenceCallIds")
    {
        return Err("invalid_arguments:desktoplab.complete".to_string());
    }
    let message = arguments
        .get("message")
        .and_then(Value::as_str)
        .filter(|message| !message.trim().is_empty())
        .ok_or_else(|| "missing_argument:message".to_string())?;
    let outcome = arguments
        .get("outcome")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing_argument:outcome".to_string())?;
    if !matches!(outcome, "answered" | "executed" | "changed" | "verified") {
        return Err("invalid_argument:outcome".to_string());
    }
    let evidence_ids = arguments
        .get("evidenceCallIds")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing_argument:evidenceCallIds".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| "invalid_argument:evidenceCallIds".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !state.observations().is_empty() && evidence_ids.is_empty() {
        return Err("completion_evidence_required".to_string());
    }
    let evidence = evidence_ids
        .iter()
        .map(|call_id| {
            state
                .observations()
                .iter()
                .find(|observation| observation.call_id() == *call_id)
                .filter(|observation| observation.error().is_none())
                .ok_or_else(|| format!("completion_evidence_invalid:{call_id}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if has_unverified_test_repair(state) {
        return Err("completion_post_change_test_evidence_required".to_string());
    }
    match outcome {
        "answered" if state.observations().is_empty() && !evidence.is_empty() => {
            return Err("completion_evidence_unexpected".to_string());
        }
        "executed" if evidence.is_empty() => {
            return Err("completion_execution_evidence_required".to_string());
        }
        "changed"
            if !evidence
                .iter()
                .any(|observation| is_successful_change(observation))
                && !is_git_change_report(&evidence) =>
        {
            return Err("completion_change_evidence_required".to_string());
        }
        "verified"
            if !evidence
                .iter()
                .any(|observation| is_passing_test(observation)) =>
        {
            return Err("completion_test_evidence_required".to_string());
        }
        _ => {}
    }
    validate_file_change_completion(state, outcome, &evidence).map_err(ToString::to_string)?;
    validate_inspection_message(message, &evidence).map_err(ToString::to_string)?;
    Ok(IterativeModelDecision::final_response(message))
}

fn is_successful_change(observation: &desktoplab_agent_engine::ToolObservation) -> bool {
    match observation.tool_name() {
        "desktoplab.write_file"
        | "desktoplab.patch_file"
        | "desktoplab.create_directory"
        | "desktoplab.move_path"
        | "desktoplab.delete_path" => {
            observation.output().get("changed").and_then(Value::as_bool) == Some(true)
        }
        "desktoplab.commit_changes" => {
            observation.output().get("status").and_then(Value::as_str) == Some("committed")
        }
        "desktoplab.push_changes" => {
            observation.output().get("status").and_then(Value::as_str) == Some("pushed")
        }
        _ => false,
    }
}

fn is_passing_test(observation: &desktoplab_agent_engine::ToolObservation) -> bool {
    observation.is_passing_test_evidence()
}

fn is_git_change_report(evidence: &[&desktoplab_agent_engine::ToolObservation]) -> bool {
    !evidence.is_empty()
        && evidence
            .iter()
            .all(|observation| is_read_only_inspection(observation.tool_name()))
        && evidence
            .iter()
            .any(|observation| match observation.tool_name() {
                "desktoplab.git_status" => observation
                    .output()
                    .get("entries")
                    .and_then(Value::as_array)
                    .is_some_and(|entries| !entries.is_empty()),
                "desktoplab.git_diff" => observation
                    .output()
                    .get("diff")
                    .and_then(Value::as_str)
                    .is_some_and(|diff| !diff.trim().is_empty()),
                _ => false,
            })
}

fn is_read_only_inspection(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "desktoplab.read_file"
            | "desktoplab.list_files"
            | "desktoplab.search_text"
            | "desktoplab.git_status"
            | "desktoplab.git_diff"
    )
}

fn has_unverified_test_repair(state: &IterativeLoopState) -> bool {
    let observations = state.observations();
    let Some(last_failed_test) = observations.iter().rposition(|observation| {
        observation.tool_name() == "desktoplab.run_tests" && observation.error().is_some()
    }) else {
        return false;
    };
    let Some(last_change) = observations
        .iter()
        .rposition(|observation| is_successful_change(observation))
    else {
        return false;
    };
    let validation_boundary = last_failed_test.max(last_change);

    !observations
        .iter()
        .skip(validation_boundary + 1)
        .any(|observation| is_passing_test(observation))
}

fn provider_arguments(value: &Value) -> Option<Value> {
    match value {
        Value::Object(_) => Some(value.clone()),
        Value::String(raw) => serde_json::from_str::<Value>(raw)
            .ok()
            .filter(Value::is_object),
        _ => None,
    }
}

fn runtime_call_id(state: &IterativeLoopState) -> String {
    format!("call.{}", state.model_turns())
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::{
        DesktopLabToolRegistry, IterativeAgentLoop, IterativeLoopState, IterativeLoopStatus,
        IterativeModelAdapter, IterativeModelDecision, IterativeToolCall, IterativeToolExecutor,
        ToolObservation,
    };
    use desktoplab_backends::BackendMessage;
    use serde_json::json;

    use super::{BackendDecisionAdapter, backend_messages};

    struct ReadExecutor;

    impl IterativeToolExecutor for ReadExecutor {
        fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
            Ok(ToolObservation::success(
                call,
                json!({"path":"README.md","text":"DesktopLab"}),
            ))
        }
    }

    struct StaticExecutor(serde_json::Value);

    impl IterativeToolExecutor for StaticExecutor {
        fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
            Ok(ToolObservation::success(call, self.0.clone()))
        }
    }

    struct FailingTestExecutor;

    impl IterativeToolExecutor for FailingTestExecutor {
        fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
            Ok(ToolObservation::failure(call, "tests_failed:1"))
        }
    }

    fn state_after_failed_test_and_patch(session_id: &str) -> IterativeLoopState {
        let mut failed_test = BackendDecisionAdapter::new("Repair tests", |_| {
            Ok(r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#.to_string())
        });
        let mut state = IterativeLoopState::new(session_id);
        IterativeAgentLoop::default().advance(
            &mut state,
            &mut failed_test,
            &mut FailingTestExecutor,
        );

        let mut read = BackendDecisionAdapter::new("Repair tests", |_| {
            Ok(r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"calculator.js"}}"#.to_string())
        });
        IterativeAgentLoop::default().advance(
            &mut state,
            &mut read,
            &mut StaticExecutor(json!({"path":"calculator.js","text":"return left - right;"})),
        );

        let mut patch = BackendDecisionAdapter::new("Repair tests", |_| {
            Ok(r#"{"id":"patch-1","tool":"desktoplab.patch_file","arguments":{"path":"calculator.js","expected":"return left - right;","replacement":"return left + right;"}}"#.to_string())
        });
        IterativeAgentLoop::default().advance(
            &mut state,
            &mut patch,
            &mut StaticExecutor(json!({"path":"calculator.js","changed":true})),
        );
        state
    }

    #[test]
    fn adapter_preserves_native_call_and_result_history() {
        let mut requests = Vec::<Vec<BackendMessage>>::new();
        let outputs = [
            r#"{"id":"read-1","tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
            r#"{"tool":"desktoplab.complete","arguments":{"message":"README inspected.","outcome":"answered","evidenceCallIds":["read-1"]}}"#,
        ];
        let mut turn = 0;
        let mut adapter = BackendDecisionAdapter::new("Inspect the repository", |messages| {
            requests.push(messages);
            let output = outputs[turn].to_string();
            turn += 1;
            Ok(output)
        });
        let mut state = IterativeLoopState::new("session.adapter");

        IterativeAgentLoop::default().run(&mut state, &mut adapter, &mut ReadExecutor);

        assert_eq!(state.status(), IterativeLoopStatus::Completed);
        assert_eq!(requests.len(), 2);
        assert!(matches!(requests[0].as_slice(), [BackendMessage::User(_)]));
        assert!(matches!(
            requests[1].as_slice(),
            [
                BackendMessage::User(_),
                BackendMessage::AssistantToolCall { .. },
                BackendMessage::ToolResult { .. },
                BackendMessage::User(_)
            ]
        ));
        let call_id = match &requests[1][1] {
            BackendMessage::AssistantToolCall { call_id, .. } => call_id,
            _ => unreachable!(),
        };
        let result_call_id = match &requests[1][2] {
            BackendMessage::ToolResult { call_id, .. } => call_id,
            _ => unreachable!(),
        };
        assert_eq!(call_id, result_call_id);
        assert_eq!(call_id, "read-1");
        let BackendMessage::User(ledger) = &requests[1][3] else {
            unreachable!();
        };
        assert!(
            ledger.contains(r#"{"id":"read-1","status":"success","tool":"desktoplab.read_file"}"#)
        );
        assert!(ledger.contains("Do not repeat a successful tool call with unchanged arguments"));
        assert!(ledger.contains("answered for read-only findings"));
        assert!(ledger.contains("reports about existing Git changes"));
        assert!(ledger.contains("changed only when the agent applied a mutation"));
        assert!(ledger.contains("verified only with passing test evidence"));
        assert!(!ledger.contains("DesktopLab"));
    }

    #[test]
    fn failed_executor_evidence_requires_progress_before_retry() {
        let mut requests = Vec::<Vec<BackendMessage>>::new();
        let outputs = [
            r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#,
            r#"{"tool":"desktoplab.complete","arguments":{"message":"The test failed with exit code 1.","outcome":"answered","evidenceCallIds":[]}}"#,
        ];
        let mut turn = 0;
        let mut adapter = BackendDecisionAdapter::new("Repair the failing test", |messages| {
            requests.push(messages);
            let output = outputs[turn].to_string();
            turn += 1;
            Ok(output)
        });
        let mut state = IterativeLoopState::new("session.failed-test-guidance");

        IterativeAgentLoop::default().run(&mut state, &mut adapter, &mut FailingTestExecutor);

        let BackendMessage::User(ledger) = requests[1].last().unwrap() else {
            panic!("executor ledger should be the final user message");
        };
        assert!(
            ledger.contains(r#"{"id":"test-1","status":"failed","tool":"desktoplab.run_tests"}"#)
        );
        assert!(ledger.contains("Do not repeat a failed tool call with unchanged arguments"));
        assert!(ledger.contains("A failing test is diagnostic evidence"));
        assert!(ledger.contains("inspect the implicated implementation and test files"));
        assert!(ledger.contains("rerun only after a corrective change"));
    }

    #[test]
    fn immediate_retry_of_the_same_failed_call_is_rejected() {
        let mut first = BackendDecisionAdapter::new("Run tests", |_| {
            Ok(r#"{"id":"test-1","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#.to_string())
        });
        let mut state = IterativeLoopState::new("session.failed-call-retry");
        IterativeAgentLoop::default().advance(&mut state, &mut first, &mut FailingTestExecutor);

        let mut repeated = BackendDecisionAdapter::new("Run tests", |_| {
            Ok(r#"{"id":"test-2","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#.to_string())
        });

        assert_eq!(
            repeated.decide(&state),
            Err("repeated_failed_tool_call_without_progress".to_string())
        );
    }

    #[test]
    fn completion_after_a_failed_test_and_change_requires_a_passing_rerun() {
        let state = state_after_failed_test_and_patch("session.unverified-repair");

        let mut completion = BackendDecisionAdapter::new("Repair tests", |_| {
            Ok(r#"{"tool":"desktoplab.complete","arguments":{"message":"Fixed calculator.js and the tests now pass.","outcome":"changed","evidenceCallIds":["patch-1"]}}"#.to_string())
        });

        assert_eq!(
            completion.decide(&state),
            Err("completion_post_change_test_evidence_required".to_string())
        );
    }

    #[test]
    fn passing_rerun_after_the_latest_change_satisfies_validation() {
        let mut state = state_after_failed_test_and_patch("session.verified-repair");

        let mut passing_test = BackendDecisionAdapter::new("Repair tests", |_| {
            Ok(r#"{"id":"test-2","tool":"desktoplab.run_tests","arguments":{"command":"npm test"}}"#.to_string())
        });
        IterativeAgentLoop::default().advance(
            &mut state,
            &mut passing_test,
            &mut StaticExecutor(json!({"passed":true,"exitCode":0})),
        );

        let mut completion = BackendDecisionAdapter::new("Repair tests", |_| {
            Ok(r#"{"tool":"desktoplab.complete","arguments":{"message":"Fixed calculator.js and npm test passed.","outcome":"verified","evidenceCallIds":["patch-1","test-2"]}}"#.to_string())
        });

        assert_eq!(
            completion.decide(&state),
            Ok(IterativeModelDecision::final_response(
                "Fixed calculator.js and npm test passed."
            ))
        );
    }

    #[test]
    fn protocol_recovery_is_the_final_user_message_without_bad_model_output() {
        let mut state = IterativeLoopState::new("session.recovery");
        assert!(state.request_model_protocol_retry("unknown_tool:read_file"));

        let messages = backend_messages(
            "Inspect the repository",
            &state,
            &DesktopLabToolRegistry::default(),
        );

        let BackendMessage::User(recovery) = messages.last().unwrap() else {
            panic!("recovery should be a user protocol correction");
        };
        assert!(recovery.contains("unknown_tool:read_file"));
        assert!(recovery.contains("Canonical names available for this turn:"));
        assert!(recovery.contains("desktoplab.patch_file"));
        assert!(!recovery.contains("previous response:"));
    }

    #[test]
    fn protocol_recovery_explains_completion_outcome_semantics() {
        let mut state = IterativeLoopState::new("session.outcome-recovery");
        assert!(state.request_model_protocol_retry("completion_test_evidence_required"));

        let messages = backend_messages(
            "Summarize Git findings",
            &state,
            &DesktopLabToolRegistry::default(),
        );
        let BackendMessage::User(recovery) = messages.last().unwrap() else {
            panic!("recovery should be a user protocol correction");
        };

        assert!(recovery.contains("verified outcome is reserved"));
        assert!(recovery.contains("use answered for read-only findings"));
    }

    #[test]
    fn protocol_recovery_explains_pending_test_validation() {
        let mut state = IterativeLoopState::new("session.validation-recovery");
        assert!(
            state.request_model_protocol_retry("completion_post_change_test_evidence_required")
        );

        let messages = backend_messages(
            "Repair the failing tests",
            &state,
            &DesktopLabToolRegistry::default(),
        );
        let BackendMessage::User(recovery) = messages.last().unwrap() else {
            panic!("recovery should be a user protocol correction");
        };

        assert!(recovery.contains("pending validation obligation"));
        assert!(recovery.contains("after the latest mutation"));
        assert!(recovery.contains("passing executor observation"));
    }

    #[test]
    fn protocol_recovery_restates_the_complete_schema_for_missing_fields() {
        let mut state = IterativeLoopState::new("session.complete-schema-recovery");
        assert!(state.request_model_protocol_retry("missing_argument:outcome"));

        let messages = backend_messages(
            "Summarize Git findings",
            &state,
            &DesktopLabToolRegistry::default(),
        );
        let BackendMessage::User(recovery) = messages.last().unwrap() else {
            panic!("recovery should be a user protocol correction");
        };

        assert!(recovery.contains("desktoplab.complete"));
        assert!(recovery.contains("message, outcome, and evidenceCallIds"));
        assert!(recovery.contains("answered, executed, changed, or verified"));
    }

    #[test]
    fn adapter_rejects_json_without_a_registered_tool_envelope() {
        let mut adapter =
            BackendDecisionAdapter::new("Inspect", |_| Ok(json!({"ok":true}).to_string()));
        let state = IterativeLoopState::new("session.invalid");

        assert_eq!(
            adapter.decide(&state),
            Err("provider_tool_call_missing_name".to_string())
        );
    }

    #[test]
    fn adapter_generates_ids_for_native_calls_that_omit_or_null_them() {
        for output in [
            r#"{"tool":"desktoplab.git_status","arguments":{}}"#,
            r#"{"id":null,"tool":"desktoplab.git_status","arguments":{}}"#,
        ] {
            let mut adapter = BackendDecisionAdapter::new("Status", |_| Ok(output.to_string()));
            let state = IterativeLoopState::new("session.generated-id");
            let decision = adapter.decide(&state).expect("call should normalize");
            let IterativeModelDecision::ToolCall(call) = decision else {
                panic!("expected tool call");
            };
            assert_eq!(call.id(), "call.0");
        }
    }

    #[test]
    fn generated_call_id_is_exposed_for_grounded_completion() {
        let outputs = [
            r#"{"tool":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
            r#"{"tool":"desktoplab.complete","arguments":{"message":"README inspected.","outcome":"answered","evidenceCallIds":["call.1"]}}"#,
        ];
        let mut turn = 0;
        let mut adapter = BackendDecisionAdapter::new("Inspect", |_| {
            let output = outputs[turn].to_string();
            turn += 1;
            Ok(output)
        });
        let mut state = IterativeLoopState::new("session.generated-evidence");

        IterativeAgentLoop::default().run(&mut state, &mut adapter, &mut ReadExecutor);

        assert_eq!(state.status(), IterativeLoopStatus::Completed);
    }

    #[test]
    fn inspection_completion_requires_concrete_executor_evidence() {
        let outputs = [
            r#"{"tool":"desktoplab.git_status","arguments":{}}"#,
            r#"{"tool":"desktoplab.complete","arguments":{"message":"Git status completed successfully.","outcome":"answered","evidenceCallIds":["call.1"]}}"#,
        ];
        let mut turn = 0;
        let mut adapter = BackendDecisionAdapter::new("Summarize changed files", |_| {
            let output = outputs[turn].to_string();
            turn += 1;
            Ok(output)
        });
        let mut state = IterativeLoopState::new("session.grounding");

        IterativeAgentLoop::default().advance(
            &mut state,
            &mut adapter,
            &mut StaticExecutor(json!({"entries":[" M calculator.js"]})),
        );

        assert_eq!(state.status(), IterativeLoopStatus::Running);
        assert_eq!(
            adapter.decide(&state),
            Err("completion_message_missing_status_entries".to_string())
        );
    }

    #[test]
    fn adapter_rejects_ambiguous_native_and_direct_envelopes() {
        let mut adapter = BackendDecisionAdapter::new("Inspect", |_| {
            Ok(r#"{"id":"ambiguous","tool":"desktoplab.git_status","arguments":{},"function":{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}}"#.to_string())
        });
        let state = IterativeLoopState::new("session.ambiguous");

        assert_eq!(
            adapter.decide(&state),
            Err("malformed_tool_call_envelope".to_string())
        );
    }

    #[test]
    fn plain_text_cannot_bypass_the_canonical_completion_contract() {
        let mut adapter =
            BackendDecisionAdapter::new("Explain", |_| Ok("Grounded answer".to_string()));
        let state = IterativeLoopState::new("session.text");

        assert_eq!(
            adapter.decide(&state),
            Err("provider_canonical_tool_call_required".to_string())
        );
    }

    #[test]
    fn completion_rejects_unregistered_arguments() {
        let mut adapter = BackendDecisionAdapter::new("Explain", |_| {
            Ok(r#"{"tool":"desktoplab.complete","arguments":{"message":"Done.","outcome":"answered","evidenceCallIds":[],"assumeSuccess":true}}"#.to_string())
        });
        let state = IterativeLoopState::new("session.extra");

        assert_eq!(
            adapter.decide(&state),
            Err("unexpected_argument:assumeSuccess".to_string())
        );
    }

    #[test]
    fn clarification_requires_a_registered_blocked_action() {
        let mut missing = BackendDecisionAdapter::new("Change", |_| {
            Ok(
                r#"{"tool":"desktoplab.clarify","arguments":{"question":"Which value?"}}"#
                    .to_string(),
            )
        });
        let mut unknown = BackendDecisionAdapter::new("Change", |_| {
            Ok(r#"{"tool":"desktoplab.clarify","arguments":{"question":"Which value?","blockedOn":"desktoplab.unknown"}}"#.to_string())
        });
        let state = IterativeLoopState::new("session.clarify");

        assert_eq!(
            missing.decide(&state),
            Err("missing_argument:blockedOn".to_string())
        );
        assert_eq!(
            unknown.decide(&state),
            Err("invalid_argument:blockedOn".to_string())
        );
    }

    #[test]
    fn clarification_preserves_question_and_blocked_action() {
        let mut adapter = BackendDecisionAdapter::new("Change", |_| {
            Ok(r#"{"tool":"desktoplab.clarify","arguments":{"question":"Which value?","blockedOn":"desktoplab.patch_file"}}"#.to_string())
        });
        let state = IterativeLoopState::new("session.clarify");

        assert_eq!(
            adapter.decide(&state),
            Ok(IterativeModelDecision::clarification(
                "Which value?",
                "desktoplab.patch_file"
            ))
        );
    }
}
