use desktoplab_agent_session::AgentSession;
use desktoplab_backend_services::ApprovalState;
use serde_json::{Value, json};

use crate::router_payloads as payloads;

use super::helpers::approval_json;
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn session_payload_with_pending_approvals(
        &self,
        session: Option<&AgentSession>,
        workspace_id: &str,
    ) -> Value {
        let pending_approvals = session
            .map(|session| {
                self.approvals
                    .list()
                    .iter()
                    .filter(|approval| {
                        approval.state() == ApprovalState::Pending
                            && approval.session_id() == session.session_id()
                    })
                    .map(|approval| {
                        let mut payload = approval_json(approval);
                        if let Some(details) = self
                            .agent_pending_actions
                            .get(approval.id())
                            .and_then(|pending| pending.approval_details())
                            && let Some(object) = payload.as_object_mut()
                        {
                            object.insert("details".to_string(), details);
                        }
                        payload
                    })
                    .collect()
            })
            .unwrap_or_default();
        let trace = session.and_then(|session| self.sessions.trace(session.session_id()));
        let mut payload = payloads::session_with_pending_approvals(
            session,
            workspace_id,
            pending_approvals,
            trace.as_ref(),
        );
        if let (Some(session), Some(object)) = (session, payload.as_object_mut()) {
            object.insert(
                "queuedTurns".to_string(),
                Value::Array(
                    self.sessions
                        .queued_turns(session.session_id())
                        .iter()
                        .map(|turn| {
                            json!({
                                "turnId":turn.turn_id(),"prompt":turn.prompt(),"state":turn.state()
                            })
                        })
                        .collect(),
                ),
            );
            object.insert(
                "cancellationState".to_string(),
                self.sessions
                    .cancellation_state(session.session_id())
                    .map(Value::String)
                    .unwrap_or(Value::Null),
            );
        }
        payload
    }

    pub(crate) fn agent_workspace(&mut self) -> ApiRouteResponse {
        if self.readiness.model_capabilities().is_none()
            && self.readiness.runtime_id() == Some("runtime.ollama")
            && let Some(model_id) = self.readiness.model_id().map(ToString::to_string)
            && let Some(pull_ref) = crate::model_routes::model_pull_ref(&model_id)
        {
            self.refresh_ollama_model_capabilities("runtime.ollama", &pull_ref);
            self.persist_readiness_state();
        }
        if !self.setup.is_ready() {
            return ApiRouteResponse::ok(json!({
                "route":{
                    "source":"service_backed",
                    "status":"blocked",
                    "backendId":Value::Null,
                    "backendDisplayName":Value::Null,
                    "backendKind":Value::Null,
                    "summary":"Setup must finish before the agent can start.",
                    "reasons":[],
                    "blockedReasons":["setup_not_ready"],
                    "nextAction":"complete_setup",
                    "nextActionLabel":"Finish setup",
                    "requiredCapabilities":["llm.chat","tools.filesystem.read"],
                    "needsFallbackApproval":false
                },
                "context":Value::Null,
                "session":Value::Null
            }));
        }
        let Some(workspace_id) = self.workspace_id() else {
            return ApiRouteResponse::ok(json!({
                "route":{
                    "source":"service_backed",
                    "status":"blocked",
                    "backendId":Value::Null,
                    "backendDisplayName":Value::Null,
                    "backendKind":Value::Null,
                    "summary":"Open a repository before starting the agent.",
                    "reasons":[],
                    "blockedReasons":["workspace_not_selected"],
                    "nextAction":"open_workspace",
                    "nextActionLabel":"Open repository",
                    "requiredCapabilities":["llm.chat","tools.filesystem.read"],
                    "needsFallbackApproval":false
                },
                "context":Value::Null,
                "session":Value::Null
            }));
        };
        let sessions = self
            .sessions
            .list_by_workspace(&workspace_id)
            .into_iter()
            .filter(|session| !self.archived_session_ids.contains(session.session_id()))
            .collect::<Vec<_>>();
        let session = self
            .agent_active_session_by_workspace
            .get(&workspace_id)
            .and_then(|session_id| {
                sessions
                    .iter()
                    .find(|session| session.session_id() == session_id)
                    .cloned()
            })
            .or_else(|| sessions.last().cloned());
        let session_json = session.as_ref().map(|session| {
            self.session_payload_with_pending_approvals(Some(session), &workspace_id)
        });
        let (local_route_ready, local_blocked_reason) = self.local_agent_readiness();
        ApiRouteResponse::ok(json!({
            "route":crate::execution_routes::route_response_for_selection_with_readiness(
                &self.selected_route_id,
                "",
                "",
                local_route_ready,
                local_blocked_reason,
                self.readiness.runtime_id(),
                self.readiness.model_id(),
                self.readiness.model_capabilities(),
                self.codex_bridge_ready(),
            ),
            "context":payloads::context(workspace_id),
            "session":session_json
        }))
    }
}
