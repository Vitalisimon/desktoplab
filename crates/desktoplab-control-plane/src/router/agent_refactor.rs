use std::path::Path;

use desktoplab_agent_engine::{
    MultiFileRefactorFile, MultiFileRefactorPlan, MultiFileRefactorRequest, RefactorPlanError,
};
use desktoplab_agent_session::SessionEvent;
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{GitToolExecutor, GitToolOutcome, ToolIntent};
use serde_json::{Value, json};

use super::agent_pending::PendingAgentAction;
use super::helpers::{body_field, body_field_or};
use super::{ApiRouteResponse, LocalApiRouter};

impl LocalApiRouter {
    pub(crate) fn create_multi_file_refactor_session(
        &mut self,
        body: &str,
        workspace_id: &str,
        backend_id: &str,
        prompt: &str,
    ) -> ApiRouteResponse {
        let session = self.create_bound_agent_session(workspace_id, backend_id);
        let session_id = session.session_id().to_string();
        self.agent_active_session_by_workspace
            .insert(workspace_id.to_string(), session_id.clone());
        self.persist_agent_active_sessions();

        let request = match refactor_request_from_body(body, prompt) {
            Ok(request) => request,
            Err(reason) => {
                self.sessions.block(&session_id, reason.as_str());
                return self.session_response(&session_id, workspace_id);
            }
        };
        let plan = match MultiFileRefactorPlan::from_request(request) {
            Ok(plan) => plan,
            Err(reason) => {
                self.sessions.block(&session_id, reason.as_str());
                return self.session_response(&session_id, workspace_id);
            }
        };

        self.sessions.start(&session_id);
        self.sessions.append_events(
            &session_id,
            &[
                SessionEvent::planning_started(format!(
                    "Multi-file refactor plan: {} across {} files",
                    plan.objective(),
                    plan.files().len()
                )),
                SessionEvent::tool_decision_recorded(format!(
                    "state=planned source=multi_file_refactor tool=refactor.files:{} approval_mode={}",
                    plan.files().len(),
                    self.session_approval_mode.as_str()
                )),
            ],
        );

        let checkpoint_id = match self.prepare_multi_file_refactor_checkpoint(&session_id, &plan) {
            Ok(checkpoint_id) => checkpoint_id,
            Err(reason) => {
                self.sessions.block(&session_id, &reason);
                return self.session_response(&session_id, workspace_id);
            }
        };

        self.sessions.append_events(
            &session_id,
            &[
                SessionEvent::backend_response_received(refactor_patch_set_summary(&plan)),
                SessionEvent::backend_response_received(refactor_diff_review_summary(&plan)),
                SessionEvent::backend_response_received(format!(
                    "Validation planned: {}",
                    plan.validation_command()
                )),
            ],
        );

        if let Err(reason) =
            self.request_multi_file_refactor_approval(&session_id, &plan, &checkpoint_id)
        {
            self.sessions.block(&session_id, &reason);
            return self.session_response(&session_id, workspace_id);
        }
        self.sessions.block(&session_id, "waiting for approval");
        self.session_response(&session_id, workspace_id)
    }

    fn session_response(&self, session_id: &str, workspace_id: &str) -> ApiRouteResponse {
        let session = self.sessions.get(session_id);
        ApiRouteResponse::ok(
            self.session_payload_with_pending_approvals(session.as_ref(), workspace_id),
        )
    }

    fn prepare_multi_file_refactor_checkpoint(
        &mut self,
        session_id: &str,
        plan: &MultiFileRefactorPlan,
    ) -> Result<String, String> {
        let workspace = self
            .workspace_record()
            .ok_or_else(|| "workspace_not_selected".to_string())?;
        let root = Path::new(&workspace.root_path);
        let checkpoint_ref = format!("checkpoint.agent.{session_id}.multi_file_refactor");
        let mut executor = GitToolExecutor::new(root, PolicyEngine::default_conservative());
        match executor.prepare_checkpoint_ref(checkpoint_ref) {
            GitToolOutcome::CheckpointReady(id) => {
                self.sessions.append_events(
                    session_id,
                    &[
                        SessionEvent::tool_decision_recorded(format!(
                            "state=checkpoint_ready checkpoint={id} status=ready"
                        )),
                        SessionEvent::backend_response_received(format!(
                            "Checkpoint ready before {}",
                            plan.checkpoint_label()
                        )),
                    ],
                );
                Ok(id)
            }
            GitToolOutcome::Blocked(reason) => Err(reason.to_string()),
            _ => Err("checkpoint_failed".to_string()),
        }
    }

    fn request_multi_file_refactor_approval(
        &mut self,
        session_id: &str,
        plan: &MultiFileRefactorPlan,
        checkpoint_id: &str,
    ) -> Result<(), String> {
        let content = Some(multi_file_patch_payload(plan).to_string());
        let tool = ToolIntent::filesystem_patch("multi-file patch set");
        let pending = PendingAgentAction::new(
            "approval.pending",
            session_id.to_string(),
            tool,
            content,
            false,
        )
        .with_checkpoint(checkpoint_id.to_string(), "ready");
        let approval = self.approvals.request_operation_with_payload_hash(
            session_id,
            "filesystem.write",
            "filesystem.patch:multi-file patch set",
            Some(pending.payload_hash().to_string()),
        );
        self.agent_pending_actions.insert(
            approval.id().to_string(),
            PendingAgentAction::new(
                approval.id().to_string(),
                session_id.to_string(),
                pending.tool().clone(),
                pending.content().map(ToString::to_string),
                false,
            )
            .with_checkpoint(checkpoint_id.to_string(), "ready"),
        );
        self.persist_agent_approval_journal()
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn refactor_request_from_body(
    body: &str,
    prompt: &str,
) -> Result<MultiFileRefactorRequest, RefactorPlanError> {
    let value: Value = serde_json::from_str(body).unwrap_or_else(|_| json!({}));
    let files = value
        .get("files")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(refactor_file_from_json)
        .collect::<Result<Vec<_>, _>>()?;
    let validation_command = body_field(body, "validationCommand")
        .or_else(|| body_field(body, "command"))
        .unwrap_or_default();
    Ok(MultiFileRefactorRequest::new(
        body_field_or(body, "objective", prompt),
        files,
        validation_command,
    ))
}

fn refactor_file_from_json(value: &Value) -> Result<MultiFileRefactorFile, RefactorPlanError> {
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or(RefactorPlanError::MissingPath)?;
    let expected = value
        .get("expected")
        .or_else(|| value.get("expectedOldContent"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let replacement = value
        .get("replacement")
        .or_else(|| value.get("newContent"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok(MultiFileRefactorFile::new(path, expected, replacement))
}

fn refactor_patch_set_summary(plan: &MultiFileRefactorPlan) -> String {
    let paths = plan.patch_summaries().join("; ");
    format!(
        "Bounded patch set ready: files={} max_files={} patches=[{}]",
        plan.files().len(),
        MultiFileRefactorPlan::MAX_FILES,
        paths
    )
}

fn refactor_diff_review_summary(plan: &MultiFileRefactorPlan) -> String {
    format!(
        "Diff review required before approval: {}",
        plan.files()
            .iter()
            .map(MultiFileRefactorFile::path)
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn multi_file_patch_payload(plan: &MultiFileRefactorPlan) -> Value {
    json!({
        "desktoplabMultiFilePatch":true,
        "files":plan.files().iter().map(|file| json!({
            "path":file.path(),
            "expected":file.expected(),
            "replacement":file.replacement(),
            "expectedBytes":file.expected().len(),
            "replacementBytes":file.replacement().len()
        })).collect::<Vec<_>>()
    })
}
