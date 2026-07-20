use desktoplab_backend_services::{ApprovalRequestRecord, ApprovalState};
use desktoplab_domain::ApprovalMode;
use desktoplab_storage::{ProductizationRecordKind, SettingValue, SqliteStore, StorageError};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use crate::provider_accounts::ProviderAccountRecord;
use crate::{BackendReadinessState, setup_pipeline::SetupPipeline, setup_state::SetupState};

use super::WorkspaceRecord;
use super::agent_compaction::AgentContextCompaction;
use super::agent_memory::WorkspaceMemoryRecord;
use super::agent_pending::PendingAgentAction;
use super::helpers::string_field;

pub(crate) fn load_current_workspace(
    storage: &SqliteStore,
) -> Result<Option<WorkspaceRecord>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::CurrentWorkspace, "current")?
    else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(workspace_record_from_value(&value))
}

pub(crate) fn load_backend_event_outbox(
    storage: &SqliteStore,
) -> Result<desktoplab_backend_services::BackendEventStreamService, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::BackendEventOutbox, "local")?
    else {
        return Ok(desktoplab_backend_services::BackendEventStreamService::default());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    desktoplab_backend_services::BackendEventStreamService::from_json(&value)
        .map_err(StorageError::Sqlite)
}

pub(crate) fn load_agent_context_compactions(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, AgentContextCompaction>, StorageError> {
    let Some(record) = storage
        .get_productization_state(ProductizationRecordKind::AgentContextCompaction, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("compactions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some((
                entry.get("sessionId")?.as_str()?.to_string(),
                AgentContextCompaction::from_json(entry.get("compaction")?)?,
            ))
        })
        .collect())
}

pub(crate) fn repaired_workspace_record(
    workspace_id: String,
    display_name: String,
    root_path: String,
) -> WorkspaceRecord {
    let normalized_root_path = normalized_workspace_path(&root_path);
    let derived_name = std::path::Path::new(&normalized_root_path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("workspace")
        .to_string();
    let display_name = if display_name.trim().is_empty() {
        derived_name
    } else {
        display_name
    };
    let workspace_id = if workspace_id == "workspace." || workspace_id.trim().is_empty() {
        format!("workspace.{display_name}")
    } else {
        workspace_id
    };
    WorkspaceRecord {
        workspace_id,
        display_name,
        root_path: normalized_root_path,
    }
}

fn workspace_record_from_value(value: &Value) -> Option<WorkspaceRecord> {
    let root_path = value.get("rootPath")?.as_str()?.trim();
    if root_path.is_empty() {
        return None;
    }
    Some(repaired_workspace_record(
        string_field(value, "workspaceId", ""),
        string_field(value, "displayName", ""),
        root_path.to_string(),
    ))
}

pub(crate) fn load_workspace_registry(
    storage: &SqliteStore,
) -> Result<
    (
        BTreeMap<String, WorkspaceRecord>,
        BTreeSet<String>,
        BTreeSet<String>,
    ),
    StorageError,
> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::WorkspaceRegistry, "local")?
    else {
        return Ok((BTreeMap::new(), BTreeSet::new(), BTreeSet::new()));
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    let mut workspaces = BTreeMap::new();
    for workspace in value
        .get("workspaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(record) = workspace_record_from_value(workspace) else {
            continue;
        };
        workspaces.insert(record.workspace_id.clone(), record);
    }
    Ok((
        workspaces,
        string_set_field(&value, "archivedWorkspaceIds"),
        string_set_field(&value, "archivedSessionIds"),
    ))
}

fn string_set_field(value: &Value, field: &str) -> BTreeSet<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn normalized_workspace_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(std::path::MAIN_SEPARATOR);
    if trimmed.is_empty() {
        path.to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn load_provider_accounts(
    storage: &SqliteStore,
) -> Result<std::collections::BTreeMap<String, ProviderAccountRecord>, StorageError> {
    let mut accounts = std::collections::BTreeMap::new();
    for provider_id in ["provider.openai"] {
        let Some(record) = storage
            .get_productization_state(ProductizationRecordKind::ProviderAccount, provider_id)?
        else {
            continue;
        };
        let value: Value = serde_json::from_str(record.payload())
            .map_err(|error| StorageError::Sqlite(error.to_string()))?;
        let account = ProviderAccountRecord::from_json(&value);
        accounts.insert(provider_id.to_string(), account);
    }
    Ok(accounts)
}

pub(crate) fn load_high_end_runtime(
    storage: &SqliteStore,
) -> Result<Option<desktoplab_runtime::HighEndRuntimeLifecycle>, StorageError> {
    let Some(record) = storage.get_setting("runtime.high_end.config")? else {
        return Ok(None);
    };
    let SettingValue::String(payload) = record.value() else {
        return Ok(None);
    };
    let value: Value =
        serde_json::from_str(payload).map_err(|error| StorageError::Sqlite(error.to_string()))?;
    let Some(runtime_id) = value.get("runtimeId").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(endpoint_url) = value.get("endpoint").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(model_id) = value.get("modelId").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(contract) = desktoplab_runtime::high_end_runtime_contracts()
        .into_iter()
        .find(|contract| contract.runtime_id().as_str() == runtime_id)
    else {
        return Ok(None);
    };
    let Ok(endpoint) = desktoplab_runtime::RuntimeEndpointSpec::local(endpoint_url, model_id)
    else {
        return Ok(None);
    };
    use desktoplab_runtime::RuntimeEndpointHealthProbe;
    let evidence = desktoplab_runtime::HttpRuntimeEndpointProbe::default().probe(&endpoint);
    Ok(Some(desktoplab_runtime::HighEndRuntimeLifecycle::attached(
        contract, endpoint, evidence,
    )))
}

pub(crate) fn load_approval_records(
    storage: &SqliteStore,
) -> Result<Vec<ApprovalRequestRecord>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::ApprovalRecord, "local")?
    else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("approvals")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(approval_record_from_json)
        .collect())
}

fn approval_record_from_json(value: &Value) -> Option<ApprovalRequestRecord> {
    Some(ApprovalRequestRecord::restored(
        value.get("approvalId")?.as_str()?,
        value.get("sessionId")?.as_str()?,
        value.get("action")?.as_str()?,
        value.get("operationId")?.as_str()?,
        value
            .get("payloadHash")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        value
            .get("consumed")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        ApprovalState::from_stable_str(
            value
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or("pending"),
        ),
    ))
}

pub(crate) fn load_agent_pending_actions(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, PendingAgentAction>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::AgentPendingAction, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("actions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(PendingAgentAction::from_json)
        .map(|action| (action.approval_id().to_string(), action))
        .collect())
}

pub(crate) fn load_agent_active_sessions(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, String>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::AgentActiveSession, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("sessions")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|object| object.iter())
        .filter_map(|(workspace_id, session_id)| {
            session_id
                .as_str()
                .map(|session_id| (workspace_id.to_string(), session_id.to_string()))
        })
        .collect())
}

pub(crate) fn load_workspace_memories(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, Vec<WorkspaceMemoryRecord>>, StorageError> {
    let mut memories = BTreeMap::new();
    for workspace_id in workspace_ids_with_memory_candidates(storage)? {
        if let Some(entries) = load_workspace_memory_entries(storage, &workspace_id)? {
            memories.insert(workspace_id, entries);
        }
    }
    Ok(memories)
}

fn workspace_ids_with_memory_candidates(
    storage: &SqliteStore,
) -> Result<BTreeSet<String>, StorageError> {
    let mut workspace_ids = BTreeSet::new();
    if let Some(current) =
        storage.get_productization_state(ProductizationRecordKind::CurrentWorkspace, "current")?
    {
        let value: Value = serde_json::from_str(current.payload())
            .map_err(|error| StorageError::Sqlite(error.to_string()))?;
        if let Some(workspace) = workspace_record_from_value(&value) {
            workspace_ids.insert(workspace.workspace_id);
        }
    }
    if let Some(registry) =
        storage.get_productization_state(ProductizationRecordKind::WorkspaceRegistry, "local")?
    {
        let value: Value = serde_json::from_str(registry.payload())
            .map_err(|error| StorageError::Sqlite(error.to_string()))?;
        for workspace in value
            .get("workspaces")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(workspace) = workspace_record_from_value(workspace) {
                workspace_ids.insert(workspace.workspace_id);
            }
        }
    }
    Ok(workspace_ids)
}

fn load_workspace_memory_entries(
    storage: &SqliteStore,
    workspace_id: &str,
) -> Result<Option<Vec<WorkspaceMemoryRecord>>, StorageError> {
    let Some(record) = storage
        .get_productization_state(ProductizationRecordKind::WorkspaceMemory, workspace_id)?
    else {
        return Ok(None);
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(Some(
        value
            .get("memories")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(WorkspaceMemoryRecord::from_json)
            .collect(),
    ))
}

pub(crate) fn load_setup_state(storage: &SqliteStore) -> Result<SetupState, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::SetupState, "local")?
    else {
        return Ok(SetupState::default());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(SetupState::from_json(&value))
}

pub(crate) fn load_setup_pipeline(storage: &SqliteStore) -> Result<SetupPipeline, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::SetupPipeline, "local")?
    else {
        return Ok(SetupPipeline::default());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(SetupPipeline::from_json(&value))
}

pub(crate) fn load_backend_readiness_state(
    storage: &SqliteStore,
) -> Result<BackendReadinessState, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::BackendReadiness, "local")?
    else {
        return Ok(BackendReadinessState::default());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(BackendReadinessState::from_json(&value))
}

pub(crate) fn load_default_approval_mode(
    storage: &SqliteStore,
) -> Result<ApprovalMode, StorageError> {
    let Some(record) = storage.get_setting("approval.default_mode")? else {
        return Ok(ApprovalMode::default());
    };
    let SettingValue::String(value) = record.value() else {
        return Ok(ApprovalMode::default());
    };
    Ok(ApprovalMode::from_stable_str(value).unwrap_or_default())
}

pub(crate) fn load_selected_route_id(storage: &SqliteStore) -> Result<String, StorageError> {
    let Some(record) = storage.get_setting("routing.selected_route_id")? else {
        return Ok(crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID.to_string());
    };
    let SettingValue::String(value) = record.value() else {
        return Ok(crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID.to_string());
    };
    if value.trim().is_empty() {
        Ok(crate::execution_routes::UNCONFIGURED_LOCAL_ROUTE_ID.to_string())
    } else {
        Ok(value.clone())
    }
}
