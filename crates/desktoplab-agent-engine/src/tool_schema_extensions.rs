use serde_json::{Value, json};

use crate::mcp_schema_validator::validate_supported_schema;
use crate::{AgentToolRisk, AgentToolSchema};

impl AgentToolSchema {
    pub fn mcp(
        id: impl Into<String>,
        description: impl Into<String>,
        requires_approval: bool,
        input_schema: Value,
    ) -> Result<Self, String> {
        let tool = Self::new(
            id,
            description,
            if requires_approval {
                AgentToolRisk::High
            } else {
                AgentToolRisk::Medium
            },
            requires_approval,
            input_schema,
            json!({"type":"object"}),
        );
        validate_mcp_tool(&tool)?;
        Ok(tool)
    }
}

pub(crate) fn validate_mcp_tool(tool: &AgentToolSchema) -> Result<(), String> {
    let id = tool.id();
    if !id.starts_with("mcp.")
        || id.len() > 160
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("mcp_tool_id_invalid".to_string());
    }
    let schema = tool.input_schema();
    if schema.get("type").and_then(Value::as_str) != Some("object")
        || !schema.get("properties").is_some_and(Value::is_object)
    {
        return Err("mcp_tool_schema_invalid".to_string());
    }
    validate_supported_schema(schema)?;
    Ok(())
}
