use serde_json::{Value, json};

use crate::tool_schema_builders::object;

pub(crate) fn clarification_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "question":{"type":"string"},
            "blockedOn":{"type":"string", "enum":["desktoplab.list_files", "desktoplab.read_file", "desktoplab.search_text", "desktoplab.write_file", "desktoplab.patch_file", "desktoplab.create_directory", "desktoplab.move_path", "desktoplab.delete_path", "desktoplab.run_terminal", "desktoplab.start_process", "desktoplab.poll_process", "desktoplab.write_process_stdin", "desktoplab.kill_process", "desktoplab.run_tests", "desktoplab.git_status", "desktoplab.git_diff", "desktoplab.create_checkpoint", "desktoplab.commit_changes", "desktoplab.push_changes"]}
        },
        "required":["question", "blockedOn"], "additionalProperties":false
    })
}

pub(crate) fn completion_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "message":{"type":"string"},
            "outcome":{
                "type":"string",
                "enum":["answered","executed","changed","verified"],
                "description":"Use answered for read-only findings, including reports about existing Git changes; executed for a successful non-mutation action; changed only when the agent applied a mutation with changed=true; and verified only with passing test evidence."
            },
            "evidenceCallIds":{"type":"array","items":{"type":"string"}}
        },
        "required":["message","outcome","evidenceCallIds"],
        "additionalProperties":false
    })
}

pub(crate) fn delete_path_input() -> Value {
    json!({"type":"object","properties":{"path":workspace_path(),"recursive":{"type":"boolean"}},"required":["path"],"additionalProperties":false})
}

pub(crate) fn read_file_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "path":workspace_path(),
            "offset":{"type":"integer","minimum":0},
            "limit":{"type":"integer","minimum":1,"maximum":2000}
        },
        "required":["path"],
        "additionalProperties":false
    })
}

pub(crate) fn search_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "query":{"type":"string"},
            "path":workspace_path(),
            "regex":{"type":"boolean"},
            "caseSensitive":{"type":"boolean"}
        },
        "required":["query"],
        "additionalProperties":false
    })
}

pub(crate) fn command_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "command":{"type":"string"},
            "timeoutSeconds":{"type":"integer","minimum":1,"maximum":1800}
        },
        "required":["command"],
        "additionalProperties":false
    })
}

pub(crate) fn commit_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "message":{"type":"string"},
            "paths":{"type":"array","items":{"type":"string"},"uniqueItems":true}
        },
        "required":["message"],
        "additionalProperties":false
    })
}

pub(crate) fn patch_file_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "path":workspace_path(),
            "expected":{"type":"string"},
            "replacement":{"type":"string"},
            "replaceAll":{"type":"boolean"}
        },
        "required":["path","expected","replacement"],
        "additionalProperties":false
    })
}

fn workspace_path() -> Value {
    json!({
        "type":"string",
        "description":"Workspace-relative path. Never use an absolute path. Omit an optional path to target the workspace root."
    })
}

pub(crate) fn subagent_spawn_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "prompt":{"type":"string"},
            "intent":{"type":"string","enum":["read_only","write_capable"]}
        },
        "required":["prompt","intent"],
        "additionalProperties":false
    })
}

pub(crate) fn plan_input() -> Value {
    json!({
        "type":"object",
        "properties":{
            "steps":{
                "type":"array", "minItems":1, "maxItems":20,
                "items":{
                    "type":"object",
                    "properties":{
                        "step":{"type":"string"},
                        "status":{"type":"string","enum":["pending","in_progress","completed"]}
                    },
                    "required":["step","status"], "additionalProperties":false
                }
            }
        },
        "required":["steps"], "additionalProperties":false
    })
}

pub(crate) fn subagent_message_input() -> Value {
    json!({
        "type":"object",
        "properties":{"subagentId":{"type":"string"},"prompt":{"type":"string"}},
        "required":["subagentId","prompt"], "additionalProperties":false
    })
}

pub(crate) fn subagent_id_input() -> Value {
    object(&["subagentId"], &["subagentId"])
}
