use desktoplab_agent_engine::ToolObservation;
use serde_json::Value;

pub(super) fn readable_observation(observation: &ToolObservation) -> String {
    let output = observation.output();
    match observation.tool_name() {
        "desktoplab.list_files" => format!(
            "Workspace files:\n{}",
            string_rows(output, "entries", |entry| {
                string_field(entry, "path").to_string()
            })
        ),
        "desktoplab.read_file" => format!(
            "Read {}:\n{}",
            string_field(output, "path"),
            string_field(output, "text")
        ),
        "desktoplab.search_text" => format!(
            "Search results:\n{}",
            string_rows(output, "matches", |entry| {
                format!(
                    "{}:{}: {}",
                    string_field(entry, "path"),
                    entry.get("lineNumber").and_then(Value::as_u64).unwrap_or(0),
                    string_field(entry, "preview")
                )
            })
        ),
        "desktoplab.patch_file" => output
            .get("diff")
            .and_then(Value::as_str)
            .map(|diff| format!("Git diff:\n{diff}"))
            .unwrap_or_else(|| changed_path(output)),
        "desktoplab.write_file"
        | "desktoplab.create_directory"
        | "desktoplab.move_path"
        | "desktoplab.delete_path" => changed_path(output),
        "desktoplab.run_tests" => command_observation("Test command", true, observation),
        "desktoplab.run_terminal" => command_observation("Command", false, observation),
        "desktoplab.git_status" => format!(
            "Git status:\n{}",
            output
                .get("entries")
                .map(Value::to_string)
                .unwrap_or_else(|| "[]".to_string())
        ),
        "desktoplab.git_diff" => format!(
            "Git diff:\n{}",
            output.get("diff").and_then(Value::as_str).unwrap_or("")
        ),
        "desktoplab.create_checkpoint" => format!(
            "Checkpoint ready: {}",
            output
                .get("ref")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        ),
        "desktoplab.commit_changes" => format!(
            "Git commit created: {}",
            output
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("commit")
        ),
        "desktoplab.push_changes" => format!(
            "Git push completed: {} {}",
            output
                .get("remote")
                .and_then(Value::as_str)
                .unwrap_or("remote"),
            output
                .get("branch")
                .and_then(Value::as_str)
                .unwrap_or("branch")
        ),
        tool => format!("Observation: tool {tool} returned {}", output),
    }
}

fn command_observation(label: &str, tests: bool, observation: &ToolObservation) -> String {
    let output = observation.output();
    let command = output
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("requested command");
    let status = output
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let exit_code = output
        .get("exitCode")
        .and_then(Value::as_i64)
        .map_or_else(|| "none".to_string(), |code| code.to_string());
    let outcome = command_outcome(tests, status, exit_code.as_str());
    let stdout = string_field(output, "stdout");
    let stderr = string_field(output, "stderr");
    let detail = match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => "No output was produced.".to_string(),
        (false, true) => format!("Output:\n{stdout}"),
        (true, false) => format!("Errors:\n{stderr}"),
        (false, false) => format!("Output:\n{stdout}\nErrors:\n{stderr}"),
    };
    format!("{label} `{command}` {outcome}\n{detail}")
}

fn command_outcome(tests: bool, status: &str, exit_code: &str) -> String {
    match (status, exit_code) {
        ("exited", "0") if tests => "passed.".to_string(),
        ("exited", "0") => "completed successfully.".to_string(),
        ("exited", code) if tests => format!("failed (exit code {code})."),
        ("exited", code) => format!("finished with exit code {code}."),
        ("timed_out", _) => "timed out.".to_string(),
        ("failed_to_spawn", _) => "could not start.".to_string(),
        _ => "finished without a readable status.".to_string(),
    }
}

fn changed_path(output: &Value) -> String {
    let path = output
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or("workspace");
    let changed = output
        .get("changed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    format!("Changed {path}: {changed}")
}

fn string_rows(output: &Value, key: &str, row: impl Fn(&Value) -> String) -> String {
    output
        .get(key)
        .and_then(Value::as_array)
        .map(|values| values.iter().map(row).collect::<Vec<_>>().join("\n"))
        .unwrap_or_default()
}

fn string_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

#[cfg(test)]
#[path = "agent_observation_display_tests.rs"]
mod tests;
