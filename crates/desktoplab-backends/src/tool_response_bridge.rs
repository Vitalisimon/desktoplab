use serde_json::{Map, Value, json};

use crate::BackendToolResponse;

pub fn backend_response_to_agent_text(response: BackendToolResponse) -> Result<String, String> {
    if let Some(error) = response.protocol_error() {
        return Err(error.to_string());
    }
    match response.tool_calls() {
        [] => response
            .assistant_text()
            .map(ToString::to_string)
            .ok_or_else(|| "provider_response_missing_content".to_string()),
        [call] => {
            let mut envelope = Map::from_iter([
                (
                    "assistantMessage".to_string(),
                    json!(response.assistant_text().unwrap_or("")),
                ),
                ("tool".to_string(), json!(call.name())),
                ("arguments".to_string(), call.arguments().clone()),
            ]);
            if let Some(id) = call.id().filter(|id| !id.trim().is_empty()) {
                envelope.insert("id".to_string(), Value::String(id.to_string()));
            }
            Ok(Value::Object(envelope).to_string())
        }
        _ => Err("parallel_tool_calls_unsupported".to_string()),
    }
}
