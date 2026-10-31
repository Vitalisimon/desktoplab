use desktoplab_agent_engine::DesktopLabToolRegistry;
use serde_json::{Value, json};

pub(super) fn missing_field_schema_guidance(
    reason: &str,
    registry: &DesktopLabToolRegistry,
    suppressed_tool: Option<&str>,
) -> String {
    let Some(field) = reason.strip_prefix("missing_argument:") else {
        return String::new();
    };
    let candidates: Vec<Value> = registry
        .tools()
        .iter()
        .filter(|tool| Some(tool.id()) != suppressed_tool)
        .filter_map(|tool| {
            let required = tool.input_schema().get("required")?.as_array()?;
            if required.len() > 16
                || !required
                    .iter()
                    .all(|value| value.as_str().is_some_and(safe_field))
                || !required.iter().any(|value| value.as_str() == Some(field))
            {
                return None;
            }
            let properties = tool.input_schema().get("properties")?.as_object()?;
            let fields: Vec<Value> = required
                .iter()
                .filter_map(Value::as_str)
                .map(|name| {
                    let kind = properties
                        .get(name)
                        .and_then(|value| value.get("type"))
                        .and_then(Value::as_str);
                    json!({"name": name, "type": kind})
                })
                .collect();
            Some(json!({"tool": tool.id(), "required": fields}))
        })
        .take(8)
        .collect();
    if candidates.is_empty() {
        return String::new();
    }
    format!(
        " Schema candidates: {}. These are possible matches, not evidence of which tool was intended; never invent missing values.",
        Value::Array(candidates)
    )
}

fn safe_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}
