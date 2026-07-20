use desktoplab_agent_engine::AgentToolSchema;
use serde_json::json;

#[test]
fn mcp_registration_rejects_schema_keywords_the_runtime_cannot_enforce() {
    assert_eq!(
        AgentToolSchema::mcp(
            "mcp.files.pick",
            "Pick a file",
            false,
            json!({
                "type":"object",
                "properties":{
                    "path":{"oneOf":[{"type":"string"},{"type":"null"}]}
                }
            }),
        ),
        Err("mcp_tool_schema_keyword_unsupported:oneOf".to_string())
    );
}

#[test]
fn mcp_registration_accepts_recursively_enforced_schema_constraints() {
    AgentToolSchema::mcp(
        "mcp.files.pick",
        "Pick files",
        false,
        json!({
            "type":"object",
            "properties":{
                "paths":{
                    "type":"array",
                    "minItems":1,
                    "uniqueItems":true,
                    "items":{"type":"string","minLength":1}
                }
            },
            "required":["paths"],
            "additionalProperties":false
        }),
    )
    .expect("supported constraints should register");
}
