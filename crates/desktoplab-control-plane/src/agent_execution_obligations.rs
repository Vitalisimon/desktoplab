use desktoplab_agent_engine::{IterativeLoopState, IterativeToolCall, ToolObservation};

pub(crate) fn validate_tool_prerequisites(
    state: &IterativeLoopState,
    call: &IterativeToolCall,
) -> Result<(), &'static str> {
    if call.name() != "desktoplab.patch_file" {
        return Ok(());
    }
    let target = call
        .arguments()
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or("patch_requires_prior_read")?;
    let after_latest_change = state
        .observations()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, observation)| {
            is_content_change(observation) && observation.provenance().target() == Some(target)
        })
        .map_or(0, |(index, _)| index + 1);
    state
        .observations()
        .iter()
        .skip(after_latest_change)
        .any(|observation| {
            observation.tool_name() == "desktoplab.read_file"
                && observation.error().is_none()
                && observation.provenance().target() == Some(target)
        })
        .then_some(())
        .ok_or("patch_requires_prior_read")
}

pub(crate) fn validate_file_change_completion(
    state: &IterativeLoopState,
    outcome: &str,
    evidence: &[&ToolObservation],
) -> Result<(), &'static str> {
    if outcome != "changed" {
        return Ok(());
    }
    let Some((change_index, change)) = state
        .observations()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, observation)| is_content_change(observation))
    else {
        return Ok(());
    };
    let verified = state
        .observations()
        .iter()
        .skip(change_index)
        .any(|observation| {
            observation.error().is_none()
                && cited(observation, evidence)
                && verifies_change(change, observation)
        });
    if verified {
        Ok(())
    } else {
        Err("completion_post_change_inspection_required")
    }
}

fn is_content_change(observation: &ToolObservation) -> bool {
    matches!(
        observation.tool_name(),
        "desktoplab.write_file" | "desktoplab.patch_file"
    ) && observation.error().is_none()
        && observation
            .output()
            .get("changed")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
}

fn verifies_change(change: &ToolObservation, observation: &ToolObservation) -> bool {
    if observation.call_id() == change.call_id() {
        return change.tool_name() == "desktoplab.patch_file"
            && change
                .output()
                .get("diff")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|diff| !diff.trim().is_empty());
    }
    let target = change.provenance().target();
    if observation.tool_name() == "desktoplab.read_file" {
        return target.is_some() && observation.provenance().target() == target;
    }
    change.tool_name() == "desktoplab.patch_file"
        && observation.tool_name() == "desktoplab.git_diff"
        && (observation.provenance().target().is_none()
            || observation.provenance().target() == target)
}

fn cited(observation: &ToolObservation, evidence: &[&ToolObservation]) -> bool {
    evidence
        .iter()
        .any(|entry| entry.call_id() == observation.call_id())
}
