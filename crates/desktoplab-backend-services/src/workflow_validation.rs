use std::collections::{HashMap, HashSet};

use crate::workflow::{WorkflowDefinition, WorkflowError, WorkflowNodeKind};

const MAX_NODES: usize = 128;
const MAX_ID_LEN: usize = 96;

pub(crate) fn validate(definition: &WorkflowDefinition) -> Result<(), WorkflowError> {
    if definition.workflow_id.is_empty() || definition.workflow_id.len() > MAX_ID_LEN {
        return Err(WorkflowError::InvalidGraph("invalid_workflow_id"));
    }
    if definition.nodes.is_empty() || definition.nodes.len() > MAX_NODES {
        return Err(WorkflowError::InvalidGraph("invalid_node_count"));
    }

    let mut indexes = HashMap::new();
    for (index, node) in definition.nodes.iter().enumerate() {
        if node.id.is_empty()
            || node.id.len() > MAX_ID_LEN
            || indexes.insert(&node.id, index).is_some()
        {
            return Err(WorkflowError::InvalidGraph("invalid_or_duplicate_node_id"));
        }
        if node.max_attempts == 0 || node.max_attempts > 8 || node.timeout_ms == 0 {
            return Err(WorkflowError::InvalidGraph("invalid_execution_bounds"));
        }
        if node.mutates && node.idempotency_key.is_none() && node.compensation.is_none() {
            return Err(WorkflowError::InvalidGraph("unsafe_replayable_mutation"));
        }
    }

    for node in &definition.nodes {
        for dependency in &node.dependencies {
            if !indexes.contains_key(dependency) || dependency == &node.id {
                return Err(WorkflowError::InvalidGraph("invalid_dependency"));
            }
        }
        if let Some(condition) = &node.condition_node {
            if !indexes.contains_key(condition) || !node.dependencies.contains(condition) {
                return Err(WorkflowError::InvalidGraph("invalid_condition_dependency"));
            }
        }
        if matches!(node.kind, WorkflowNodeKind::Approval) && node.mutates {
            return Err(WorkflowError::InvalidGraph("approval_node_cannot_mutate"));
        }
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    for node in &definition.nodes {
        visit(&node.id, definition, &mut visiting, &mut visited)?;
    }
    Ok(())
}

fn visit<'a>(
    id: &'a str,
    definition: &'a WorkflowDefinition,
    visiting: &mut HashSet<&'a str>,
    visited: &mut HashSet<&'a str>,
) -> Result<(), WorkflowError> {
    if visited.contains(id) {
        return Ok(());
    }
    if !visiting.insert(id) {
        return Err(WorkflowError::InvalidGraph("dependency_cycle"));
    }
    let node = definition
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("validated node");
    for dependency in &node.dependencies {
        visit(dependency, definition, visiting, visited)?;
    }
    visiting.remove(id);
    visited.insert(id);
    Ok(())
}
