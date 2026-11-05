use super::super::agent_model_execution::AgentModelExecutionError;
use super::{CompletedAgentModelTurn, LocalApiRouter};
use crate::router::WorkspaceRecord;

#[test]
fn generation_tests_stay_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_model_jobs_generation_tests.rs",
        include_str!("agent_model_jobs_generation_tests.rs"),
        140,
    )
    .unwrap();
}

#[test]
fn model_turn_keeps_the_workspace_owned_by_its_session() {
    let (mut router, id) = setup();
    router.agent_iterative_prompts.insert(id, "prompt".into());
    router.agent_model_inflight.clear();
    router.workspace = Some(WorkspaceRecord {
        workspace_id: "workspace.b".into(),
        display_name: "B".into(),
        root_path: "/tmp/workspace-b".into(),
    });
    assert_eq!(
        router.claim_next_agent_model_turn().unwrap().workspace_id(),
        "workspace.a"
    );
}
use desktoplab_agent_engine::IterativeLoopState;
use desktoplab_agent_session::SessionState;
use std::sync::{Arc, atomic::AtomicBool};

fn setup() -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    let id = router
        .sessions
        .create_session("workspace.a", "backend.ollama")
        .session_id()
        .to_string();
    router.sessions.start(&id);
    router
        .agent_iterative_states
        .insert(id.clone(), IterativeLoopState::new(&id));
    router
        .agent_cancellation_tokens
        .insert(id.clone(), Arc::new(AtomicBool::new(false)));
    router.agent_model_inflight.insert(id.clone());
    (router, id)
}

fn completion(
    id: &str,
    token: Arc<AtomicBool>,
    output: Result<String, AgentModelExecutionError>,
) -> CompletedAgentModelTurn {
    CompletedAgentModelTurn {
        session_id: id.into(),
        workspace_id: "workspace.a".into(),
        backend_id: "backend.ollama".into(),
        cancellation: token,
        output,
    }
}

#[test]
fn stale_cancelled_model_result_cannot_cancel_a_new_turn() {
    let (mut router, id) = setup();
    router.complete_agent_model_turn(completion(
        &id,
        Arc::new(AtomicBool::new(true)),
        Err(AgentModelExecutionError::Cancelled),
    ));
    assert_eq!(
        router.sessions.get(&id).unwrap().state(),
        SessionState::Running
    );
    assert!(router.agent_iterative_states.contains_key(&id));
    assert!(!router.agent_model_inflight.contains(&id));
}

#[test]
fn late_success_from_a_replaced_turn_is_discarded() {
    let (mut router, id) = setup();
    router.complete_agent_model_turn(completion(
        &id,
        Arc::new(AtomicBool::new(true)),
        Ok("stale output".into()),
    ));
    assert_eq!(
        router.sessions.get(&id).unwrap().state(),
        SessionState::Running
    );
    assert!(router.agent_iterative_states.contains_key(&id));
    assert!(!router.agent_model_inflight.contains(&id));
}

#[test]
fn current_cancelled_model_result_still_cancels() {
    let (mut router, id) = setup();
    let token = Arc::clone(router.agent_cancellation_tokens.get(&id).unwrap());
    router.complete_agent_model_turn(completion(
        &id,
        token,
        Err(AgentModelExecutionError::Cancelled),
    ));
    assert_eq!(
        router.sessions.get(&id).unwrap().state(),
        SessionState::Cancelled
    );
}
