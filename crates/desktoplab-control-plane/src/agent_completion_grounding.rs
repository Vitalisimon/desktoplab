use std::collections::BTreeSet;

use desktoplab_agent_engine::ToolObservation;
use serde_json::Value;

const INSPECTION_TOOLS: &[&str] = &[
    "desktoplab.read_file",
    "desktoplab.list_files",
    "desktoplab.search_text",
    "desktoplab.git_status",
    "desktoplab.git_diff",
];

const GENERIC_WORDS: &[&str] = &[
    "changed",
    "completed",
    "diff",
    "executed",
    "file",
    "files",
    "git",
    "inspected",
    "output",
    "path",
    "read",
    "repository",
    "result",
    "status",
    "success",
    "successful",
    "successfully",
    "tool",
    "workspace",
];

const COMPLETE_STATUS_PATH_LIMIT: usize = 12;

pub(crate) fn validate_inspection_message(
    message: &str,
    evidence: &[&ToolObservation],
) -> Result<(), &'static str> {
    let inspected = evidence
        .iter()
        .copied()
        .filter(|observation| INSPECTION_TOOLS.contains(&observation.tool_name()))
        .collect::<Vec<_>>();
    if inspected.is_empty() {
        return Ok(());
    }

    validate_bounded_git_status(message, &inspected)?;

    let mut anchors = BTreeSet::new();
    for observation in inspected {
        collect_value_tokens(observation.output(), &mut anchors);
        if let Some(target) = observation.provenance().target() {
            collect_tokens(target, &mut anchors);
        }
    }
    if anchors.is_empty() {
        return Ok(());
    }

    let mut message_tokens = BTreeSet::new();
    collect_tokens(message, &mut message_tokens);
    if anchors.is_disjoint(&message_tokens) {
        return Err("completion_message_missing_evidence_anchor");
    }
    Ok(())
}

fn validate_bounded_git_status(
    message: &str,
    evidence: &[&ToolObservation],
) -> Result<(), &'static str> {
    let paths = evidence
        .iter()
        .filter(|observation| observation.tool_name() == "desktoplab.git_status")
        .flat_map(|observation| {
            observation
                .output()
                .get("entries")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(Value::as_str)
        .filter_map(status_entry_path)
        .collect::<Vec<_>>();
    if paths.is_empty() || paths.len() > COMPLETE_STATUS_PATH_LIMIT {
        return Ok(());
    }

    let compact_message = compact_identifier(message);
    if paths.iter().all(|path| {
        let compact_path = compact_identifier(path);
        let compact_stem = path.rsplit_once('.').map_or_else(
            || compact_path.clone(),
            |(stem, _)| compact_identifier(stem),
        );
        compact_message.contains(&compact_path) || compact_message.contains(&compact_stem)
    }) {
        Ok(())
    } else {
        Err("completion_message_missing_status_entries")
    }
}

fn status_entry_path(entry: &str) -> Option<&str> {
    let path = entry.get(3..).unwrap_or(entry).trim();
    (!path.is_empty()).then_some(path)
}

fn compact_identifier(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn collect_value_tokens(value: &Value, output: &mut BTreeSet<String>) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_value_tokens(value, output);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                collect_value_tokens(value, output);
            }
        }
        Value::String(value) => collect_tokens(value, output),
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn collect_tokens(value: &str, output: &mut BTreeSet<String>) {
    for token in value
        .split(|character: char| !character.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|token| token.chars().count() >= 4)
        .filter(|token| !GENERIC_WORDS.contains(&token.as_str()))
    {
        output.insert(token);
    }
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::{IterativeToolCall, ToolObservation};
    use serde_json::json;

    use super::validate_inspection_message;

    fn observation(tool: &str, output: serde_json::Value) -> ToolObservation {
        let call = IterativeToolCall::new("call.1", tool, json!({}));
        ToolObservation::success(&call, output)
    }

    #[test]
    fn generic_inspection_completion_is_rejected() {
        let status = observation(
            "desktoplab.git_status",
            json!({"entries":[" M calculator.js","?? release-summary.md"]}),
        );

        assert_eq!(
            validate_inspection_message(
                "Git status and diff have been successfully executed.",
                &[&status]
            ),
            Err("completion_message_missing_status_entries")
        );
    }

    #[test]
    fn inspection_completion_accepts_concrete_evidence() {
        let status = observation(
            "desktoplab.git_status",
            json!({"entries":[" M calculator.js","?? release-summary.md"]}),
        );

        assert_eq!(
            validate_inspection_message(
                "calculator.js is modified and release-summary.md is untracked.",
                &[&status]
            ),
            Ok(())
        );
    }

    #[test]
    fn git_status_completion_rejects_a_stale_partial_file_list() {
        let status = observation(
            "desktoplab.git_status",
            json!({"entries":[
                " M calculator.js",
                " M release-note.md",
                "?? release-summary.md"
            ]}),
        );

        assert_eq!(
            validate_inspection_message(
                "Tracked files: README.md, calculator.js, calculator.test.js, package.json, release-note.md",
                &[&status]
            ),
            Err("completion_message_missing_status_entries")
        );
    }

    #[test]
    fn mutation_completion_does_not_require_inspection_anchors() {
        let write = observation(
            "desktoplab.write_file",
            json!({"path":"notes.md","changed":true}),
        );

        assert_eq!(
            validate_inspection_message("The requested document was created.", &[&write]),
            Ok(())
        );
    }
}
