use serde_json::{Value, json};

use crate::tool_schema_builders::{boolean_output, string_output};
use crate::{AgentToolRisk, AgentToolSchema};

pub(crate) fn process_tools() -> Vec<AgentToolSchema> {
    vec![
        tool(
            "desktoplab.run_terminal",
            "Run an approved terminal command. The optional cwd is workspace-relative; omit it for the workspace root. Absolute paths are rejected.",
            AgentToolRisk::High,
            true,
            command_input(true),
            string_output("stdout"),
        ),
        tool(
            "desktoplab.start_process",
            "Start an approved long-running process and return a session-owned process ID.",
            AgentToolRisk::High,
            true,
            object(&["command", "cwd"], &["command"]),
            string_output("processId"),
        ),
        tool(
            "desktoplab.poll_process",
            "Poll incremental output and state from a process owned by this session.",
            AgentToolRisk::Low,
            false,
            object(&["processId"], &["processId"]),
            string_output("status"),
        ),
        tool(
            "desktoplab.write_process_stdin",
            "Write input to a running process owned by this session.",
            AgentToolRisk::Medium,
            false,
            object(&["processId", "input"], &["processId", "input"]),
            boolean_output("accepted"),
        ),
        tool(
            "desktoplab.kill_process",
            "Terminate a running process owned by this session, including its child process tree.",
            AgentToolRisk::Medium,
            false,
            object(&["processId"], &["processId"]),
            string_output("status"),
        ),
    ]
}

fn tool(
    id: &'static str,
    description: &'static str,
    risk: AgentToolRisk,
    requires_approval: bool,
    input: Value,
    output: Value,
) -> AgentToolSchema {
    AgentToolSchema::new(id, description, risk, requires_approval, input, output)
}

fn object(properties: &[&str], required: &[&str]) -> Value {
    let properties = properties
        .iter()
        .map(|name| {
            let schema = if *name == "cwd" {
                json!({
                    "type":"string",
                    "description":"Workspace-relative working directory. Omit it for the workspace root. Never use an absolute path."
                })
            } else {
                json!({"type":"string"})
            };
            ((*name).to_string(), schema)
        })
        .collect::<serde_json::Map<_, _>>();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn command_input(with_cwd: bool) -> Value {
    let mut properties = serde_json::Map::from_iter([
        ("command".to_string(), json!({"type":"string"})),
        (
            "timeoutSeconds".to_string(),
            json!({"type":"integer","minimum":1,"maximum":1800}),
        ),
    ]);
    if with_cwd {
        properties.insert(
            "cwd".to_string(),
            json!({
                "type":"string",
                "description":"Workspace-relative working directory. Omit it for the workspace root. Never use an absolute path."
            }),
        );
    }
    json!({
        "type":"object",
        "properties":properties,
        "required":["command"],
        "additionalProperties":false
    })
}
