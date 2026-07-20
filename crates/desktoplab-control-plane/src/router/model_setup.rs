use desktoplab_backend_services::{AuditAction, BackendEventScope, JobRetryClass};
use desktoplab_model_manager::{
    MlxLmModelRuntimeAdapter, ModelRuntimeAdapter, OllamaModelRuntimeAdapter,
};
use desktoplab_runtime::{ProcessCommand, ProcessRunner, RuntimeId, SystemProcessRunner};
use serde_json::{Value, json};

use super::helpers::{body_field, segment};
use super::{ApiRouteResponse, LocalApiRouter, ModelDownloadExecutionMode};

impl LocalApiRouter {
    pub(crate) fn models_inventory(&self) -> ApiRouteResponse {
        let installed_models = self
            .local_model_inventory_for_test
            .clone()
            .unwrap_or_else(|| {
                if !self.readiness.runtime_verified() {
                    return Vec::new();
                }
                OllamaModelRuntimeAdapter::new(SystemProcessRunner)
                    .list(RuntimeId::new("runtime.ollama"))
            });
        let readiness = self.readiness.to_json();
        ApiRouteResponse::ok(crate::model_routes::models_response_with_state(
            self.readiness.runtime_verified(),
            self.readiness.runtime_id(),
            readiness
                .get("modelId")
                .and_then(Value::as_str)
                .filter(|_| self.readiness.model_verified()),
            &installed_models,
            self.host_memory_gb_for_test,
        ))
    }

    pub(crate) fn model_download(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        match crate::model_routes::plan_model_download(path, body, self.host_memory_gb_for_test) {
            Ok(start) => {
                let job = self.jobs.create_job(format!(
                    "model.download:{}:{}:{}",
                    start.model_id(),
                    start.runtime_id(),
                    start.pull_ref()
                ));
                let model_id = segment(path, 2);
                self.audit.record(
                    AuditAction::ModelDownload,
                    format!("model.download requested model_id={model_id} body={body}"),
                );
                if !self.readiness.runtime_verified_for(start.runtime_id()) {
                    let _ = self
                        .jobs
                        .block_with_message(job.id(), "runtime_not_verified");
                    self.persist_model_jobs();
                    return ApiRouteResponse::ok(
                        crate::model_routes::model_download_runtime_blocked_response(
                            path,
                            job.id().as_str(),
                            start.runtime_id(),
                        ),
                    );
                }
                let _ = self.jobs.start(job.id());
                self.events.publish_model_download_progress(
                    job.id().as_str(),
                    &model_id,
                    "running",
                    5,
                    "retryable",
                    "",
                );
                match self.model_download_execution {
                    #[cfg(debug_assertions)]
                    ModelDownloadExecutionMode::PlanOnlyForTest => {
                        self.persist_model_jobs();
                        ApiRouteResponse::ok(crate::model_routes::model_download_response(
                            &start,
                            job.id().as_str(),
                        ))
                    }
                    #[cfg(debug_assertions)]
                    ModelDownloadExecutionMode::CompleteForTest => self
                        .complete_model_download_job(
                            job.id(),
                            &model_id,
                            start.runtime_id(),
                            start.pull_ref(),
                            start.setup_choice(),
                            "test model download completed",
                        ),
                    ModelDownloadExecutionMode::Execute => {
                        let inventory = model_inventory_output(start.runtime_id());
                        let already_installed = inventory.as_ref().is_some_and(|output| {
                            output.succeeded()
                                && crate::model_routes::verify_model_inventory(
                                    start.pull_ref(),
                                    output.stdout(),
                                ) == desktoplab_model_manager::ModelVerification::passed()
                        });
                        if already_installed && start.should_use_existing() {
                            return self.complete_model_download_job(
                                job.id(),
                                &model_id,
                                start.runtime_id(),
                                start.pull_ref(),
                                start.setup_choice(),
                                format!("existing model detected by {}", start.runtime_id()),
                            );
                        }
                        let pull = model_pull_result(start.runtime_id(), start.pull_ref());
                        if pull.state() == "completed" {
                            self.complete_model_download_job(
                                job.id(),
                                &model_id,
                                start.runtime_id(),
                                start.pull_ref(),
                                start.setup_choice(),
                                pull.command_evidence(),
                            )
                        } else {
                            self.block_model_download_job(
                                job.id(),
                                &model_id,
                                start.runtime_id(),
                                "runtime pull failed",
                            )
                        }
                    }
                }
            }
            Err(error) => ApiRouteResponse::ok(
                crate::model_routes::model_download_blocked_response(path, error),
            ),
        }
    }

    fn complete_model_download_job(
        &mut self,
        job_id: &desktoplab_backend_services::JobId,
        model_id: &str,
        runtime_id: &str,
        pull_ref: &str,
        setup_choice: &str,
        evidence: impl Into<String>,
    ) -> ApiRouteResponse {
        let evidence = evidence.into();
        let _ = self.jobs.succeed(job_id);
        self.readiness.mark_model_verified(
            runtime_id.to_string(),
            model_id.to_string(),
            format!("{evidence}; {runtime_id} inventory {pull_ref}"),
        );
        self.refresh_ollama_model_capabilities(runtime_id, pull_ref);
        self.selected_route_id = crate::execution_routes::local_route_id(model_id);
        self.stability.mark_route_decision();
        self.setup_pipeline = self.setup_pipeline.clone().ready();
        self.setup = self
            .setup
            .clone()
            .complete(self.readiness.runtime_verified(), true);
        self.persist_model_jobs();
        self.persist_readiness_state();
        self.persist_selected_route_id();
        self.persist_setup_pipeline();
        self.persist_setup_state();
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "jobId":job_id.as_str(),
            "modelId":model_id,
            "runtimeId":runtime_id,
            "state":"completed",
            "progressPercent":100,
            "retryClass":"none",
            "failureReason":Value::Null,
            "setupChoice":setup_choice,
            "executionEvidence":evidence
        }))
    }

    fn block_model_download_job(
        &mut self,
        job_id: &desktoplab_backend_services::JobId,
        model_id: &str,
        runtime_id: &str,
        reason: &str,
    ) -> ApiRouteResponse {
        let _ = self
            .jobs
            .fail_with_message(job_id, JobRetryClass::Retryable, reason);
        self.readiness.mark_model_blocked(
            runtime_id.to_string(),
            model_id.to_string(),
            reason.to_string(),
        );
        self.setup_pipeline = self.setup_pipeline.clone().block(reason);
        self.setup = self
            .setup
            .clone()
            .complete(self.readiness.runtime_verified(), false);
        self.persist_model_jobs();
        self.persist_readiness_state();
        self.persist_setup_pipeline();
        self.persist_setup_state();
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "jobId":job_id.as_str(),
            "modelId":model_id,
            "runtimeId":runtime_id,
            "state":"blocked",
            "progressPercent":0,
            "retryClass":"retryable",
            "blockedReason":reason
        }))
    }

    pub(crate) fn model_download_cancel(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let model_id = segment(path, 2);
        let job_id = body_field(body, "jobId").unwrap_or_default();
        if job_id.is_empty() {
            return ApiRouteResponse::bad_request(json!({
                "code":"JOB_ID_REQUIRED",
                "message":"jobId is required to cancel a model download."
            }));
        }
        let job_id = desktoplab_backend_services::JobId::new(job_id);
        match self.jobs.cancel(&job_id) {
            Ok(()) => {
                self.events.publish_model_download_progress(
                    job_id.as_str(),
                    &model_id,
                    "cancelled",
                    0,
                    "non_retryable",
                    "cancelled",
                );
                self.persist_model_jobs();
                ApiRouteResponse::ok(json!({
                    "source":"service_backed",
                    "jobId":job_id.as_str(),
                    "modelId":model_id,
                    "state":"cancelled",
                    "setupState":"incomplete"
                }))
            }
            Err(error) => ApiRouteResponse::bad_request(json!({
                "code":"MODEL_DOWNLOAD_CANCEL_FAILED",
                "message":error
            })),
        }
    }

    pub(crate) fn model_download_resume(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let resume_supported = serde_json::from_str::<Value>(body)
            .ok()
            .and_then(|value| value.get("resumeSupported").and_then(Value::as_bool))
            .unwrap_or(true);
        if !resume_supported {
            return ApiRouteResponse::ok(crate::model_routes::model_download_blocked_response(
                path,
                crate::model_routes::ModelRouteError::Download(
                    desktoplab_model_manager::ModelDownloadError::ResumeUnsupported,
                ),
            ));
        }
        let previous_job_id = body_field(body, "jobId").unwrap_or_default();
        match crate::model_routes::plan_model_download(path, body, self.host_memory_gb_for_test) {
            Ok(_start) => {
                if previous_job_id.is_empty() {
                    return ApiRouteResponse::bad_request(json!({
                        "code":"JOB_ID_REQUIRED",
                        "message":"jobId is required to resume a model download."
                    }));
                }
                let previous = desktoplab_backend_services::JobId::new(previous_job_id.clone());
                if self.jobs.get_job(&previous).is_none() {
                    return ApiRouteResponse::bad_request(json!({
                        "code":"MODEL_DOWNLOAD_JOB_MISSING",
                        "message":"The previous model download job is not available for resume."
                    }));
                }
                let response = self.model_download(path.trim_end_matches("/resume"), body);
                ApiRouteResponse::ok(resume_response_body(response.body(), &previous_job_id))
            }
            Err(error) => ApiRouteResponse::ok(
                crate::model_routes::model_download_blocked_response(path, error),
            ),
        }
    }

    pub(crate) fn model_verify(&mut self, path: &str, _body: &str) -> ApiRouteResponse {
        let model_id = segment(path, 2);
        let Some(runtime_id) = crate::model_routes::model_runtime_id(&model_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"UNKNOWN_MODEL",
                "message":"Model is not known to the compatibility catalog."
            }));
        };
        let Some(pull_ref) = crate::model_routes::model_pull_ref(&model_id) else {
            return ApiRouteResponse::bad_request(json!({
                "code":"UNKNOWN_MODEL",
                "message":"Model is not known to the compatibility catalog."
            }));
        };
        let inventory = self
            .local_model_inventory_for_test
            .clone()
            .unwrap_or_else(|| model_inventory_lines(&runtime_id));
        let inventory_output = inventory.join("\n");
        let verified = crate::model_routes::verify_model_inventory(&pull_ref, &inventory_output)
            == desktoplab_model_manager::ModelVerification::passed();
        if verified {
            self.readiness.mark_model_verified(
                runtime_id.clone(),
                model_id.clone(),
                format!("{runtime_id} inventory {pull_ref}"),
            );
            self.refresh_ollama_model_capabilities_fresh(&runtime_id, &pull_ref);
            self.selected_route_id = crate::execution_routes::local_route_id(&model_id);
            self.stability.mark_route_decision();
            self.persist_readiness_state();
            self.persist_selected_route_id();
            self.events.publish_json(
                BackendEventScope::Job,
                json!({
                    "kind":"model.verify","modelId":model_id,"runtimeId":runtime_id,
                    "state":"verified","evidence":format!("{runtime_id} inventory {pull_ref}")
                }),
            );
        } else {
            self.readiness.mark_model_blocked(
                runtime_id.clone(),
                model_id.clone(),
                "model_not_reported_by_runtime",
            );
            self.persist_readiness_state();
            self.events.publish_json(
                BackendEventScope::Job,
                json!({
                    "kind":"model.verify","modelId":model_id,"runtimeId":runtime_id,
                    "state":"blocked","failureReason":"model_not_reported_by_runtime"
                }),
            );
        }
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "modelId":model_id,
            "runtimeId":runtime_id,
            "verificationState":if verified {"verified"} else {"blocked"},
            "blockedReason":if verified {Value::Null} else {json!("model_not_reported_by_runtime")},
            "readinessEvidence":self.readiness.to_json()
        }))
    }

    pub(crate) fn refresh_ollama_model_capabilities(&mut self, runtime_id: &str, pull_ref: &str) {
        self.refresh_ollama_model_capabilities_with_mode(runtime_id, pull_ref, false);
    }

    fn refresh_ollama_model_capabilities_fresh(&mut self, runtime_id: &str, pull_ref: &str) {
        self.refresh_ollama_model_capabilities_with_mode(runtime_id, pull_ref, true);
    }

    fn refresh_ollama_model_capabilities_with_mode(
        &mut self,
        runtime_id: &str,
        pull_ref: &str,
        force_canary: bool,
    ) {
        if runtime_id != "runtime.ollama" {
            return;
        }
        if cfg!(debug_assertions)
            && (self.local_model_inventory_for_test.is_some()
                || self.model_download_execution != super::ModelDownloadExecutionMode::Execute)
        {
            return;
        }
        if let Ok(mut capabilities) = self
            .ollama_model_capabilities
            .resolve("http://127.0.0.1:11434", pull_ref)
        {
            let Some(request_timeout_seconds) =
                crate::model_routes::agent_request_timeout_seconds_for_pull_ref(
                    pull_ref,
                    self.host_memory_gb_for_test.unwrap_or(self.host_memory_gb),
                )
            else {
                return;
            };
            let persisted = self
                .readiness
                .model_capabilities()
                .filter(|current| current.fingerprint() == capabilities.fingerprint())
                .and_then(|current| current.tool_protocol_certification())
                .cloned();
            let certification = if force_canary {
                self.ollama_tool_protocol_canary.certify_fresh(
                    "http://127.0.0.1:11434",
                    &capabilities,
                    request_timeout_seconds,
                )
            } else if let Some(persisted) = persisted {
                persisted
            } else {
                self.ollama_tool_protocol_canary.certify(
                    "http://127.0.0.1:11434",
                    &capabilities,
                    request_timeout_seconds,
                )
            };
            capabilities = capabilities.with_tool_protocol_certification(certification);
            self.readiness.mark_model_capabilities(capabilities);
        }
    }
}

fn model_inventory_lines(runtime_id: &str) -> Vec<String> {
    match runtime_id {
        "runtime.ollama" => OllamaModelRuntimeAdapter::new(SystemProcessRunner)
            .list(RuntimeId::new("runtime.ollama")),
        _ => Vec::new(),
    }
}

fn resume_response_body(body: &str, previous_job_id: &str) -> Value {
    let Ok(mut value) = serde_json::from_str::<Value>(body) else {
        return json!({
            "source":"service_backed",
            "state":"blocked",
            "blockedReason":"resume_response_parse_failed"
        });
    };
    if let Some(object) = value.as_object_mut() {
        object.insert("previousJobId".to_string(), json!(previous_job_id));
        object.insert("resume".to_string(), json!(true));
        object.insert("resumeMode".to_string(), json!("runtime_pull_resume"));
    }
    value
}

fn model_inventory_output(runtime_id: &str) -> Option<desktoplab_runtime::ProcessOutput> {
    match runtime_id {
        "runtime.ollama" => Some(<SystemProcessRunner as ProcessRunner>::run(
            &SystemProcessRunner,
            ProcessCommand::new("ollama").arg("list"),
        )),
        _ => None,
    }
}

fn model_pull_result(
    runtime_id: &str,
    pull_ref: &str,
) -> desktoplab_model_manager::ModelRuntimePullResult {
    match runtime_id {
        "runtime.ollama" => OllamaModelRuntimeAdapter::new(SystemProcessRunner).pull(
            desktoplab_model_manager::RuntimeModelPullRequest::new(
                RuntimeId::new(runtime_id),
                pull_ref,
            ),
        ),
        "runtime.mlx-lm" => MlxLmModelRuntimeAdapter::new(SystemProcessRunner).pull(
            desktoplab_model_manager::RuntimeModelPullRequest::new(
                RuntimeId::new(runtime_id),
                pull_ref,
            ),
        ),
        _ => desktoplab_model_manager::ModelRuntimePullResult::blocked_for_unsupported_runtime(),
    }
}
