use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpTypedTool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolSurface {
    pub server_id: String,
    pub tools: Vec<McpTypedTool>,
}

impl McpToolSurface {
    pub fn from_tools_list(server_id: &str, response: &Value) -> Result<Self, String> {
        let tools = response
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .ok_or_else(|| "mcp_tools_list_invalid".to_string())?;
        let mut typed = Vec::with_capacity(tools.len());
        for tool in tools {
            typed.push(McpTypedTool {
                name: required_string(tool, "name")?,
                description: required_string(tool, "description")?,
                input_schema: tool
                    .get("inputSchema")
                    .cloned()
                    .ok_or_else(|| "mcp_tool_schema_required".to_string())?,
            });
        }
        typed.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(Self {
            server_id: server_id.to_string(),
            tools: typed,
        })
    }

    pub fn relevant_to(&self, task: &str, limit: usize) -> Self {
        let terms: Vec<_> = task
            .split(|character: char| !character.is_ascii_alphanumeric())
            .filter(|term| term.len() >= 3)
            .map(str::to_ascii_lowercase)
            .collect();
        let mut tools: Vec<_> = self
            .tools
            .iter()
            .filter(|tool| {
                let haystack = format!("{} {}", tool.name, tool.description).to_ascii_lowercase();
                terms.iter().any(|term| haystack.contains(term))
            })
            .take(limit)
            .cloned()
            .collect();
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        Self {
            server_id: self.server_id.clone(),
            tools,
        }
    }

    pub fn stable_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| format!("mcp_tool_{key}_required"))
}
