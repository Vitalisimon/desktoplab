use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::workflow_validation;

const WORKFLOW_SCHEMA_VERSION: u32 = 1;
const MAX_STATE_BYTES: usize = 512 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowNodeKind {
    Action,
    Compute,
    Agent,
    Decision,
    Approval,
    Checkpoint,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNode {
    pub id: String,
    pub kind: WorkflowNodeKind,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub condition_node: Option<String>,
    #[serde(default)]
    pub mutates: bool,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub compensation: Option<String>,
    pub max_attempts: u8,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDefinition {
    pub workflow_id: String,
    pub nodes: Vec<WorkflowNode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Running,
    WaitingApproval,
    Completed,
    Failed,
    Compensated,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowRunState {
    schema_version: u32,
    run_id: String,
    definition: WorkflowDefinition,
    status: WorkflowStatus,
    completed: BTreeMap<String, Value>,
    attempts: BTreeMap<String, u8>,
    waiting_node: Option<String>,
    resume_token_hash: Option<String>,
    failure: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowGraph {
    pub nodes: Vec<(String, WorkflowNodeKind)>,
    pub edges: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowProgress {
    pub run_id: String,
    pub status: WorkflowStatus,
    pub completed_nodes: Vec<String>,
    pub resume_token: Option<String>,
    pub failure: Option<String>,
    pub graph: WorkflowGraph,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkflowStepOutcome {
    Completed(Value),
    Retryable(String),
    Failed(String),
    TimedOut,
}

pub trait WorkflowExecutor {
    fn execute(&mut self, node: &WorkflowNode, attempt: u8) -> WorkflowStepOutcome;
    fn compensate(&mut self, node: &WorkflowNode, compensation: &str) -> Result<(), String>;
}

#[derive(Debug)]
pub enum WorkflowError {
    InvalidGraph(&'static str),
    NotFound,
    InvalidResumeToken,
    NotWaitingForApproval,
    StateTooLarge,
    Persistence(String),
}

pub struct WorkflowService<'a> {
    storage: &'a SqliteStore,
}

impl<'a> WorkflowService<'a> {
    pub fn new(storage: &'a SqliteStore) -> Self {
        Self { storage }
    }

    pub fn start(
        &self,
        run_id: impl Into<String>,
        definition: WorkflowDefinition,
        executor: &mut impl WorkflowExecutor,
    ) -> Result<WorkflowProgress, WorkflowError> {
        workflow_validation::validate(&definition)?;
        let run_id = run_id.into();
        if run_id.is_empty() || run_id.len() > 96 {
            return Err(WorkflowError::InvalidGraph("invalid_run_id"));
        }
        let mut state = WorkflowRunState {
            schema_version: WORKFLOW_SCHEMA_VERSION,
            run_id,
            definition,
            status: WorkflowStatus::Running,
            completed: BTreeMap::new(),
            attempts: BTreeMap::new(),
            waiting_node: None,
            resume_token_hash: None,
            failure: None,
        };
        self.persist(&state)?;
        self.drive(&mut state, executor)
    }

    pub fn resume(
        &self,
        run_id: &str,
        token: &str,
        approved: bool,
        executor: &mut impl WorkflowExecutor,
    ) -> Result<WorkflowProgress, WorkflowError> {
        let mut state = self.load(run_id)?;
        if state.status != WorkflowStatus::WaitingApproval {
            return Err(WorkflowError::NotWaitingForApproval);
        }
        if state.resume_token_hash.as_deref() != Some(&hash(token)) {
            return Err(WorkflowError::InvalidResumeToken);
        }
        let node_id = state
            .waiting_node
            .take()
            .ok_or(WorkflowError::NotWaitingForApproval)?;
        state.resume_token_hash = None;
        if !approved {
            state.status = WorkflowStatus::Failed;
            state.failure = Some("approval_denied".to_string());
            self.persist(&state)?;
            return Ok(progress(&state, None));
        }
        state.completed.insert(node_id, Value::Bool(true));
        state.status = WorkflowStatus::Running;
        self.persist(&state)?;
        self.drive(&mut state, executor)
    }

    pub fn inspect(&self, run_id: &str) -> Result<WorkflowProgress, WorkflowError> {
        self.load(run_id).map(|state| progress(&state, None))
    }

    fn drive(
        &self,
        state: &mut WorkflowRunState,
        executor: &mut impl WorkflowExecutor,
    ) -> Result<WorkflowProgress, WorkflowError> {
        loop {
            let Some(node) = next_ready_node(state) else {
                state.status = WorkflowStatus::Completed;
                self.persist(state)?;
                return Ok(progress(state, None));
            };
            if node.condition_node.as_ref().is_some_and(|id| {
                state
                    .completed
                    .get(id)
                    .is_some_and(|value| value == &Value::Bool(false))
            }) {
                state.completed.insert(node.id.clone(), Value::Null);
                self.persist(state)?;
                continue;
            }
            if node.kind == WorkflowNodeKind::Approval {
                let token = resume_token(&state.run_id, &node.id);
                state.status = WorkflowStatus::WaitingApproval;
                state.waiting_node = Some(node.id.clone());
                state.resume_token_hash = Some(hash(&token));
                self.persist(state)?;
                return Ok(progress(state, Some(token)));
            }
            let attempt = state.attempts.get(&node.id).copied().unwrap_or(0) + 1;
            state.attempts.insert(node.id.clone(), attempt);
            self.persist(state)?;
            match executor.execute(&node, attempt) {
                WorkflowStepOutcome::Completed(value) => {
                    state.completed.insert(node.id.clone(), value);
                    state.failure = None;
                    self.persist(state)?;
                }
                WorkflowStepOutcome::Retryable(reason) if attempt < node.max_attempts => {
                    state.failure = Some(reason);
                    self.persist(state)?;
                }
                WorkflowStepOutcome::Retryable(reason) | WorkflowStepOutcome::Failed(reason) => {
                    return self.fail_and_compensate(state, &reason, executor);
                }
                WorkflowStepOutcome::TimedOut => {
                    return self.fail_and_compensate(state, "step_timeout", executor);
                }
            }
        }
    }

    fn fail_and_compensate(
        &self,
        state: &mut WorkflowRunState,
        reason: &str,
        executor: &mut impl WorkflowExecutor,
    ) -> Result<WorkflowProgress, WorkflowError> {
        state.status = WorkflowStatus::Failed;
        state.failure = Some(reason.to_string());
        let completed = state
            .definition
            .nodes
            .iter()
            .rev()
            .filter(|node| state.completed.contains_key(&node.id));
        let mut compensated = false;
        for node in completed {
            if let Some(compensation) = &node.compensation {
                executor
                    .compensate(node, compensation)
                    .map_err(WorkflowError::Persistence)?;
                compensated = true;
            }
        }
        if compensated {
            state.status = WorkflowStatus::Compensated;
        }
        self.persist(state)?;
        Ok(progress(state, None))
    }

    fn persist(&self, state: &WorkflowRunState) -> Result<(), WorkflowError> {
        let payload = serde_json::to_string(state)
            .map_err(|error| WorkflowError::Persistence(error.to_string()))?;
        if payload.len() > MAX_STATE_BYTES {
            return Err(WorkflowError::StateTooLarge);
        }
        self.storage
            .put_productization_state(ProductizationStateRecord::new(
                ProductizationRecordKind::WorkflowRun,
                &state.run_id,
                payload,
            ))
            .map_err(|error| WorkflowError::Persistence(error.to_string()))
    }

    fn load(&self, run_id: &str) -> Result<WorkflowRunState, WorkflowError> {
        let record = self
            .storage
            .get_productization_state(ProductizationRecordKind::WorkflowRun, run_id)
            .map_err(|error| WorkflowError::Persistence(error.to_string()))?
            .ok_or(WorkflowError::NotFound)?;
        let state: WorkflowRunState = serde_json::from_str(record.payload())
            .map_err(|error| WorkflowError::Persistence(error.to_string()))?;
        if state.schema_version != WORKFLOW_SCHEMA_VERSION {
            return Err(WorkflowError::Persistence(
                "unsupported_workflow_schema".to_string(),
            ));
        }
        Ok(state)
    }
}

fn next_ready_node(state: &WorkflowRunState) -> Option<WorkflowNode> {
    state
        .definition
        .nodes
        .iter()
        .find(|node| {
            !state.completed.contains_key(&node.id)
                && state.waiting_node.as_deref() != Some(&node.id)
                && node
                    .dependencies
                    .iter()
                    .all(|dependency| state.completed.contains_key(dependency))
        })
        .cloned()
}

fn progress(state: &WorkflowRunState, resume_token: Option<String>) -> WorkflowProgress {
    WorkflowProgress {
        run_id: state.run_id.clone(),
        status: state.status.clone(),
        completed_nodes: state.completed.keys().cloned().collect(),
        resume_token,
        failure: state.failure.clone(),
        graph: WorkflowGraph {
            nodes: state
                .definition
                .nodes
                .iter()
                .map(|node| (node.id.clone(), node.kind.clone()))
                .collect(),
            edges: state
                .definition
                .nodes
                .iter()
                .flat_map(|node| {
                    node.dependencies
                        .iter()
                        .map(|dependency| (dependency.clone(), node.id.clone()))
                })
                .collect(),
        },
    }
}

fn resume_token(run_id: &str, node_id: &str) -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    format!("wf1.{}", hash(&format!("{run_id}:{node_id}:{nonce}")))
}

fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
