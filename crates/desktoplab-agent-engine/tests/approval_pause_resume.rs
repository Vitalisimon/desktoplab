use std::collections::VecDeque;

use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeApproval, IterativeLoopEvent, IterativeLoopState,
    IterativeLoopStatus, IterativeModelAdapter, IterativeModelDecision, IterativeToolCall,
    IterativeToolExecutor, ToolObservation,
};
use serde_json::json;
use xtask::check_logical_line_limit;

struct ScriptedModel {
    decisions: VecDeque<IterativeModelDecision>,
}

impl IterativeModelAdapter for ScriptedModel {
    fn decide(&mut self, _state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        self.decisions
            .pop_front()
            .ok_or_else(|| "script_exhausted".to_string())
    }
}

#[derive(Default)]
struct ApprovalExecutor {
    approved_calls: Vec<String>,
}

impl IterativeToolExecutor for ApprovalExecutor {
    fn execute(&mut self, _call: &IterativeToolCall) -> Result<ToolObservation, String> {
        Err("approval_required".to_string())
    }

    fn execute_approved(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        self.approved_calls.push(call.id().to_string());
        Ok(ToolObservation::success(call, json!({"applied":true})))
    }
}

#[test]
fn approval_resumes_same_loop_and_reaches_final_without_new_user_prompt() {
    let call = tool("write-1", "desktoplab.write_file");
    let mut state = IterativeLoopState::new("same-session");
    let mut model = model_for(call.clone());
    let mut executor = ApprovalExecutor::default();
    let agent_loop = IterativeAgentLoop::default();

    agent_loop.run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::WaitingForApproval);
    let pending = state.pending_approval().expect("approval must persist");
    assert_eq!(pending.payload_fingerprint().len(), 64);
    assert!(!pending.payload_fingerprint().contains("notes"));
    let approval = IterativeApproval::approved(pending.call_id(), pending.payload_fingerprint());
    agent_loop.resume_with_approval(&mut state, approval, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
    assert_eq!(
        state.final_response(),
        Some("Work completed with evidence.")
    );
    assert_eq!(executor.approved_calls, ["write-1"]);
    assert_eq!(state.model_turns(), 2);
    assert_eq!(
        state
            .events()
            .iter()
            .filter(|event| matches!(event, IterativeLoopEvent::ApprovalRequired { .. }))
            .count(),
        1
    );
}

#[test]
fn restart_between_approval_and_execution_does_not_duplicate_action() {
    let call = tool("patch-1", "desktoplab.patch_file");
    let mut state = IterativeLoopState::new("restart-approval");
    let mut model = model_for(call);
    let mut executor = ApprovalExecutor::default();
    let agent_loop = IterativeAgentLoop::default();

    agent_loop.run(&mut state, &mut model, &mut executor);
    let persisted = state.to_json().unwrap();
    let mut restored = IterativeLoopState::from_json(&persisted).unwrap();
    let pending = restored.pending_approval().unwrap();
    let approval = IterativeApproval::approved(pending.call_id(), pending.payload_fingerprint());

    agent_loop.resume_with_approval(&mut restored, approval.clone(), &mut model, &mut executor);
    agent_loop.resume_with_approval(&mut restored, approval, &mut model, &mut executor);

    assert_eq!(restored.status(), IterativeLoopStatus::Completed);
    assert_eq!(executor.approved_calls, ["patch-1"]);
    assert!(restored.pending_approval().is_none());
}

#[test]
fn externally_executed_tool_resumes_from_correlated_observation_without_reexecution() {
    let call = tool("write-external-1", "desktoplab.write_file");
    let mut state = IterativeLoopState::new("external-execution");
    let mut model = model_for(call.clone());
    let mut executor = ApprovalExecutor::default();
    let agent_loop = IterativeAgentLoop::default();
    agent_loop.run(&mut state, &mut model, &mut executor);
    let pending = state.pending_approval().unwrap();
    let approval = IterativeApproval::approved(pending.call_id(), pending.payload_fingerprint());
    let observation = ToolObservation::success(&call, json!({"written":true}));

    assert!(agent_loop.accept_approved_observation(&mut state, approval, observation));
    assert_eq!(state.status(), IterativeLoopStatus::Running);
    assert_eq!(state.model_turns(), 1);
    agent_loop.run(&mut state, &mut model, &mut executor);

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
    assert!(executor.approved_calls.is_empty());
    assert_eq!(state.observations()[0].call_id(), "write-external-1");
}

#[test]
fn changed_denied_and_expired_approvals_fail_closed_without_execution() {
    for (session, approval_kind, reason) in [
        ("changed", "changed", "approval_payload_mismatch"),
        ("denied", "denied", "approval_denied"),
        ("expired", "expired", "approval_expired"),
    ] {
        let call = tool("terminal-1", "desktoplab.run_terminal");
        let mut state = IterativeLoopState::new(session);
        let mut model = model_for(call);
        let mut executor = ApprovalExecutor::default();
        let agent_loop = IterativeAgentLoop::default();
        agent_loop.run(&mut state, &mut model, &mut executor);
        let pending = state.pending_approval().unwrap();
        let approval = match approval_kind {
            "changed" => IterativeApproval::approved(pending.call_id(), "changed-payload"),
            "denied" => IterativeApproval::denied(pending.call_id(), pending.payload_fingerprint()),
            _ => IterativeApproval::expired(pending.call_id(), pending.payload_fingerprint()),
        };

        agent_loop.resume_with_approval(&mut state, approval, &mut model, &mut executor);

        assert_eq!(state.stop_reason_code(), Some(reason));
        assert!(executor.approved_calls.is_empty());
    }
}

#[test]
fn all_sensitive_coding_tools_share_the_same_resume_contract() {
    for (index, name) in [
        "desktoplab.write_file",
        "desktoplab.patch_file",
        "desktoplab.run_terminal",
        "desktoplab.run_tests",
        "desktoplab.commit_changes",
        "desktoplab.push_changes",
    ]
    .iter()
    .enumerate()
    {
        let mut state = IterativeLoopState::new(format!("approval-{index}"));
        let mut model = model_for(tool(&format!("call-{index}"), name));
        let mut executor = ApprovalExecutor::default();
        let agent_loop = IterativeAgentLoop::default();
        agent_loop.run(&mut state, &mut model, &mut executor);
        let pending = state.pending_approval().unwrap();
        let approval =
            IterativeApproval::approved(pending.call_id(), pending.payload_fingerprint());

        agent_loop.resume_with_approval(&mut state, approval, &mut model, &mut executor);

        assert_eq!(state.status(), IterativeLoopStatus::Completed, "{name}");
        assert_eq!(executor.approved_calls.len(), 1, "{name}");
    }
}

#[test]
fn approval_sources_stay_below_line_guards() {
    for (path, source) in [
        (
            "crates/desktoplab-agent-engine/src/iterative_approval.rs",
            include_str!("../src/iterative_approval.rs"),
        ),
        (
            "crates/desktoplab-agent-engine/src/iterative_resume.rs",
            include_str!("../src/iterative_resume.rs"),
        ),
    ] {
        check_logical_line_limit(path, source, 250).expect("approval source grew too large");
    }
}

fn model_for(call: IterativeToolCall) -> ScriptedModel {
    ScriptedModel {
        decisions: [
            IterativeModelDecision::tool_call(call),
            IterativeModelDecision::final_response("Work completed with evidence."),
        ]
        .into(),
    }
}

fn tool(id: &str, name: &str) -> IterativeToolCall {
    IterativeToolCall::new(
        id,
        name,
        match name {
            "desktoplab.write_file" => json!({"path":"notes.md","content":"notes"}),
            "desktoplab.patch_file" => {
                json!({"path":"notes.md","expected":"old","replacement":"new"})
            }
            "desktoplab.run_terminal" => json!({"command":"printf ok","cwd":""}),
            "desktoplab.run_tests" => json!({"command":"cargo test"}),
            "desktoplab.commit_changes" => json!({"message":"test commit"}),
            "desktoplab.push_changes" => json!({"remote":"origin","branch":"main"}),
            _ => json!({}),
        },
    )
}
