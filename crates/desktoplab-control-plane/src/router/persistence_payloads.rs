use std::collections::{BTreeMap, BTreeSet};

use desktoplab_backend_services::ApprovalService;
use desktoplab_storage::{
    ProductizationRecordKind, ProductizationStateRecord, SqliteStore, StorageError,
};
use serde_json::{Value, json};

use super::WorkspaceRecord;
use super::agent_compaction::AgentContextCompaction;
use super::agent_execution_binding::AgentExecutionBinding;
use super::agent_memory::WorkspaceMemoryRecord;
use super::agent_pending::PendingAgentAction;
use desktoplab_agent_engine::IterativeLoopState;

pub(super) fn persist_state(
    storage: &SqliteStore,
    kind: ProductizationRecordKind,
    key: &str,
    payload: Value,
) -> Result<(), StorageError> {
    storage.put_productization_state(ProductizationStateRecord::new(
        kind,
        key,
        payload.to_string(),
    ))
}

pub(super) fn workspace_registry_payload(
    workspaces: &BTreeMap<String, WorkspaceRecord>,
    archived_workspace_ids: &BTreeSet<String>,
    archived_session_ids: &BTreeSet<String>,
) -> Value {
    let workspaces = workspaces
        .values()
        .map(|workspace| {
            json!({
                "workspaceId":workspace.workspace_id,
                "displayName":workspace.display_name,
                "rootPath":workspace.root_path
            })
        })
        .collect::<Vec<_>>();
    json!({
        "workspaces":workspaces,
        "archivedWorkspaceIds":archived_workspace_ids.iter().collect::<Vec<_>>(),
        "archivedSessionIds":archived_session_ids.iter().collect::<Vec<_>>()
    })
}

pub(super) fn approval_records_payload(approvals: &ApprovalService) -> Value {
    let approvals = approvals
        .list()
        .iter()
        .map(|approval| {
            json!({
                "approvalId":approval.id(),
                "sessionId":approval.session_id(),
                "action":approval.action(),
                "operationId":approval.operation_id(),
                "payloadHash":approval.payload_hash(),
                "consumed":approval.is_consumed(),
                "state":approval.state().as_str()
            })
        })
        .collect::<Vec<_>>();
    json!({"approvals":approvals})
}

pub(super) fn pending_actions_payload(actions: &BTreeMap<String, PendingAgentAction>) -> Value {
    json!({"actions":actions.values().map(|action| action.to_json()).collect::<Vec<_>>()})
}

pub(super) fn iterative_states_payload(
    states: &BTreeMap<String, IterativeLoopState>,
    prompts: &BTreeMap<String, String>,
    event_offsets: &BTreeMap<String, usize>,
    streaming_sessions: &std::collections::BTreeSet<String>,
    execution_bindings: &BTreeMap<String, AgentExecutionBinding>,
) -> Value {
    let states = states
        .iter()
        .filter_map(|(session_id, state)| {
            serde_json::to_value(state)
                .ok()
                .map(|state| (session_id.clone(), state))
        })
        .collect::<serde_json::Map<_, _>>();
    json!({
        "states":states,
        "prompts":prompts,
        "eventOffsets":event_offsets,
        "streamingSessionIds":streaming_sessions,
        "executionBindings":execution_bindings.iter().map(|(session_id, binding)| {
            (session_id.clone(), binding.to_json())
        }).collect::<serde_json::Map<_, _>>()
    })
}

pub(super) fn active_sessions_payload(sessions: &BTreeMap<String, String>) -> Value {
    json!({"sessions":sessions})
}

pub(super) fn context_compactions_payload(
    compactions: &BTreeMap<String, AgentContextCompaction>,
) -> Value {
    json!({
        "compactions":compactions.iter().map(|(session_id, compaction)| {
            json!({"sessionId":session_id,"compaction":compaction.to_json()})
        }).collect::<Vec<_>>()
    })
}

pub(super) fn workspace_memory_payload(memories: &[WorkspaceMemoryRecord]) -> Value {
    json!({
        "memories":memories.iter().map(WorkspaceMemoryRecord::to_json).collect::<Vec<_>>()
    })
}
