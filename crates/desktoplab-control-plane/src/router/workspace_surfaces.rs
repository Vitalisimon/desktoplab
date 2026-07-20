use desktoplab_backend_services::{ApprovalState, BackendEventScope};
use desktoplab_workspace::{
    CommitApproval, CommitOperation, ParallelAgentRouter, ProductWorktreeManager, RollbackApproval,
    RollbackOperation, SavePoint, SavePointManager, SessionIntent,
};
use serde_json::{Value, json};

use crate::provider_accounts::ProviderAccountRecord;
use crate::router_payloads as payloads;

use super::git_fingerprint::git_change_fingerprint;
use super::helpers::{
    body_field, body_field_or, body_string_array, git_commit_payload_hash, query_value, segment,
    string_field, terminal_command_payload_hash, terminal_operation_id, workspace_json,
};
use super::{ApiRouteResponse, LocalApiRouter, WorkspaceRecord};

impl LocalApiRouter {
    pub(crate) fn workspaces_list(&self) -> ApiRouteResponse {
        ApiRouteResponse::ok(json!({"workspaces":self.visible_workspace_values()}))
    }

    pub(crate) fn relink_workspace(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let workspace_id = segment(path, 2);
        if !self.workspaces.contains_key(&workspace_id) {
            return ApiRouteResponse::not_found();
        }
        let root_path = normalize_workspace_root(&body_field_or(body, "path", ""));
        let root = std::path::Path::new(&root_path);
        if !root.exists() || !root.is_dir() {
            return ApiRouteResponse::bad_request(json!({
                "code":"WORKSPACE_PATH_NOT_FOUND",
                "message":"Choose an existing local folder.",
                "blockedReason":"workspace_path_not_found"
            }));
        }
        if desktoplab_workspace::GitRepository::open(root).is_err() {
            return ApiRouteResponse::bad_request(json!({
                "code":"GIT_REPOSITORY_REQUIRED",
                "message":"Choose a local Git repository.",
                "blockedReason":"git_repository_required"
            }));
        }
        let display_name = root
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .filter(|name| !name.is_empty())
            .unwrap_or("workspace")
            .to_string();
        let workspace = WorkspaceRecord {
            workspace_id: workspace_id.clone(),
            display_name,
            root_path,
        };
        self.archived_workspace_ids.remove(&workspace_id);
        self.workspaces
            .insert(workspace_id.clone(), workspace.clone());
        self.workspace = Some(workspace);
        self.persist_current_workspace();
        self.persist_workspace_registry();
        ApiRouteResponse::ok(workspace_json(
            self.workspace.as_ref().expect("workspace was relinked"),
        ))
    }

    pub(crate) fn archive_workspace(&mut self, path: &str) -> ApiRouteResponse {
        let workspace_id = segment(path, 2);
        if !self.workspaces.contains_key(&workspace_id) {
            return ApiRouteResponse::not_found();
        }
        self.archived_workspace_ids.insert(workspace_id.clone());
        if self
            .workspace
            .as_ref()
            .is_some_and(|workspace| workspace.workspace_id == workspace_id)
        {
            self.workspace = self
                .workspaces
                .values()
                .find(|workspace| {
                    !self
                        .archived_workspace_ids
                        .contains(&workspace.workspace_id)
                })
                .cloned();
            self.persist_current_workspace();
        }
        self.persist_workspace_registry();
        ApiRouteResponse::ok(json!({"archived":true,"workspaceId":workspace_id}))
    }

    pub(crate) fn archive_session(&mut self, path: &str) -> ApiRouteResponse {
        let session_id = segment(path, 2);
        if self.sessions.get(&session_id).is_none() {
            return ApiRouteResponse::not_found();
        }
        self.archived_session_ids.insert(session_id.clone());
        self.agent_active_session_by_workspace
            .retain(|_, active_session_id| active_session_id != &session_id);
        self.persist_agent_active_sessions();
        self.persist_workspace_registry();
        ApiRouteResponse::ok(json!({"archived":true,"sessionId":session_id}))
    }

    pub(crate) fn workspace_files(&self, path: &str) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        if segment(path, 2) != workspace.workspace_id {
            return ApiRouteResponse::not_found();
        }
        crate::workspace_files::file_tree_json(&workspace.workspace_id, &workspace.root_path)
            .map(ApiRouteResponse::ok)
            .unwrap_or_else(|_| ApiRouteResponse::not_found())
    }

    pub(crate) fn workspace_file_preview(
        &self,
        route_path: &str,
        request_path: &str,
    ) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        if segment(route_path, 2) != workspace.workspace_id {
            return ApiRouteResponse::not_found();
        }
        let Some(relative_path) = crate::workspace_files::preview_query_path(request_path) else {
            return ApiRouteResponse::not_found();
        };
        crate::workspace_files::file_preview_json(
            &workspace.workspace_id,
            &workspace.root_path,
            &relative_path,
        )
        .map(ApiRouteResponse::ok)
        .unwrap_or_else(|_| ApiRouteResponse::not_found())
    }

    pub(crate) fn context_attachments(&self, route_path: &str) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        if segment(route_path, 2) != workspace.workspace_id {
            return ApiRouteResponse::not_found();
        }
        crate::workspace_files::context_attachments_json(
            &workspace.workspace_id,
            &workspace.root_path,
        )
        .map(ApiRouteResponse::ok)
        .unwrap_or_else(|error| {
            ApiRouteResponse::bad_request(
                json!({"code":"CONTEXT_ATTACHMENTS_UNAVAILABLE","message":error.to_string()}),
            )
        })
    }

    pub(crate) fn terminal_command(&mut self, route_path: &str, body: &str) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        let approval_required = terminal_approval_required(body);
        let approval_id = body_field(body, "approvalId");
        let session_id = body_field_or(body, "sessionId", "session.local");
        let operation_id = terminal_operation_id(&workspace.workspace_id);
        let payload_hash = terminal_command_payload_hash(body);
        let approval_state = approval_id
            .as_deref()
            .and_then(|approval_id| {
                self.approvals.get(approval_id).filter(|record| {
                    record.session_id() == session_id
                        && record.action() == "terminal.command"
                        && record.operation_id() == operation_id
                        && record.payload_hash() == payload_hash.as_deref()
                })
            })
            .map(|record| record.state());
        if approval_required && approval_state == Some(ApprovalState::Denied) {
            self.events.publish_json(
                BackendEventScope::Terminal,
                json!({
                    "terminalId":"terminal.local",
                    "kind":"terminal.denied",
                    "workspaceId":workspace.workspace_id,
                    "command":body_field_or(body, "command", ""),
                    "cwd":body_field_or(body, "cwd", "."),
                    "approvalId":approval_id.clone().unwrap_or_default(),
                    "approvalState":"denied",
                    "copy":"Terminal command denied."
                }),
            );
            return ApiRouteResponse::ok(json!({
                "terminalId":"terminal.local",
                "workspaceId":workspace.workspace_id,
                "state":"denied",
                "command":body_field_or(body, "command", ""),
                "cwd":body_field_or(body, "cwd", "."),
                "approval":{"approvalId":approval_id.unwrap_or_default(),"state":"denied","copy":"Terminal command denied."}
            }));
        }
        let approved_record = approval_id.as_deref().is_some_and(|approval_id| {
            self.approvals.consume_approved_for_payload(
                approval_id,
                &session_id,
                "terminal.command",
                &operation_id,
                payload_hash.as_deref(),
            )
        });
        let approval_authorized = !approval_required || approved_record;
        let pending_approval_id = if approval_required && !approval_authorized {
            self.approvals
                .request_operation_with_payload_hash(
                    &session_id,
                    "terminal.command",
                    operation_id,
                    payload_hash,
                )
                .id()
                .to_string()
        } else {
            String::new()
        };
        crate::terminal_routes::terminal_command_route_response(
            route_path,
            &workspace.workspace_id,
            &workspace.root_path,
            body,
            approval_authorized,
            &pending_approval_id,
        )
        .map(|response| {
            self.publish_terminal_response(&response);
            ApiRouteResponse::ok(response)
        })
        .unwrap_or_else(ApiRouteResponse::not_found)
    }

    pub(crate) fn publish_terminal_response(&mut self, response: &Value) {
        let terminal_id = string_field(response, "terminalId", "terminal.local");
        let workspace_id = string_field(response, "workspaceId", "");
        let command = string_field(response, "command", "");
        match response
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "approval_required" => self.events.publish_json(
                BackendEventScope::Terminal,
                json!({
                    "terminalId":terminal_id,
                    "kind":"terminal.approval_required",
                    "workspaceId":workspace_id,
                    "command":command,
                    "cwd":string_field(response, "cwd", "."),
                    "approvalState":"pending",
                    "approvalId":response
                        .get("approval")
                        .and_then(|approval| approval.get("approvalId"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                }),
            ),
            "completed" => {
                self.events
                    .publish_terminal_started(&terminal_id, &workspace_id, &command);
                if let Some(event) = response
                    .get("events")
                    .and_then(Value::as_array)
                    .and_then(|events| events.first())
                {
                    self.events.publish_terminal_output(
                        &terminal_id,
                        "stdout",
                        event
                            .get("stdout")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        event
                            .get("stdoutTruncated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    );
                    self.events.publish_terminal_completed(
                        &terminal_id,
                        event
                            .get("exitCode")
                            .and_then(Value::as_i64)
                            .map(|code| code as i32),
                    );
                }
            }
            _ => {}
        }
    }

    pub(crate) fn connect_provider(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let response = crate::provider_routes::connect_provider_response(
            path,
            body,
            self.openai_codex_native_vault_for_test.as_mut(),
        );
        if let Some(account) = ProviderAccountRecord::from_connection_response(&response) {
            let provider_id = account.provider_id().to_string();
            self.provider_accounts.insert(provider_id.clone(), account);
            self.persist_provider_account(&provider_id);
        }
        ApiRouteResponse::ok(response)
    }

    pub(crate) fn start_openai_codex_bridge_pairing(&mut self, body: &str) -> ApiRouteResponse {
        let device_authorization = self
            .openai_codex_device_authorization_for_test
            .as_ref()
            .and_then(|fixture| {
                desktoplab_backends::OpenAiCodexDeviceAuthorization::new(
                    &fixture.device_auth_id,
                    &fixture.user_code,
                    1,
                )
                .ok()
            });
        match crate::provider_bridge_routes::start_openai_codex_pairing(body, device_authorization)
        {
            Ok((pairing, response)) => {
                self.openai_codex_pairings
                    .insert(pairing.pairing_id().to_string(), pairing);
                ApiRouteResponse::ok(response)
            }
            Err(error) => ApiRouteResponse::bad_request(error),
        }
    }

    pub(crate) fn complete_openai_codex_bridge_pairing(&mut self, body: &str) -> ApiRouteResponse {
        let pairing_id = body_field(body, "pairingId").unwrap_or_default();
        let pairing = self.openai_codex_pairings.get(&pairing_id);
        match crate::provider_bridge_routes::complete_openai_codex_pairing(body, pairing) {
            Ok(response) => {
                let vault_ref = response
                    .get("vaultRef")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !self.codex_credential_available(vault_ref) {
                    return ApiRouteResponse::bad_request(json!({
                        "code":"CODEX_CREDENTIAL_NOT_FOUND",
                        "message":"The declared Codex credential reference is not readable from the native vault."
                    }));
                }
                let responder_url = response
                    .get("bridgeResponderUrl")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if !crate::provider_bridge_routes::codex_responder_reachable(responder_url) {
                    return ApiRouteResponse::bad_request(json!({
                        "code":"CODEX_RESPONDER_UNREACHABLE",
                        "message":"The declared local Codex responder is not reachable."
                    }));
                }
                if let Some(account) = ProviderAccountRecord::from_connection_response(&response) {
                    let provider_id = account.provider_id().to_string();
                    self.provider_accounts.insert(provider_id.clone(), account);
                    self.persist_provider_account(&provider_id);
                }
                self.openai_codex_pairings.remove(&pairing_id);
                ApiRouteResponse::ok(response)
            }
            Err(error) => ApiRouteResponse::bad_request(error),
        }
    }

    pub(crate) fn poll_openai_codex_bridge_pairing(&mut self, body: &str) -> ApiRouteResponse {
        let pairing_id = body_field(body, "pairingId").unwrap_or_default();
        let pairing = self.openai_codex_pairings.get(&pairing_id).cloned();
        let credential_dir = self
            .openai_codex_bridge_dir
            .clone()
            .unwrap_or_else(|| std::env::temp_dir().join("desktoplab-local-provider-bridge"));
        let test_authorization = self
            .openai_codex_device_authorization_for_test
            .as_ref()
            .map(|fixture| {
                (
                    fixture.authorization_code.as_str(),
                    fixture.code_verifier.as_str(),
                )
            });
        match crate::provider_bridge_routes::poll_openai_codex_pairing(
            body,
            pairing.as_ref(),
            &credential_dir,
            test_authorization,
            self.openai_codex_native_vault_for_test.as_mut(),
        ) {
            Ok(response) => {
                if response.get("status").and_then(Value::as_str) == Some("connected") {
                    if let Some(account) =
                        ProviderAccountRecord::from_connection_response(&response)
                    {
                        let provider_id = account.provider_id().to_string();
                        self.provider_accounts.insert(provider_id.clone(), account);
                        self.persist_provider_account(&provider_id);
                    }
                    self.openai_codex_pairings.remove(&pairing_id);
                }
                ApiRouteResponse::ok(response)
            }
            Err(error) => ApiRouteResponse::bad_request(error),
        }
    }

    pub(crate) fn test_provider(&self, path: &str, body: &str) -> ApiRouteResponse {
        ApiRouteResponse::ok(crate::provider_routes::test_provider_response(
            path,
            body,
            self.openai_codex_native_vault_for_test.as_ref(),
        ))
    }

    pub(crate) fn disconnect_provider(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let provider_id = segment(path, 2);
        let persisted_vault_ref = self
            .provider_accounts
            .get(&provider_id)
            .and_then(ProviderAccountRecord::vault_ref)
            .map(str::to_string);
        let response = crate::provider_routes::disconnect_provider_response(
            path,
            body,
            persisted_vault_ref.as_deref(),
            self.openai_codex_native_vault_for_test.as_mut(),
        );
        if response.get("status").and_then(Value::as_str) == Some("removed") {
            let account_mode = body_field_or(body, "accountMode", "api_key_billing");
            self.provider_accounts.insert(
                provider_id.clone(),
                ProviderAccountRecord::removed(&provider_id, account_mode),
            );
            self.persist_provider_account(&provider_id);
        }
        ApiRouteResponse::ok(response)
    }

    pub(crate) fn provider_diagnostics(&self, path: &str) -> ApiRouteResponse {
        let provider_id = segment(path, 2);
        let account = self.provider_accounts.get(&provider_id).filter(|account| {
            account
                .vault_ref()
                .is_some_and(|vault_ref| self.codex_credential_available(vault_ref))
        });
        ApiRouteResponse::ok(crate::provider_routes::provider_diagnostics_response(
            path, account,
        ))
    }

    pub(crate) fn git_operations(&self, path: &str) -> ApiRouteResponse {
        let workspace = if let Some(workspace_id) = query_value(path, "workspace_id") {
            let Some(workspace) = self.workspaces.get(&workspace_id) else {
                return ApiRouteResponse::not_found();
            };
            workspace.clone()
        } else {
            let Some(workspace) = self.workspace_record() else {
                return ApiRouteResponse::not_found();
            };
            workspace
        };
        let Ok(repo) =
            desktoplab_workspace::GitRepository::open(std::path::Path::new(&workspace.root_path))
        else {
            return ApiRouteResponse::ok(payloads::git_operations(workspace.workspace_id));
        };
        let status = repo.status().ok();
        let dirty = status.as_ref().is_some_and(|status| status.is_dirty());
        let status_entries = status
            .as_ref()
            .map(|status| status.entries())
            .unwrap_or(&[]);
        let changed_files = status
            .as_ref()
            .map(|status| {
                status
                    .files()
                    .iter()
                    .map(|file| file.path().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let diff_text = repo
            .diff()
            .map(|diff| diff.as_text().to_string())
            .unwrap_or_default();
        let change_fingerprint = git_change_fingerprint(status_entries, &diff_text);
        let save_points = SavePointManager::default()
            .list(std::path::Path::new(&workspace.root_path))
            .map(savepoint_summaries)
            .unwrap_or_default();
        ApiRouteResponse::ok(json!({
            "workspaceId":workspace.workspace_id,
            "workspaceState":if dirty { "dirty" } else { "clean" },
            "warnings":if dirty { vec!["Dirty worktree"] } else { Vec::<&str>::new() },
            "changedFiles":changed_files,
            "statusEntries":status_entries,
            "diffPreview":bounded_diff_preview(&diff_text),
            "changeFingerprint":change_fingerprint,
            "savePoints":save_points,
            "commit":{
                "supported":dirty,
                "sessionId":"session.1",
                "message":"agent change",
                "changeFingerprint":change_fingerprint,
                "preview":if dirty { "Commit requires approval." } else { "No completed change to commit." },
                "requiresApproval":true
            },
            "push":{
                "supported":false,
                "remote":"origin",
                "branch":"main",
                "preview":"Push requires approval and a committed change.",
                "requiresApproval":true,
                "normalizedReason":"no_commit"
            },
            "worktrees":self.worktree_inventory(&workspace.workspace_id)
        }))
    }

    pub(crate) fn git_commit(&mut self, body: &str) -> ApiRouteResponse {
        let payload_hash = git_commit_payload_hash(body);
        let approved = match self.consume_body_approved_record(
            body,
            &body_field_or(body, "sessionId", "session.local"),
            "git.commit",
            "git.commit",
            Some(&payload_hash),
        ) {
            Ok(approved) => approved,
            Err(error) => return ApiRouteResponse::state_journal_failed(error),
        };
        if !approved {
            return ApiRouteResponse::ok(json!({"status":"blocked","reason":"approval_required"}));
        }
        let expected_changed_files = body_string_array(body, "changedFiles");
        if expected_changed_files.is_empty() {
            return ApiRouteResponse::ok(
                json!({"status":"blocked","reason":"missing_reviewed_file_set"}),
            );
        }
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        let session_id = body_field_or(body, "sessionId", "session.local");
        let message = body_field_or(body, "message", "agent change");
        let expected_fingerprint = body_field_or(body, "changeFingerprint", "");
        let root = std::path::Path::new(&workspace.root_path);
        let Ok(repo) = desktoplab_workspace::GitRepository::open(root) else {
            return ApiRouteResponse::ok(
                json!({"status":"blocked","reason":"git_repository_required"}),
            );
        };
        let status = repo.status().ok();
        let status_entries = status
            .as_ref()
            .map(|status| status.entries().to_vec())
            .unwrap_or_default();
        let diff_text = repo
            .diff()
            .map(|diff| diff.as_text().to_string())
            .unwrap_or_default();
        let current_fingerprint = git_change_fingerprint(&status_entries, &diff_text);
        if expected_fingerprint.is_empty() || expected_fingerprint != current_fingerprint {
            return ApiRouteResponse::ok(json!({
                "status":"blocked",
                "reason":"working_tree_changed_after_approval",
                "currentFingerprint":current_fingerprint
            }));
        }
        let changed_files = status
            .as_ref()
            .map(|status| {
                status
                    .files()
                    .iter()
                    .map(|file| file.path().to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if normalized_file_set(&expected_changed_files) != normalized_file_set(&changed_files) {
            return ApiRouteResponse::ok(json!({
                "status":"blocked",
                "reason":"working_tree_changed_after_approval",
                "changedFiles":changed_files
            }));
        }
        match CommitOperation::new(CommitApproval::Approved).commit(
            root,
            &session_id,
            &message,
            &changed_files,
        ) {
            Ok(outcome) if outcome.status() == "committed" => {
                let commit_hash = git_head(root).unwrap_or_default();
                ApiRouteResponse::ok(json!({"status":"committed","commitHash":commit_hash.trim()}))
            }
            Ok(outcome) => ApiRouteResponse::ok(json!({"status":outcome.status()})),
            Err(error) => {
                ApiRouteResponse::ok(json!({"status":"blocked","reason":error.to_string()}))
            }
        }
    }

    pub(crate) fn create_agent_worktree(&mut self, body: &str) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        let root = std::path::Path::new(&workspace.root_path);
        let session_id = body_field_or(body, "sessionId", "session.local");
        let intent = match body_field_or(body, "intent", "write_capable").as_str() {
            "read_only" => SessionIntent::ReadOnly,
            _ => SessionIntent::WriteCapable,
        };
        if intent == SessionIntent::ReadOnly {
            return ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "workspaceId":workspace.workspace_id,
                "sessionId":session_id,
                "worktreeId":Value::Null,
                "status":"ready",
                "isolationReason":"read_only_can_share_workspace",
                "worktreePath":Value::Null,
                "mergePolicy":{"requiresExplicitApproval":true,"reconciliation":"review_diff_before_merge"}
            }));
        }
        if self.sessions.get(&session_id).is_none() {
            return ApiRouteResponse::bad_request(json!({
                "code":"AGENT_SESSION_NOT_FOUND",
                "message":"Create the owning agent session before requesting an isolated worktree."
            }));
        }
        let base_head = git_head(root).unwrap_or_default().trim().to_string();
        if let Some(binding) = self.worktree_bindings.get(&session_id) {
            return ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "workspaceId":binding.workspace_id(),
                "sessionId":session_id,
                "worktreeId":session_id,
                "status":if std::path::Path::new(binding.worktree_root()).is_dir() { "ready" } else { "blocked" },
                "isolationReason":if std::path::Path::new(binding.worktree_root()).is_dir() { "write_capable_parallel_requires_worktree" } else { "managed_worktree_missing" },
                "worktreePath":binding.worktree_root(),
                "mergePolicy":{"requiresExplicitApproval":true,"reconciliation":"review_diff_before_merge"}
            }));
        }
        let route = ParallelAgentRouter::default().route(root, &session_id, intent);
        if let Some(worktree_path) = route.worktree_path() {
            self.worktree_bindings.insert(
                session_id.clone(),
                super::worktree_bindings::WorktreeBinding::new(
                    &session_id,
                    &workspace.workspace_id,
                    &workspace.root_path,
                    worktree_path.display().to_string(),
                    base_head,
                ),
            );
            self.persist_worktree_bindings();
            if let Some(error) = self.state_journal_failure() {
                return ApiRouteResponse::state_journal_failed(error);
            }
        }
        ApiRouteResponse::ok(json!({
            "source":"service_backed",
            "workspaceId":workspace.workspace_id,
            "sessionId":session_id,
            "worktreeId":session_id,
            "status":if route.worktree_path().is_some() || route.can_share_workspace() { "ready" } else { "blocked" },
            "isolationReason":route.isolation_reason(),
            "worktreePath":route.worktree_path().map(|path| path.display().to_string()),
            "mergePolicy":{
                "requiresExplicitApproval":true,
                "reconciliation":"review_diff_before_merge"
            }
        }))
    }

    pub(crate) fn cleanup_agent_worktree(&mut self, path: &str) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        let root = std::path::Path::new(&workspace.root_path);
        let worktree_id = segment(path, 3);
        let Some(binding) = self.worktree_bindings.get(&worktree_id).cloned() else {
            return ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "status":"blocked",
                "worktreeId":worktree_id,
                "reason":"managed_worktree_not_registered"
            }));
        };
        if binding.workspace_id() != workspace.workspace_id
            || binding.base_root() != workspace.root_path
        {
            return ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "status":"blocked",
                "worktreeId":worktree_id,
                "reason":"managed_worktree_owner_mismatch"
            }));
        }
        if self.sessions.get(&worktree_id).is_some_and(|session| {
            matches!(
                session.state(),
                desktoplab_agent_session::SessionState::Running
                    | desktoplab_agent_session::SessionState::Blocked
                    | desktoplab_agent_session::SessionState::Paused
            )
        }) {
            return ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "status":"blocked",
                "worktreeId":worktree_id,
                "reason":"agent_session_active"
            }));
        }
        match ProductWorktreeManager::default().cleanup(root, &worktree_id) {
            Ok(outcome) => {
                self.worktree_bindings.remove(&worktree_id);
                self.persist_worktree_bindings();
                if let Some(error) = self.state_journal_failure() {
                    return ApiRouteResponse::state_journal_failed(error);
                }
                ApiRouteResponse::ok(json!({
                    "source":"service_backed",
                    "status":outcome.status(),
                    "worktreeId":worktree_id
                }))
            }
            Err(error) => ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "status":"blocked",
                "worktreeId":worktree_id,
                "reason":error.to_string()
            })),
        }
    }

    pub(crate) fn git_rollback_preview(&self, path: &str) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        let root = std::path::Path::new(&workspace.root_path);
        let savepoint = savepoint_from_path(path);
        match RollbackOperation::new(RollbackApproval::Denied).preview(root, &savepoint) {
            Ok(preview) => ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "status":"preview",
                "savePointId":savepoint.ref_name(),
                "changedFiles":preview.changed_files(),
                "protectedUntrackedFiles":preview.protected_untracked_files(),
                "approvalAction":"git.rollback",
                "operationId":savepoint.ref_name(),
                "requiresApproval":true
            })),
            Err(error) => {
                ApiRouteResponse::ok(json!({"status":"blocked","reason":error.to_string()}))
            }
        }
    }

    pub(crate) fn git_rollback(&mut self, path: &str, body: &str) -> ApiRouteResponse {
        let savepoint = savepoint_from_path(path);
        let approved = match self.consume_body_approved_record(
            body,
            &body_field_or(body, "sessionId", "session.local"),
            "git.rollback",
            savepoint.ref_name(),
            None,
        ) {
            Ok(approved) => approved,
            Err(error) => return ApiRouteResponse::state_journal_failed(error),
        };
        if !approved {
            return ApiRouteResponse::ok(json!({"status":"blocked","reason":"approval_required"}));
        }
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        let root = std::path::Path::new(&workspace.root_path);
        match RollbackOperation::new(RollbackApproval::Approved).rollback(root, &savepoint) {
            Ok(outcome) => ApiRouteResponse::ok(json!({
                "source":"service_backed",
                "status":outcome.status(),
                "protectedUntrackedFilesRemain":true
            })),
            Err(error) => {
                ApiRouteResponse::ok(json!({"status":"blocked","reason":error.to_string()}))
            }
        }
    }

    pub(crate) fn workspace_intelligence(&self) -> ApiRouteResponse {
        let Some(workspace) = self.workspace_record() else {
            return ApiRouteResponse::not_found();
        };
        ApiRouteResponse::ok(payloads::workspace_intelligence(
            workspace.workspace_id,
            workspace.display_name,
            workspace.root_path,
        ))
    }

    pub(crate) fn context_preview(&self) -> ApiRouteResponse {
        let Some(workspace_id) = self.workspace_id() else {
            return ApiRouteResponse::not_found();
        };
        ApiRouteResponse::ok(payloads::context_preview(workspace_id))
    }

    pub(crate) fn app_state(&self) -> ApiRouteResponse {
        let current_workspace = self.workspace.as_ref().map(workspace_json);
        ApiRouteResponse::ok(crate::app_state::app_state_json(
            &self.setup,
            &self.setup_pipeline,
            &self.readiness,
            current_workspace,
            self.visible_workspace_values(),
            self.approvals
                .list()
                .iter()
                .filter(|approval| approval.state() == ApprovalState::Pending)
                .count(),
            self.workspace_id()
                .map(|workspace_id| self.sessions.list_by_workspace(&workspace_id).len())
                .unwrap_or_default(),
            self.approval_modes_payload(),
            self.session_approval_mode,
        ))
    }

    fn visible_workspace_values(&self) -> Vec<Value> {
        let mut values = Vec::new();
        if let Some(current) = &self.workspace
            && !self.archived_workspace_ids.contains(&current.workspace_id)
        {
            values.push(workspace_json(current));
        }
        values.extend(
            self.workspaces
                .values()
                .filter(|workspace| {
                    !self
                        .archived_workspace_ids
                        .contains(&workspace.workspace_id)
                        && match &self.workspace {
                            Some(current) => current.workspace_id != workspace.workspace_id,
                            None => true,
                        }
                })
                .map(workspace_json),
        );
        if values.is_empty() {
            if let Some(workspace) = &self.workspace {
                values.push(workspace_json(workspace));
            }
        }
        values
    }
}

fn normalize_workspace_root(path: &str) -> String {
    path.trim()
        .trim_end_matches(std::path::MAIN_SEPARATOR)
        .to_string()
}

fn terminal_approval_required(body: &str) -> bool {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|value| value.get("approvalRequired").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn bounded_diff_preview(diff: &str) -> String {
    const MAX_PREVIEW_CHARS: usize = 32_000;
    let mut chars = diff.chars();
    let preview = chars.by_ref().take(MAX_PREVIEW_CHARS).collect::<String>();
    if chars.next().is_some() {
        format!("{preview}\n... Diff preview truncated.")
    } else {
        preview
    }
}

fn normalized_file_set(files: &[String]) -> Vec<String> {
    let mut files = files.to_vec();
    files.sort();
    files.dedup();
    files
}

fn git_head(root: &std::path::Path) -> Result<String, std::io::Error> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Ok(String::new())
    }
}

fn savepoint_summaries(savepoints: Vec<SavePoint>) -> Vec<Value> {
    savepoints
        .into_iter()
        .map(|savepoint| {
            json!({
                "savePointId":savepoint.ref_name(),
                "title":format!("Before {}", savepoint.session_id()),
                "sessionId":savepoint.session_id(),
                "createdAt":"",
                "rollbackSupported":true,
                "rollbackPreview":"Rollback preview available"
            })
        })
        .collect()
}

fn savepoint_from_path(path: &str) -> SavePoint {
    let savepoint_id = path
        .trim_start_matches("/v1/git/savepoints/")
        .trim_end_matches("/rollback/preview")
        .trim_end_matches("/rollback")
        .trim_matches('/');
    SavePoint::from_ref("session.local", savepoint_id)
}
