use std::collections::VecDeque;

use desktoplab_backend_services::{
    WorkflowDefinition, WorkflowError, WorkflowExecutor, WorkflowNode, WorkflowNodeKind,
    WorkflowService, WorkflowStatus, WorkflowStepOutcome,
};
use desktoplab_storage::SqliteStore;
use serde_json::json;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[derive(Default)]
struct Executor {
    outcomes: VecDeque<WorkflowStepOutcome>,
    calls: Vec<String>,
    compensations: Vec<String>,
}

impl WorkflowExecutor for Executor {
    fn execute(&mut self, node: &WorkflowNode, attempt: u8) -> WorkflowStepOutcome {
        self.calls.push(format!("{}:{attempt}", node.id));
        self.outcomes
            .pop_front()
            .unwrap_or_else(|| WorkflowStepOutcome::Completed(json!(node.id)))
    }

    fn compensate(&mut self, node: &WorkflowNode, compensation: &str) -> Result<(), String> {
        self.compensations
            .push(format!("{}:{compensation}", node.id));
        Ok(())
    }
}

fn node(id: &str, kind: WorkflowNodeKind, dependencies: &[&str]) -> WorkflowNode {
    WorkflowNode {
        id: id.to_string(),
        kind,
        dependencies: dependencies.iter().map(|value| value.to_string()).collect(),
        condition_node: None,
        mutates: false,
        idempotency_key: None,
        compensation: None,
        max_attempts: 2,
        timeout_ms: 1_000,
    }
}

fn definition() -> WorkflowDefinition {
    WorkflowDefinition {
        workflow_id: "review_then_write".to_string(),
        nodes: vec![
            node("compute", WorkflowNodeKind::Compute, &[]),
            node("approve", WorkflowNodeKind::Approval, &["compute"]),
            WorkflowNode {
                mutates: true,
                idempotency_key: Some("write-v1".to_string()),
                ..node("write", WorkflowNodeKind::Action, &["approve"])
            },
            node("checkpoint", WorkflowNodeKind::Checkpoint, &["write"]),
        ],
    }
}

fn migrated_store(path: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(path).unwrap();
    store.apply_migrations().unwrap();
    store
}

fn memory_store() -> SqliteStore {
    let store = SqliteStore::open_in_memory().unwrap();
    store.apply_migrations().unwrap();
    store
}

#[test]
fn approval_resumes_persisted_workflow_exactly_once() {
    let temp = TempDir::new().unwrap();
    let store = migrated_store(&temp.path().join("state.sqlite"));
    let service = WorkflowService::new(&store);
    let mut executor = Executor::default();
    let waiting = service.start("run-1", definition(), &mut executor).unwrap();
    assert_eq!(waiting.status, WorkflowStatus::WaitingApproval);
    assert_eq!(executor.calls, ["compute:1"]);
    assert_eq!(waiting.graph.edges.len(), 3);
    let token = waiting.resume_token.unwrap();
    drop(store);

    let reopened = migrated_store(&temp.path().join("state.sqlite"));
    let service = WorkflowService::new(&reopened);
    let completed = service
        .resume("run-1", &token, true, &mut executor)
        .unwrap();
    assert_eq!(completed.status, WorkflowStatus::Completed);
    assert_eq!(executor.calls, ["compute:1", "write:1", "checkpoint:1"]);
    assert!(matches!(
        service.resume("run-1", &token, true, &mut executor),
        Err(WorkflowError::NotWaitingForApproval)
    ));
}

#[test]
fn invalid_graph_fails_before_executor_mutation() {
    let store = memory_store();
    let service = WorkflowService::new(&store);
    let mut executor = Executor::default();
    let unsafe_definition = WorkflowDefinition {
        workflow_id: "unsafe".to_string(),
        nodes: vec![WorkflowNode {
            mutates: true,
            ..node("write", WorkflowNodeKind::Action, &[])
        }],
    };
    assert!(matches!(
        service.start("run", unsafe_definition, &mut executor),
        Err(WorkflowError::InvalidGraph("unsafe_replayable_mutation"))
    ));
    assert!(executor.calls.is_empty());
}

#[test]
fn retry_timeout_and_compensation_are_explicit() {
    let store = memory_store();
    let service = WorkflowService::new(&store);
    let mut first = WorkflowNode {
        mutates: true,
        compensation: Some("delete-output".to_string()),
        ..node("write", WorkflowNodeKind::Action, &[])
    };
    first.max_attempts = 2;
    let workflow = WorkflowDefinition {
        workflow_id: "bounded-failure".to_string(),
        nodes: vec![first, node("verify", WorkflowNodeKind::Compute, &["write"])],
    };
    let mut executor = Executor {
        outcomes: VecDeque::from([
            WorkflowStepOutcome::Retryable("busy".to_string()),
            WorkflowStepOutcome::Completed(json!(true)),
            WorkflowStepOutcome::TimedOut,
        ]),
        ..Executor::default()
    };
    let result = service.start("failure", workflow, &mut executor).unwrap();
    assert_eq!(result.status, WorkflowStatus::Compensated);
    assert_eq!(result.failure.as_deref(), Some("step_timeout"));
    assert_eq!(executor.calls, ["write:1", "write:2", "verify:1"]);
    assert_eq!(executor.compensations, ["write:delete-output"]);
}

#[test]
fn workflow_sources_stay_bounded() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/workflow.rs",
        include_str!("../src/workflow.rs"),
        380,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/workflow_validation.rs",
        include_str!("../src/workflow_validation.rs"),
        180,
    )
    .unwrap();
}
