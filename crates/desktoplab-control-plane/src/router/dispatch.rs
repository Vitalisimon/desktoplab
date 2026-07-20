use serde_json::json;

use super::helpers::{body_field_or, path_without_query, segment};
use super::{ApiRouteResponse, LocalApiRouter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentContinuationMode {
    Immediate,
    Deferred,
}

impl LocalApiRouter {
    pub fn route(&mut self, method: &str, path: &str, body: &str) -> Option<ApiRouteResponse> {
        self.route_with_agent_continuation(method, path, body, AgentContinuationMode::Immediate)
    }

    pub(crate) fn route_deferred(
        &mut self,
        method: &str,
        path: &str,
        body: &str,
    ) -> Option<ApiRouteResponse> {
        self.route_with_agent_continuation(method, path, body, AgentContinuationMode::Deferred)
    }

    fn route_with_agent_continuation(
        &mut self,
        method: &str,
        path: &str,
        body: &str,
        agent_continuation: AgentContinuationMode,
    ) -> Option<ApiRouteResponse> {
        if !path.starts_with("/v1/") {
            return None;
        }
        let journal_diagnostics = path_without_query(path).starts_with("/v1/diagnostics");
        if !journal_diagnostics && let Some(error) = self.state_journal_failure() {
            return Some(ApiRouteResponse::state_journal_failed(error));
        }
        let event_sequence_before = self.events.latest_sequence();

        let response = match (method, path_without_query(path)) {
            #[cfg(debug_assertions)]
            ("POST", "/v1/test/reset") => self.reset_for_test_control(),
            #[cfg(debug_assertions)]
            ("POST", "/v1/test/agent-backend") => self.agent_backend_for_test_control(body),
            #[cfg(debug_assertions)]
            ("POST", "/v1/test/model-protocol") => self.model_protocol_for_test_control(body),
            ("GET", "/v1/setup/preview") => {
                ApiRouteResponse::ok(crate::setup_routes::setup_preview_response())
            }
            ("GET", "/v1/app/state") => self.app_state(),
            ("GET", "/v1/approval-modes") => self.approval_modes(),
            ("POST", "/v1/approval-modes/default") => self.update_default_approval_mode(body),
            ("POST", "/v1/approval-modes/session") => self.update_session_approval_mode(body),
            ("POST", "/v1/setup/accept") => self.accept_setup_plan(body),
            ("POST", "/v1/setup/complete") => self.complete_setup(body),
            ("GET", "/v1/setup/catalog-refresh") => {
                ApiRouteResponse::ok(crate::setup_routes::catalog_refresh_status_response())
            }
            ("POST", "/v1/setup/catalog-refresh") => self.refresh_catalog(),
            ("GET", "/v1/providers") => {
                let connected_account =
                    self.provider_accounts
                        .get("provider.openai")
                        .filter(|account| {
                            account
                                .vault_ref()
                                .is_some_and(|vault_ref| self.codex_credential_available(vault_ref))
                        });
                ApiRouteResponse::ok(crate::provider_routes::providers_response(
                    connected_account,
                ))
            }
            ("POST", "/v1/provider-bridges/openai-codex/pairing/start") => {
                self.start_openai_codex_bridge_pairing(body)
            }
            ("POST", "/v1/provider-bridges/openai-codex/pairing/poll") => {
                self.poll_openai_codex_bridge_pairing(body)
            }
            ("POST", "/v1/provider-bridges/openai-codex/pairing/complete") => {
                self.complete_openai_codex_bridge_pairing(body)
            }
            ("POST", "/v1/provider-bridges/openai-codex/certify") => {
                match crate::provider_bridge_routes::certify_openai_codex_bridge(body) {
                    Ok(response) => ApiRouteResponse::ok(response),
                    Err(error) => ApiRouteResponse::bad_request(error),
                }
            }
            ("POST", provider_connect_path)
                if provider_connect_path.starts_with("/v1/providers/")
                    && provider_connect_path.ends_with("/connect") =>
            {
                self.connect_provider(provider_connect_path, body)
            }
            ("POST", provider_test_path)
                if provider_test_path.starts_with("/v1/providers/")
                    && provider_test_path.ends_with("/test") =>
            {
                self.test_provider(provider_test_path, body)
            }
            ("POST", provider_disconnect_path)
                if provider_disconnect_path.starts_with("/v1/providers/")
                    && provider_disconnect_path.ends_with("/disconnect") =>
            {
                self.disconnect_provider(provider_disconnect_path, body)
            }
            ("GET", provider_diagnostics_path)
                if provider_diagnostics_path.starts_with("/v1/providers/")
                    && provider_diagnostics_path.ends_with("/diagnostics") =>
            {
                self.provider_diagnostics(provider_diagnostics_path)
            }
            ("GET", "/v1/audit/local") => self.local_audit(),
            ("GET", "/v1/routing/preference") => self.route_preference(path, body),
            ("POST", "/v1/routing/preference") => self.route_preference(path, body),
            ("GET", "/v1/routing/options") => self.route_options(),
            ("POST", "/v1/routing/options/selection") => self.update_route_selection(body),
            ("GET", "/v1/runtime/inspect") => self.runtime_inspect(),
            ("GET", "/v1/runtimes/high-end/inspect") => self.high_end_runtime_inspect(),
            ("POST", "/v1/runtimes/high-end/discover") => self.high_end_runtime_discover(body),
            ("POST", "/v1/runtimes/high-end/attach") => self.high_end_runtime_attach(body),
            ("POST", "/v1/runtimes/high-end/stop") => self.high_end_runtime_stop(),
            ("GET", "/v1/runtimes") => ApiRouteResponse::ok(
                crate::runtime_routes::runtimes_response(self.owns_managed_ollama_runtime()),
            ),
            ("POST", runtime_install_path)
                if runtime_install_path.starts_with("/v1/runtimes/")
                    && runtime_install_path.ends_with("/install") =>
            {
                self.runtime_install(runtime_install_path, body)
            }
            ("POST", runtime_verify_path)
                if runtime_verify_path.starts_with("/v1/runtimes/")
                    && runtime_verify_path.ends_with("/verify") =>
            {
                self.runtime_verify(runtime_verify_path, body)
            }
            ("GET", "/v1/models") => self.models_inventory(),
            ("POST", model_cancel_path)
                if model_cancel_path.starts_with("/v1/models/")
                    && model_cancel_path.ends_with("/download/cancel") =>
            {
                self.model_download_cancel(model_cancel_path, body)
            }
            ("POST", model_resume_path)
                if model_resume_path.starts_with("/v1/models/")
                    && model_resume_path.ends_with("/download/resume") =>
            {
                self.model_download_resume(model_resume_path, body)
            }
            ("POST", model_download_path)
                if model_download_path.starts_with("/v1/models/")
                    && model_download_path.ends_with("/download") =>
            {
                self.model_download(model_download_path, body)
            }
            ("POST", model_verify_path)
                if model_verify_path.starts_with("/v1/models/")
                    && model_verify_path.ends_with("/verify") =>
            {
                self.model_verify(model_verify_path, body)
            }
            ("GET", "/v1/agent/workspace") => self.agent_workspace(),
            ("POST", "/v1/agent/subagents") => self.spawn_subagent(body),
            (subagent_method, subagent_path)
                if subagent_path.starts_with("/v1/agent/subagents/") =>
            {
                self.subagent_route(subagent_method, subagent_path, body)
            }
            ("GET", "/v1/git/operations") => self.git_operations(path),
            ("POST", "/v1/agent/worktrees") => self.create_agent_worktree(body),
            ("POST", rollback_preview_path)
                if rollback_preview_path.starts_with("/v1/git/savepoints/")
                    && rollback_preview_path.ends_with("/rollback/preview") =>
            {
                self.git_rollback_preview(rollback_preview_path)
            }
            ("POST", rollback_path)
                if rollback_path.starts_with("/v1/git/savepoints/")
                    && rollback_path.ends_with("/rollback") =>
            {
                self.git_rollback(rollback_path, body)
            }
            ("POST", "/v1/git/commit") => self.git_commit(body),
            ("POST", "/v1/git/push") => self.approved_git_response(
                body,
                "git.push",
                "git.push".to_string(),
                json!({"status":"blocked","reason":"remote push requires explicit user approval"}),
            ),
            ("POST", worktree_path)
                if worktree_path.starts_with("/v1/git/worktrees/")
                    && worktree_path.ends_with("/cleanup") =>
            {
                self.cleanup_agent_worktree(worktree_path)
            }
            ("GET", workspace_path) if workspace_path.ends_with("/intelligence") => {
                self.workspace_intelligence()
            }
            ("POST", workspace_path) if workspace_path.ends_with("/intelligence/refresh") => {
                ApiRouteResponse::ok(json!({
                    "source":"service_backed",
                    "status":"blocked",
                    "reason":"workspace_scan_refresh_not_available"
                }))
            }
            ("GET", workspace_path) if workspace_path.ends_with("/memory") => {
                self.workspace_memory(workspace_path)
            }
            ("POST", workspace_path) if workspace_path.ends_with("/memory") => {
                self.remember_workspace_memory(workspace_path, body)
            }
            ("POST", memory_path)
                if memory_path.starts_with("/v1/workspaces/memory/")
                    && memory_path.ends_with("/delete") =>
            {
                self.delete_workspace_memory(memory_path)
            }
            ("GET", workspace_path) if workspace_path.ends_with("/context-preview") => {
                self.context_preview()
            }
            ("GET", "/v1/plugins") => self.plugins_list(),
            ("GET", "/v1/mcp/tools") => self.mcp_tools(),
            ("POST", "/v1/mcp/servers/import") => self.import_mcp_server(body),
            ("POST", mcp_path)
                if mcp_path.starts_with("/v1/mcp/servers/")
                    && mcp_path.ends_with("/disconnect") =>
            {
                self.disconnect_mcp_server(mcp_path)
            }
            ("POST", plugin_path)
                if plugin_path.starts_with("/v1/plugins/") && plugin_path.ends_with("/trust") =>
            {
                self.plugin_trust_response(plugin_path, body)
            }
            ("GET", "/v1/external-backends") => {
                ApiRouteResponse::ok(crate::execution_routes::external_backends_response())
            }
            ("GET", "/v1/external-backends/bridge-contract/v2") => {
                ApiRouteResponse::ok(crate::execution_routes::external_agent_bridge_v2_contract())
            }
            ("POST", backend_path)
                if backend_path.starts_with("/v1/external-backends/routes/")
                    && backend_path.ends_with("/resolve") =>
            {
                ApiRouteResponse::ok(json!({
                    "source":"service_backed",
                    "status":"blocked",
                    "requestedResolution":body_field_or(body, "resolution", "denied"),
                    "reason":"external_route_not_connected"
                }))
            }
            ("GET", "/v1/diagnostics") => self.diagnostics_snapshot(),
            ("GET", "/v1/diagnostics/export") => self.diagnostics_export(),
            ("GET", "/v1/diagnostics/doctor/lint") => self.doctor_lint(),
            ("GET", "/v1/diagnostics/migrations") => self.migration_status(),
            ("GET", "/v1/security/audit") => self.security_audit(),
            ("POST", repair_path)
                if repair_path.starts_with("/v1/diagnostics/repairs/")
                    && repair_path.ends_with("/run") =>
            {
                self.run_diagnostic_repair(&segment(repair_path, 3), body)
            }
            ("POST", "/v1/workspaces/open") => self.open_workspace(body),
            ("GET", "/v1/workspaces") => self.workspaces_list(),
            ("POST", workspace_relink_path)
                if workspace_relink_path.starts_with("/v1/workspaces/")
                    && workspace_relink_path.ends_with("/relink") =>
            {
                self.relink_workspace(workspace_relink_path, body)
            }
            ("POST", workspace_archive_path)
                if workspace_archive_path.starts_with("/v1/workspaces/")
                    && workspace_archive_path.ends_with("/archive") =>
            {
                self.archive_workspace(workspace_archive_path)
            }
            ("POST", terminal_path)
                if crate::terminal_routes::is_terminal_command_path(terminal_path) =>
            {
                self.terminal_command(terminal_path, body)
            }
            ("GET", workspace_context_attachments_path)
                if workspace_context_attachments_path.starts_with("/v1/workspaces/")
                    && workspace_context_attachments_path.ends_with("/context-attachments") =>
            {
                self.context_attachments(workspace_context_attachments_path)
            }
            ("GET", workspace_files_path)
                if workspace_files_path.starts_with("/v1/workspaces/")
                    && workspace_files_path.ends_with("/files") =>
            {
                self.workspace_files(workspace_files_path)
            }
            ("GET", workspace_preview_path)
                if workspace_preview_path.starts_with("/v1/workspaces/")
                    && workspace_preview_path.ends_with("/files/preview") =>
            {
                self.workspace_file_preview(workspace_preview_path, path)
            }
            ("GET", "/v1/jobs") => self.jobs_list(),
            ("POST", job_path)
                if job_path.starts_with("/v1/jobs/") && job_path.ends_with("/retry") =>
            {
                self.retry_job(job_path)
            }
            ("GET", "/v1/events/replay") => self.events_replay(path),
            ("GET", "/v1/sessions") => self.sessions_list(path),
            ("POST", "/v1/sessions") => self.create_session(body, agent_continuation),
            ("POST", session_messages_path)
                if session_messages_path.starts_with("/v1/sessions/")
                    && session_messages_path.ends_with("/messages") =>
            {
                self.continue_session(session_messages_path, body, agent_continuation)
            }
            ("POST", session_archive_path)
                if session_archive_path.starts_with("/v1/sessions/")
                    && session_archive_path.ends_with("/archive") =>
            {
                self.archive_session(session_archive_path)
            }
            ("POST", session_path)
                if session_path.starts_with("/v1/sessions/")
                    && session_path.ends_with("/control") =>
            {
                self.session_control(session_path, body)
            }
            ("GET", "/v1/approvals") => self.list_approvals(),
            ("POST", "/v1/approvals") => self.create_approval(body),
            ("POST", approval_path)
                if approval_path.starts_with("/v1/approvals/")
                    && approval_path.ends_with("/resolve") =>
            {
                self.resolve_approval(approval_path, body, agent_continuation)
            }
            _ => ApiRouteResponse::not_found(),
        };

        if self.events.latest_sequence() != event_sequence_before {
            self.persist_event_outbox();
        }

        if !journal_diagnostics && let Some(error) = self.state_journal_failure() {
            return Some(ApiRouteResponse::state_journal_failed(error));
        }
        Some(response)
    }
}
