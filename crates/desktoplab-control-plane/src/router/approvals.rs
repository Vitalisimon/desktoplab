use desktoplab_backend_services::ApprovalResolution;
use desktoplab_storage::StorageError;
use serde_json::{Value, json};

use super::dispatch::AgentContinuationMode;
use super::helpers::{
    approval_json, approval_payload_hash_from_payload, body_field, body_field_or, segment,
};
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn approved_git_response(
        &mut self,
        body: &str,
        action: &str,
        operation_id: String,
        approved: Value,
    ) -> ApiRouteResponse {
        match self.consume_body_approved_record(
            body,
            &body_field_or(body, "sessionId", "session.local"),
            action,
            &operation_id,
            None,
        ) {
            Ok(true) => return ApiRouteResponse::ok(approved),
            Err(error) => return ApiRouteResponse::state_journal_failed(error),
            Ok(false) => {}
        }
        ApiRouteResponse::ok(json!({"status":"blocked","reason":"approval_required"}))
    }

    pub(crate) fn consume_body_approved_record(
        &mut self,
        body: &str,
        session_id: &str,
        action: &str,
        operation_id: &str,
        payload_hash: Option<&str>,
    ) -> Result<bool, StorageError> {
        let Some(approval_id) = body_field(body, "approvalId") else {
            return Ok(false);
        };
        let approvals_before = self.approvals.list();
        let consumed = self.approvals.consume_approved_for_payload(
            &approval_id,
            session_id,
            action,
            operation_id,
            payload_hash,
        );
        if consumed && let Err(error) = self.persist_agent_approval_journal() {
            self.approvals =
                desktoplab_backend_services::ApprovalService::from_records(approvals_before);
            return Err(error);
        }
        Ok(consumed)
    }

    pub(crate) fn create_approval(&mut self, body: &str) -> ApiRouteResponse {
        let session_id = body_field_or(body, "sessionId", "session.local");
        let action = body_field_or(body, "action", "local.action");
        let operation_id = body_field_or(body, "operationId", &action);
        let payload_hash = approval_payload_hash_from_payload(body);
        let approvals_before = self.approvals.list();
        let approval = self.approvals.request_operation_with_payload_hash(
            session_id,
            action,
            operation_id,
            payload_hash,
        );
        if let Err(error) = self.persist_agent_approval_journal() {
            self.approvals =
                desktoplab_backend_services::ApprovalService::from_records(approvals_before);
            return ApiRouteResponse::state_journal_failed(error);
        }
        ApiRouteResponse::ok(approval_json(&approval))
    }

    pub(crate) fn list_approvals(&self) -> ApiRouteResponse {
        let approvals = self
            .approvals
            .list()
            .iter()
            .map(approval_json)
            .collect::<Vec<_>>();
        ApiRouteResponse::ok(json!({"source":"service_backed","approvals":approvals}))
    }

    pub(crate) fn resolve_approval(
        &mut self,
        path: &str,
        body: &str,
        continuation: AgentContinuationMode,
    ) -> ApiRouteResponse {
        let approval_id = segment(path, 2);
        let resolution = match body_field_or(body, "resolution", "deny").as_str() {
            "approve" | "approved" => ApprovalResolution::Approve,
            _ => ApprovalResolution::Deny,
        };
        let approvals_before = self.approvals.list();
        match self.approvals.resolve(&approval_id, resolution) {
            Ok(approval) => {
                if let Err(error) = self.persist_agent_approval_journal() {
                    self.approvals = desktoplab_backend_services::ApprovalService::from_records(
                        approvals_before,
                    );
                    return ApiRouteResponse::state_journal_failed(error);
                }
                if let Some(pending) = self.agent_pending_actions.get(&approval_id)
                    && (resolution == ApprovalResolution::Deny
                        || continuation == AgentContinuationMode::Immediate)
                {
                    let session_id = pending.session_id().to_string();
                    let session = self.sessions.get(&session_id);
                    let workspace_id = self.sessions.workspace_id_for(&session_id);
                    if let (Some(session), Some(workspace_id)) = (session, workspace_id) {
                        let _ = self.continue_pending_agent_action(
                            &workspace_id,
                            session.execution_backend_id(),
                            &approval_id,
                        );
                    } else {
                        self.fail_pending_execution_identity(
                            &approval_id,
                            &session_id,
                            "execution_identity_unavailable",
                        );
                    }
                }
                let resolved = self
                    .approvals
                    .get(&approval_id)
                    .map(|record| approval_json(&record))
                    .unwrap_or_else(|| approval_json(&approval));
                ApiRouteResponse::ok(resolved)
            }
            Err("approval_missing") => ApiRouteResponse::not_found(),
            Err(code) => ApiRouteResponse::bad_request(json!({
                "code":code.to_ascii_uppercase(),
                "message":"The approval can no longer be changed."
            })),
        }
    }
}
