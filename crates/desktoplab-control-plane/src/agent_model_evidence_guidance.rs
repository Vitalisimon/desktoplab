use desktoplab_agent_engine::{IterativeLoopState, ToolObservation};
use serde_json::Value;

pub(crate) fn evidence_state_guidance(state: &IterativeLoopState) -> String {
    if has_unverified_test_repair(state) {
        return " Required next action: return exactly one desktoplab.run_tests call after the latest mutation to prove the repair. Do not call desktoplab.complete until that rerun passes."
            .to_string();
    }
    let observations = state.observations();
    let Some((change_index, change)) = observations
        .iter()
        .enumerate()
        .rev()
        .find(|(_, observation)| is_content_change(observation))
    else {
        return String::new();
    };
    if let Some(test) = observations
        .iter()
        .skip(change_index + 1)
        .find(|observation| is_passing_test(observation))
    {
        return completion_state_guidance("verified", change, test);
    }
    let inspection = observations
        .iter()
        .skip(change_index)
        .find(|observation| verifies_content_change(change, observation));
    let Some(inspection) = inspection else {
        let action = change.provenance().target().map_or_else(
            || "return exactly one desktoplab.git_diff call".to_string(),
            |target| {
                format!(
                    "return exactly one desktoplab.read_file call with path {}",
                    Value::String(target.to_string())
                )
            },
        );
        return format!(
            " Required next action: {action}. Do not call desktoplab.complete and do not repeat {} until this separate inspection succeeds.",
            change.tool_name()
        );
    };
    completion_state_guidance("changed", change, inspection)
}

pub(crate) fn is_successful_change(observation: &ToolObservation) -> bool {
    match observation.tool_name() {
        "desktoplab.write_file"
        | "desktoplab.patch_file"
        | "desktoplab.create_directory"
        | "desktoplab.move_path"
        | "desktoplab.delete_path" => {
            observation.output().get("changed").and_then(Value::as_bool) == Some(true)
        }
        "desktoplab.commit_changes" => {
            observation.output().get("status").and_then(Value::as_str) == Some("committed")
        }
        "desktoplab.push_changes" => {
            observation.output().get("status").and_then(Value::as_str) == Some("pushed")
        }
        _ => false,
    }
}

pub(crate) fn is_passing_test(observation: &ToolObservation) -> bool {
    observation.is_passing_test_evidence()
}

pub(crate) fn has_unverified_test_repair(state: &IterativeLoopState) -> bool {
    let observations = state.observations();
    let Some(last_failed_test) = observations.iter().rposition(|observation| {
        observation.tool_name() == "desktoplab.run_tests" && observation.error().is_some()
    }) else {
        return false;
    };
    let Some(last_change) = observations.iter().rposition(is_successful_change) else {
        return false;
    };
    let validation_boundary = last_failed_test.max(last_change);

    !observations
        .iter()
        .skip(validation_boundary + 1)
        .any(is_passing_test)
}

fn completion_state_guidance(
    outcome: &str,
    change: &ToolObservation,
    proof: &ToolObservation,
) -> String {
    let mut ids = vec![Value::String(change.call_id().to_string())];
    if proof.call_id() != change.call_id() {
        ids.push(Value::String(proof.call_id().to_string()));
    }
    let forbidden = if outcome == "verified" {
        "answered, executed, or changed"
    } else {
        "answered or executed"
    };
    format!(
        " Current completion classification: if the current goal is complete, desktoplab.complete must use outcome {outcome} with evidenceCallIds {}, not {forbidden}.",
        Value::Array(ids)
    )
}

fn is_content_change(observation: &ToolObservation) -> bool {
    matches!(
        observation.tool_name(),
        "desktoplab.write_file" | "desktoplab.patch_file"
    ) && observation.error().is_none()
        && observation.output().get("changed").and_then(Value::as_bool) == Some(true)
}

fn verifies_content_change(change: &ToolObservation, observation: &ToolObservation) -> bool {
    if observation.error().is_some() {
        return false;
    }
    if observation.call_id() == change.call_id() {
        return change.tool_name() == "desktoplab.patch_file"
            && change
                .output()
                .get("diff")
                .and_then(Value::as_str)
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

#[cfg(test)]
mod tests {
    #[test]
    fn evidence_guidance_module_stays_focused() {
        xtask::check_logical_line_limit(
            "crates/desktoplab-control-plane/src/agent_model_evidence_guidance.rs",
            include_str!("agent_model_evidence_guidance.rs"),
            180,
        )
        .expect("agent evidence guidance should stay focused");
    }
}
