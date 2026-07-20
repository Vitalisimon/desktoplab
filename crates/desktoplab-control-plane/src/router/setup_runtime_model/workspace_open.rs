use desktoplab_backend_services::AuditAction;
use desktoplab_domain::WorkspaceId;
use serde_json::json;

use crate::router::helpers::{body_bool_or, body_field, workspace_json};
use crate::router::{ApiRouteResponse, LocalApiRouter, WorkspaceRecord};

impl LocalApiRouter {
    pub(crate) fn open_workspace(&mut self, body: &str) -> ApiRouteResponse {
        if !self.setup.is_ready() || !self.readiness.is_ready() {
            self.audit.record_denied(
                AuditAction::PolicyDecision,
                "workspace.open denied reason=setup_not_ready",
            );
            return ApiRouteResponse::bad_request(json!({
                "code":"SETUP_REQUIRED",
                "message":"Setup must finish before opening a repository.",
                "blockedReason":"setup_not_ready",
                "nextRoute":"setup"
            }));
        }
        let Some(root_path) = body_field(body, "path")
            .filter(|path| !path.trim().is_empty())
            .map(|path| super::normalized_workspace_path(&path))
        else {
            return workspace_path_not_found_response();
        };
        let root = std::path::Path::new(&root_path);
        if !root.exists() || !root.is_dir() {
            return workspace_path_not_found_response();
        }
        let (workspace_id, display_name) =
            super::super::workspace_identity::resolve(root, &self.workspaces);
        if desktoplab_workspace::GitRepository::open(root).is_err() {
            if !body_bool_or(body, "initializeGit", false) {
                return git_required_response();
            }
            if let Err(error) = desktoplab_workspace::WorkspaceApiService::default()
                .create_repository(WorkspaceId::new(workspace_id.clone()), root)
            {
                return ApiRouteResponse::bad_request(json!({
                    "code":"GIT_REPOSITORY_INIT_FAILED",
                    "message":"DesktopLab could not initialize Git in this folder.",
                    "blockedReason":"git_repository_init_failed",
                    "details":error.to_string()
                }));
            }
            self.audit.record(
                AuditAction::PolicyDecision,
                format!("workspace.git_init allowed path={root_path}"),
            );
        }
        let workspace = WorkspaceRecord {
            workspace_id,
            display_name,
            root_path: root_path.clone(),
        };
        self.archived_workspace_ids.remove(&workspace.workspace_id);
        self.workspaces
            .insert(workspace.workspace_id.clone(), workspace.clone());
        self.workspace = Some(workspace);
        self.persist_current_workspace();
        self.persist_workspace_registry();
        self.audit.record(
            AuditAction::PolicyDecision,
            format!("workspace.open allowed path={root_path}"),
        );
        ApiRouteResponse::ok(workspace_json(
            self.workspace.as_ref().expect("workspace was set"),
        ))
    }
}

fn workspace_path_not_found_response() -> ApiRouteResponse {
    ApiRouteResponse::bad_request(json!({
        "code":"WORKSPACE_PATH_NOT_FOUND",
        "message":"Choose an existing local folder.",
        "blockedReason":"workspace_path_not_found"
    }))
}

fn git_required_response() -> ApiRouteResponse {
    ApiRouteResponse::bad_request(json!({
        "code":"GIT_REPOSITORY_REQUIRED",
        "message":"Choose a local Git repository.",
        "blockedReason":"git_repository_required"
    }))
}
