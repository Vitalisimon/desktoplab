use std::collections::BTreeMap;

use desktoplab_agent_session::SessionState;
use desktoplab_storage::{ProductizationRecordKind, SqliteStore, StorageError};
use serde_json::{Value, json};

use super::dispatch::AgentContinuationMode;
use super::helpers::{body_field_or, segment};
use super::{ApiRouteResponse, LocalApiRouter};

const MAX_ACTIVE_CHILDREN: usize = 6;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubagentRecord {
    child_session_id: String,
    parent_session_id: String,
    workspace_id: String,
    intent: String,
    closed: bool,
}

impl SubagentRecord {
    fn new(child: &str, parent: &str, workspace: &str, intent: &str) -> Self {
        Self {
            child_session_id: child.to_string(),
            parent_session_id: parent.to_string(),
            workspace_id: workspace.to_string(),
            intent: intent.to_string(),
            closed: false,
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "childSessionId":self.child_session_id,
            "parentSessionId":self.parent_session_id,
            "workspaceId":self.workspace_id,
            "intent":self.intent,
            "closed":self.closed
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self {
            child_session_id: value.get("childSessionId")?.as_str()?.to_string(),
            parent_session_id: value.get("parentSessionId")?.as_str()?.to_string(),
            workspace_id: value.get("workspaceId")?.as_str()?.to_string(),
            intent: value.get("intent")?.as_str()?.to_string(),
            closed: value
                .get("closed")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }

    pub(super) fn belongs_to(&self, parent_session_id: &str) -> bool {
        self.parent_session_id == parent_session_id
    }
}

pub(crate) fn subagent_payload(records: &BTreeMap<String, SubagentRecord>) -> Value {
    json!({"subagents":records.values().map(SubagentRecord::to_json).collect::<Vec<_>>()})
}

pub(crate) fn load_subagents(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, SubagentRecord>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::SubagentSession, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("subagents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(SubagentRecord::from_json)
        .map(|record| (record.child_session_id.clone(), record))
        .collect())
}

impl LocalApiRouter {
    pub(crate) fn spawn_subagent(&mut self, body: &str) -> ApiRouteResponse {
        let parent_id = body_field_or(body, "parentSessionId", "");
        let prompt = body_field_or(body, "prompt", "");
        let intent = body_field_or(body, "intent", "read_only");
        let Some(parent) = self.sessions.get(&parent_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"PARENT_SESSION_NOT_FOUND",
                "message":"A subagent must belong to an existing DesktopLab session."
            }));
        };
        if prompt.trim().is_empty() || !matches!(intent.as_str(), "read_only" | "write_capable") {
            return ApiRouteResponse::bad_request(json!({
                "code":"INVALID_SUBAGENT_REQUEST",
                "message":"Provide a prompt and a read_only or write_capable intent."
            }));
        }
        let active_children = self
            .subagents
            .values()
            .filter(|record| record.parent_session_id == parent_id && !record.closed)
            .count();
        if active_children >= MAX_ACTIVE_CHILDREN {
            return ApiRouteResponse::bad_request(json!({
                "code":"SUBAGENT_LIMIT_REACHED",
                "message":"Close an existing child session before spawning another."
            }));
        }
        let Some(workspace_id) = self.sessions.workspace_id_for(&parent_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"PARENT_WORKSPACE_NOT_FOUND",
                "message":"The parent session no longer has a workspace."
            }));
        };
        let backend_id = parent.execution_backend_id().to_string();
        let response = self.create_session(
            &json!({
                "workspaceId":workspace_id,
                "executionBackendId":backend_id,
                "initialPrompt":prompt,
                "stream":true,
                "newChat":true,
                "parentSessionId":parent_id
            })
            .to_string(),
            AgentContinuationMode::Deferred,
        );
        if response.status() != "200 OK" {
            return response;
        }
        let Ok(payload) = serde_json::from_str::<Value>(response.body()) else {
            return ApiRouteResponse::bad_request(json!({"code":"SUBAGENT_CREATE_FAILED"}));
        };
        let Some(child_id) = payload.get("sessionId").and_then(Value::as_str) else {
            return ApiRouteResponse::bad_request(json!({"code":"SUBAGENT_CREATE_FAILED"}));
        };
        let child_id = child_id.to_string();
        self.agent_active_session_by_workspace
            .insert(workspace_id.clone(), parent_id.clone());
        self.persist_agent_active_sessions();
        if let Some(error) = self.state_journal_failure() {
            self.sessions
                .cancel(&child_id, "parent_session_restore_failed");
            return ApiRouteResponse::state_journal_failed(error);
        }
        if intent == "write_capable" {
            let isolation = self.create_agent_worktree(
                &json!({"sessionId":child_id,"intent":"write_capable"}).to_string(),
            );
            let isolated = serde_json::from_str::<Value>(isolation.body())
                .ok()
                .is_some_and(|value| value.get("status") == Some(&Value::String("ready".into())));
            if !isolated {
                self.sessions
                    .cancel(&child_id, "subagent_worktree_creation_failed");
                return ApiRouteResponse::bad_request(json!({
                    "code":"SUBAGENT_ISOLATION_FAILED",
                    "message":"The write-capable child session could not be isolated."
                }));
            }
        }
        self.subagents.insert(
            child_id.clone(),
            SubagentRecord::new(&child_id, &parent_id, &workspace_id, &intent),
        );
        self.persist_subagents();
        if let Some(error) = self.state_journal_failure() {
            return ApiRouteResponse::state_journal_failed(error);
        }
        self.events.publish_agent_event(
            "agent.subagent.spawned",
            &workspace_id,
            &child_id,
            &backend_id,
            "Child agent session created",
        );
        self.subagent_status(&child_id)
    }

    pub(crate) fn subagent_route(
        &mut self,
        method: &str,
        path: &str,
        body: &str,
    ) -> ApiRouteResponse {
        let child_id = segment(path, 3);
        let Some(record) = self.subagents.get(&child_id).cloned() else {
            return ApiRouteResponse::not_found();
        };
        if method == "GET" {
            return self.subagent_status(&child_id);
        }
        if path.ends_with("/messages") && method == "POST" && !record.closed {
            let Some(session) = self.sessions.get(&child_id) else {
                return ApiRouteResponse::not_found();
            };
            let backend_id = session.execution_backend_id().to_string();
            return self.continue_session(
                &format!("/v1/sessions/{child_id}/messages"),
                &json!({
                    "workspaceId":record.workspace_id,
                    "executionBackendId":backend_id,
                    "prompt":body_field_or(body, "prompt", "Continue the delegated task")
                })
                .to_string(),
                AgentContinuationMode::Deferred,
            );
        }
        if path.ends_with("/cancel") && method == "POST" && !record.closed {
            return self.session_control(
                &format!("/v1/sessions/{child_id}/control"),
                r#"{"action":"cancel"}"#,
            );
        }
        if path.ends_with("/close") && method == "POST" {
            let terminal = self.sessions.get(&child_id).is_some_and(|session| {
                matches!(
                    session.state(),
                    SessionState::Completed | SessionState::Failed | SessionState::Cancelled
                )
            });
            if !terminal {
                return ApiRouteResponse::bad_request(json!({
                    "code":"SUBAGENT_NOT_TERMINAL",
                    "message":"Cancel or wait for the child session before closing it."
                }));
            }
            if let Some(record) = self.subagents.get_mut(&child_id) {
                record.closed = true;
            }
            self.persist_subagents();
            return self.subagent_status(&child_id);
        }
        ApiRouteResponse::bad_request(json!({"code":"INVALID_SUBAGENT_OPERATION"}))
    }

    fn subagent_status(&self, child_id: &str) -> ApiRouteResponse {
        let Some(record) = self.subagents.get(child_id) else {
            return ApiRouteResponse::not_found();
        };
        let session = self.sessions.get(child_id);
        let terminal = session.as_ref().is_some_and(|session| {
            matches!(
                session.state(),
                SessionState::Completed | SessionState::Failed | SessionState::Cancelled
            )
        });
        let change_review = (record.intent == "write_capable").then(|| {
            self.worktree_bindings
                .get(child_id)
                .map(|binding| super::subagent_change_review::review(binding, terminal))
                .unwrap_or_else(|| {
                    json!({
                        "status":"blocked",
                        "readyToIntegrate":false,
                        "reason":"managed_worktree_missing"
                    })
                })
        });
        ApiRouteResponse::ok(json!({
            "subagentId":child_id,
            "parentSessionId":record.parent_session_id,
            "workspaceId":record.workspace_id,
            "intent":record.intent,
            "closed":record.closed,
            "state":session.as_ref().map(|session| format!("{:?}", session.state()).to_ascii_lowercase()).unwrap_or_else(|| "missing".to_string()),
            "summary":session.as_ref().and_then(|session| session.summary()),
            "worktree":self.worktree_bindings.get(child_id).map(|binding| binding.worktree_root()),
            "changeReview":change_review
        }))
    }
}
