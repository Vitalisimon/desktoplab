use desktoplab_backend_services::{AuditAction, BackendEventScope, JobState};
use serde_json::{Value, json};

use crate::setup_pipeline::{SetupPipeline, SetupPipelineState};
use crate::setup_state::SetupState;

use super::helpers::{segment, setup_accept_selection, valid_model_for_runtime, valid_runtime_id};
use super::{ApiRouteResponse, LocalApiRouter};

mod ownership;
mod workspace_open;

impl LocalApiRouter {
    pub(crate) fn accept_setup_plan(&mut self, body: &str) -> ApiRouteResponse {
        let selection = setup_accept_selection(body);
        let (Some(runtime_id), Some(model_id)) = (selection.runtime_id, selection.model_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"SETUP_SELECTION_REQUIRED",
                "message":"runtimeId and modelId are required"
            }));
        };
        if !valid_runtime_id(&runtime_id) || !valid_model_for_runtime(&model_id, &runtime_id) {
            return ApiRouteResponse::bad_request(json!({
                "code":"SETUP_SELECTION_INCOMPATIBLE",
                "message":"Selected runtime and model are not compatible with the current catalog."
            }));
        }
        if self.setup.is_ready() && self.readiness.is_ready() {
            return ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "setup":self.setup.to_json(),
                "pipeline":self.setup_pipeline.to_json(),
                "readinessEvidence":self.readiness.to_json(),
                "startedJobIds":[],
                "jobs":[],
                "message":"Local setup is already verified."
            }));
        }
        self.audit.record(
            AuditAction::RuntimeInstall,
            format!("setup.accept runtime_id={runtime_id} model_id={model_id}"),
        );
        self.setup = SetupState::accept(runtime_id.clone(), model_id.clone());
        self.setup_pipeline = SetupPipeline::select(runtime_id.clone(), model_id.clone());
        self.readiness = self.readiness.clone().select(runtime_id, model_id.clone());
        let mut started_job_ids = Vec::new();
        let mut jobs = Vec::new();
        let runtime_job = self.jobs.create_job("runtime.install");
        let _ = self.jobs.start(runtime_job.id());
        self.setup_pipeline = self
            .setup_pipeline
            .clone()
            .advance(SetupPipelineState::RuntimeInstalling);
        self.events.publish_runtime_install_progress(
            runtime_job.id().as_str(),
            "running",
            5,
            "unknown",
            "",
        );
        started_job_ids.push(runtime_job.id().to_string());
        jobs.push(json!({
            "jobId":runtime_job.id().as_str(),
            "kind":"runtime.install",
            "state":"running",
            "pipelineState":"runtime_installing"
        }));
        let model_job = self.jobs.create_job("model.download");
        let _ = self
            .jobs
            .block_with_message(model_job.id(), "runtime_not_ready");
        self.events.publish_model_download_progress(
            model_job.id().as_str(),
            &model_id,
            "blocked",
            0,
            "non_retryable",
            "runtime_not_ready",
        );
        started_job_ids.push(model_job.id().to_string());
        jobs.push(json!({
            "jobId":model_job.id().as_str(),
            "kind":"model.download",
            "state":"blocked",
            "blockedReason":"runtime_not_ready",
            "pipelineState":"runtime_installing",
            "dependsOn":runtime_job.id().as_str()
        }));
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.persist_readiness_state();
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "setup":self.setup.to_json(),
            "pipeline":self.setup_pipeline.to_json(),
            "readinessEvidence":self.readiness.to_json(),
            "startedJobIds":started_job_ids,
            "jobs":jobs
        }))
    }

    pub(crate) fn refresh_catalog(&mut self) -> ApiRouteResponse {
        let job = self.jobs.create_job("registry.refresh");
        let _ = self.jobs.start(job.id());
        let _ = self.jobs.succeed(job.id());
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "jobId":job.id().as_str(),
            "state":"completed",
            "catalogSource":"bundled_seed_catalog"
        }))
    }

    pub(crate) fn complete_setup(&mut self, body: &str) -> ApiRouteResponse {
        let runtime_ready = self.readiness.runtime_verified();
        let model_ready = self.readiness.model_verified();
        self.setup = self.setup.clone().complete(runtime_ready, model_ready);
        if runtime_ready && model_ready {
            self.setup_pipeline = self.setup_pipeline.clone().ready();
            for job in self.jobs.list_jobs() {
                if job.kind() == "runtime.install" || job.kind().starts_with("model.download") {
                    let _ = self.jobs.succeed(job.id());
                }
            }
            self.persist_runtime_jobs();
            self.persist_model_jobs();
        } else {
            self.setup_pipeline = self
                .setup_pipeline
                .clone()
                .block(self.readiness.blocked_reason().unwrap_or("setup_not_ready"));
        }
        self.audit.record(
            AuditAction::PolicyDecision,
            format!(
                "setup.complete derived_readiness runtime_ready={runtime_ready} model_ready={model_ready} ignored_client_body={}",
                !body.trim().is_empty()
            ),
        );
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.app_state()
    }

    pub(crate) fn runtime_install(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let setup_choice = match crate::runtime_routes::runtime_setup_choice(body) {
            Ok(choice) => choice,
            Err(error) => {
                return ApiRouteResponse::ok(
                    crate::runtime_routes::runtime_install_blocked_response(path, error),
                );
            }
        };
        match crate::runtime_routes::plan_runtime_install(path, body) {
            Ok((runtime_id, _planned_job)) => {
                let job = self.jobs.create_job("runtime.install");
                self.audit.record(
                    AuditAction::RuntimeInstall,
                    format!("runtime.install requested runtime_id={runtime_id} body={body}"),
                );
                let _ = self.jobs.start(job.id());
                let result =
                    crate::runtime_routes::execute_runtime_install(&runtime_id, setup_choice);
                match result.state() {
                    desktoplab_runtime::RuntimeExecutionState::Completed => {
                        let _ = self.jobs.succeed(job.id());
                        if result.verification_state() == "verified" {
                            self.readiness
                                .mark_runtime_verified(runtime_id.clone(), result.evidence());
                            ownership::record_desktoplab_runtime_ownership(
                                self.managed_runtime_marker_path.as_deref(),
                                self.managed_runtime_owner_id.as_deref(),
                                &runtime_id,
                                &result,
                            );
                            self.setup_pipeline = self
                                .setup_pipeline
                                .clone()
                                .advance(SetupPipelineState::RuntimeVerifying);
                            self.persist_readiness_state();
                            self.persist_setup_pipeline();
                        }
                    }
                    desktoplab_runtime::RuntimeExecutionState::Blocked
                    | desktoplab_runtime::RuntimeExecutionState::ExternalGuided => {
                        let _ = self.jobs.block_with_message(job.id(), result.remediation());
                        self.block_runtime_setup(runtime_id.clone(), result.remediation());
                    }
                    desktoplab_runtime::RuntimeExecutionState::Failed => {
                        let _ = self.jobs.fail_with_message(
                            job.id(),
                            desktoplab_backend_services::JobRetryClass::Retryable,
                            result.remediation(),
                        );
                        self.block_runtime_setup(runtime_id.clone(), result.remediation());
                    }
                }
                self.publish_runtime_install_phases(job.id().as_str(), &result);
                self.persist_runtime_jobs();
                ApiRouteResponse::ok(crate::runtime_routes::runtime_install_response(
                    &runtime_id,
                    job.id().as_str(),
                    setup_choice,
                    &result,
                ))
            }
            Err(error) => {
                let runtime_id = segment(path, 2);
                self.block_runtime_setup(
                    runtime_id,
                    crate::runtime_routes::runtime_install_error_blocked_reason(&error),
                );
                ApiRouteResponse::ok(crate::runtime_routes::runtime_install_blocked_response(
                    path, error,
                ))
            }
        }
    }

    pub(crate) fn runtime_verify(&mut self, path: &str, _body: &str) -> ApiRouteResponse {
        let runtime_id = segment(path, 2);
        let verification = self
            .runtime_verification_for_test
            .as_ref()
            .map(|fixture| {
                (
                    fixture.verified,
                    fixture.evidence.clone(),
                    fixture.blocked_reason.clone(),
                )
            })
            .unwrap_or_else(|| {
                let result = crate::runtime_routes::verify_runtime(&runtime_id);
                (
                    result.verification_state() == "verified",
                    result.evidence().to_string(),
                    if result.remediation().is_empty() {
                        result.verification_state().to_string()
                    } else {
                        result.remediation().to_string()
                    },
                )
            });
        let (verified, evidence, blocked_reason) = verification;
        if verified {
            self.readiness
                .mark_runtime_verified(runtime_id.clone(), evidence.clone());
            self.persist_readiness_state();
            self.events.publish_json(
                BackendEventScope::Job,
                json!({
                    "kind":"runtime.verify","runtimeId":runtime_id,
                    "state":"verified","evidence":evidence
                }),
            );
        } else {
            self.readiness
                .mark_runtime_blocked(runtime_id.clone(), blocked_reason.clone());
            self.persist_readiness_state();
            self.events.publish_json(
                BackendEventScope::Job,
                json!({
                    "kind":"runtime.verify","runtimeId":runtime_id,
                    "state":"blocked","failureReason":blocked_reason
                }),
            );
        }
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "runtimeId":runtime_id,
            "verificationState":if verified {"verified"} else {"blocked"},
            "blockedReason":if verified {Value::Null} else {json!(blocked_reason)},
            "readinessEvidence":self.readiness.to_json()
        }))
    }

    fn block_runtime_setup(&mut self, runtime_id: String, reason: impl Into<String>) {
        let reason = reason.into();
        self.setup_pipeline = self.setup_pipeline.clone().block(reason.clone());
        self.setup = self.setup.clone().complete(false, false);
        self.readiness
            .mark_runtime_blocked(runtime_id, reason.clone());
        self.block_running_runtime_install_jobs(&reason);
        self.persist_setup_state();
        self.persist_setup_pipeline();
        self.persist_readiness_state();
        self.persist_runtime_jobs();
    }

    fn block_running_runtime_install_jobs(&mut self, reason: &str) {
        for job in self.jobs.list_jobs() {
            if job.kind() == "runtime.install" && job.state() == JobState::Running {
                let _ = self.jobs.block_with_message(job.id(), reason);
            }
        }
    }
}

fn normalized_workspace_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(std::path::MAIN_SEPARATOR);
    if trimmed.is_empty() {
        path.to_string()
    } else {
        trimmed.to_string()
    }
}
