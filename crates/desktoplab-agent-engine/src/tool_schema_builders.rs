use serde_json::{Value, json};

use crate::{AgentToolExecutionOwner, AgentToolRisk, AgentToolSchema, AgentToolScope};

pub(crate) fn tool(
    id: &'static str,
    description: &'static str,
    risk: AgentToolRisk,
    requires_approval: bool,
    input: Value,
    output: Value,
) -> AgentToolSchema {
    AgentToolSchema::new(id, description, risk, requires_approval, input, output)
}

pub(crate) fn router_tool(
    id: &'static str,
    description: &'static str,
    risk: AgentToolRisk,
    input: Value,
    output: Value,
) -> AgentToolSchema {
    tool(id, description, risk, false, input, output).with_execution(
        AgentToolExecutionOwner::RouterControl,
        AgentToolScope::Session,
    )
}

pub(crate) fn loop_tool(
    id: &'static str,
    description: &'static str,
    risk: AgentToolRisk,
    input: Value,
    output: Value,
) -> AgentToolSchema {
    tool(id, description, risk, false, input, output).with_execution(
        AgentToolExecutionOwner::LoopControl,
        AgentToolScope::Session,
    )
}

pub(crate) fn object(properties: &[&str], required: &[&str]) -> Value {
    let properties = properties
        .iter()
        .map(|name| ((*name).to_string(), string_property(name)))
        .collect::<serde_json::Map<_, _>>();
    json!({"type":"object","properties":properties,"required":required,"additionalProperties":false})
}

fn string_property(name: &str) -> Value {
    match name {
        "path" => json!({
            "type":"string",
            "description":"Workspace-relative path. Never use an absolute path. When this argument is optional, omit it to target the workspace root."
        }),
        "source" | "destination" => json!({
            "type":"string",
            "description":"Workspace-relative path. Never use an absolute path."
        }),
        _ => json!({"type":"string"}),
    }
}

fn output(field: &str, field_type: &str) -> Value {
    json!({
        "type":"object",
        "properties":{(field):{"type":field_type}},
        "required":[field],
        "additionalProperties":true
    })
}

pub(crate) fn array_output(field: &str) -> Value {
    output(field, "array")
}

pub(crate) fn boolean_output(field: &str) -> Value {
    output(field, "boolean")
}

pub(crate) fn string_output(field: &str) -> Value {
    output(field, "string")
}

pub(crate) fn no_output() -> Value {
    json!({"type":"null"})
}
