use std::collections::{BTreeMap, BTreeSet};

use desktoplab_agent_engine::IterativeLoopState;
use desktoplab_storage::{ProductizationRecordKind, SqliteStore, StorageError};
use serde_json::Value;

use super::agent_execution_binding::AgentExecutionBinding;

pub(crate) type IterativeAgentJournal = (
    BTreeMap<String, IterativeLoopState>,
    BTreeMap<String, String>,
    BTreeMap<String, usize>,
    BTreeSet<String>,
    BTreeMap<String, AgentExecutionBinding>,
);

pub(crate) fn load_agent_iterative_journal(
    storage: &SqliteStore,
) -> Result<IterativeAgentJournal, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::AgentSession, "iterative")?
    else {
        return Ok((
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeMap::new(),
            BTreeSet::new(),
            BTreeMap::new(),
        ));
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    let states = value
        .get("states")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|states| states.iter())
        .map(|(session_id, state)| {
            IterativeLoopState::from_json(&state.to_string())
                .map(|state| (session_id.clone(), state))
                .map_err(|error| StorageError::Sqlite(error.to_string()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let prompts = string_map(value.get("prompts"));
    let event_offsets = value
        .get("eventOffsets")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|offsets| offsets.iter())
        .filter_map(|(session_id, offset)| {
            offset
                .as_u64()
                .and_then(|offset| usize::try_from(offset).ok())
                .map(|offset| (session_id.clone(), offset))
        })
        .collect();
    let streaming_sessions = value
        .get("streamingSessionIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect();
    let execution_bindings = value
        .get("executionBindings")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|bindings| bindings.iter())
        .filter_map(|(session_id, binding)| {
            AgentExecutionBinding::from_json(binding).map(|binding| (session_id.clone(), binding))
        })
        .collect();
    Ok((
        states,
        prompts,
        event_offsets,
        streaming_sessions,
        execution_bindings,
    ))
}

fn string_map(value: Option<&Value>) -> BTreeMap<String, String> {
    value
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|values| values.iter())
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
        .collect()
}
