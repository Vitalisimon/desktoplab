use std::collections::BTreeMap;

use desktoplab_storage::{ProductizationRecordKind, SqliteStore, StorageError};
use serde_json::{Value, json};

use super::{LocalApiRouter, WorkspaceRecord};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeBinding {
    session_id: String,
    workspace_id: String,
    base_root: String,
    worktree_root: String,
    base_head: String,
}

impl WorktreeBinding {
    pub(crate) fn new(
        session_id: impl Into<String>,
        workspace_id: impl Into<String>,
        base_root: impl Into<String>,
        worktree_root: impl Into<String>,
        base_head: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            workspace_id: workspace_id.into(),
            base_root: base_root.into(),
            worktree_root: worktree_root.into(),
            base_head: base_head.into(),
        }
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub(crate) fn base_root(&self) -> &str {
        &self.base_root
    }

    pub(crate) fn worktree_root(&self) -> &str {
        &self.worktree_root
    }

    pub(crate) fn base_head(&self) -> &str {
        &self.base_head
    }

    fn to_json(&self) -> Value {
        json!({
            "sessionId":self.session_id,
            "workspaceId":self.workspace_id,
            "baseRoot":self.base_root,
            "worktreeRoot":self.worktree_root,
            "baseHead":self.base_head
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        Some(Self::new(
            value.get("sessionId")?.as_str()?,
            value.get("workspaceId")?.as_str()?,
            value.get("baseRoot")?.as_str()?,
            value.get("worktreeRoot")?.as_str()?,
            value.get("baseHead").and_then(Value::as_str).unwrap_or(""),
        ))
    }
}

pub(crate) fn bindings_payload(bindings: &BTreeMap<String, WorktreeBinding>) -> Value {
    json!({"bindings":bindings.values().map(WorktreeBinding::to_json).collect::<Vec<_>>()})
}

pub(crate) fn load_worktree_bindings(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, WorktreeBinding>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::WorktreeSession, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("bindings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(WorktreeBinding::from_json)
        .map(|binding| (binding.session_id().to_string(), binding))
        .collect())
}

impl LocalApiRouter {
    pub(crate) fn execution_workspace_record(&self, session_id: &str) -> Option<WorkspaceRecord> {
        let workspace_id = self.sessions.workspace_id_for(session_id);
        let mut workspace = workspace_id
            .as_deref()
            .and_then(|workspace_id| self.workspaces.get(workspace_id))
            .cloned()
            .or_else(|| {
                self.workspace
                    .as_ref()
                    .filter(|workspace| {
                        Some(workspace.workspace_id.as_str()) == workspace_id.as_deref()
                    })
                    .cloned()
            })?;
        if let Some(binding) = self.worktree_bindings.get(session_id)
            && binding.workspace_id() == workspace.workspace_id
            && binding.base_root() == workspace.root_path
        {
            workspace.root_path = binding.worktree_root().to_string();
        }
        Some(workspace)
    }

    pub(crate) fn worktree_inventory(&self, workspace_id: &str) -> Vec<Value> {
        self.worktree_bindings
            .values()
            .filter(|binding| binding.workspace_id() == workspace_id)
            .map(|binding| {
                json!({
                    "worktreeId":binding.session_id(),
                    "sessionId":binding.session_id(),
                    "path":binding.worktree_root(),
                    "state":if std::path::Path::new(binding.worktree_root()).is_dir() {
                        "active"
                    } else {
                        "missing"
                    }
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{LocalApiRouter, WorkspaceRecord};

    #[test]
    fn missing_session_workspace_never_resolves_to_an_existing_process_path() {
        let mut router = LocalApiRouter::default();
        let session = router
            .sessions
            .create_session("workspace.missing", "backend.ollama");

        let resolved = router.execution_workspace_record(session.session_id());

        assert!(resolved.is_none());
    }

    #[test]
    fn execution_workspace_comes_from_the_session_not_current_selection() {
        let mut router = LocalApiRouter::default();
        let workspace_a = WorkspaceRecord {
            workspace_id: "workspace.a".to_string(),
            display_name: "A".to_string(),
            root_path: "/tmp/workspace-a".to_string(),
        };
        let workspace_b = WorkspaceRecord {
            workspace_id: "workspace.b".to_string(),
            display_name: "B".to_string(),
            root_path: "/tmp/workspace-b".to_string(),
        };
        router
            .workspaces
            .insert(workspace_a.workspace_id.clone(), workspace_a.clone());
        router.workspace = Some(workspace_b);
        let session = router
            .sessions
            .create_session("workspace.a", "backend.ollama");

        let resolved = router.execution_workspace_record(session.session_id());

        assert_eq!(
            resolved.as_ref().unwrap().workspace_id,
            workspace_a.workspace_id
        );
        assert_eq!(resolved.as_ref().unwrap().root_path, workspace_a.root_path);
    }
}
