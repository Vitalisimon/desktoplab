use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use desktoplab_redaction::redact_sensitive;
use serde_json::{Value, json};

use crate::{McpConnectionPool, McpImportCandidate, McpTokenSource, McpToolSurface, NoMcpToken};

const MAX_MCP_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, PartialEq)]
pub struct ConnectedMcpTool {
    canonical_id: String,
    server_id: String,
    remote_name: String,
    description: String,
    input_schema: Value,
    permission_scopes: Vec<String>,
    requires_approval: bool,
    trusted_server: bool,
}

impl ConnectedMcpTool {
    pub fn canonical_id(&self) -> &str {
        &self.canonical_id
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    pub fn remote_name(&self) -> &str {
        &self.remote_name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn input_schema(&self) -> &Value {
        &self.input_schema
    }

    pub fn permission_scopes(&self) -> &[String] {
        &self.permission_scopes
    }

    pub fn requires_approval(&self) -> bool {
        self.requires_approval || !self.trusted_server
    }
}

#[derive(Clone, Default)]
pub struct SharedMcpRuntime {
    inner: Arc<Mutex<McpRuntime>>,
}

impl SharedMcpRuntime {
    pub fn connect(
        &self,
        candidate: McpImportCandidate,
        permission_scopes: Vec<String>,
        requires_approval: bool,
        trusted_server: bool,
    ) -> Result<Vec<ConnectedMcpTool>, String> {
        self.connect_with_tokens(
            candidate,
            permission_scopes,
            requires_approval,
            trusted_server,
            &mut NoMcpToken,
        )
    }

    pub fn connect_with_tokens(
        &self,
        candidate: McpImportCandidate,
        permission_scopes: Vec<String>,
        requires_approval: bool,
        trusted_server: bool,
        tokens: &mut dyn McpTokenSource,
    ) -> Result<Vec<ConnectedMcpTool>, String> {
        self.inner
            .lock()
            .map_err(|_| "mcp_runtime_poisoned".to_string())?
            .connect(
                candidate,
                permission_scopes,
                requires_approval,
                trusted_server,
                tokens,
            )
    }

    pub fn tools(&self) -> Vec<ConnectedMcpTool> {
        self.inner
            .lock()
            .map(|runtime| runtime.tools.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn invoke(
        &self,
        canonical_id: &str,
        arguments: Value,
        approved: bool,
    ) -> Result<Value, String> {
        self.invoke_with_tokens(canonical_id, arguments, approved, &mut NoMcpToken)
    }

    pub fn invoke_with_tokens(
        &self,
        canonical_id: &str,
        arguments: Value,
        approved: bool,
        tokens: &mut dyn McpTokenSource,
    ) -> Result<Value, String> {
        self.inner
            .lock()
            .map_err(|_| "mcp_runtime_poisoned".to_string())?
            .invoke(canonical_id, arguments, approved, tokens)
    }

    pub fn disconnect(&self, server_id: &str) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|_| "mcp_runtime_poisoned".to_string())?
            .disconnect(server_id)
    }
}

#[derive(Default)]
struct McpRuntime {
    pool: McpConnectionPool,
    tools: BTreeMap<String, ConnectedMcpTool>,
}

impl McpRuntime {
    fn connect(
        &mut self,
        candidate: McpImportCandidate,
        permission_scopes: Vec<String>,
        requires_approval: bool,
        trusted_server: bool,
        tokens: &mut dyn McpTokenSource,
    ) -> Result<Vec<ConnectedMcpTool>, String> {
        if permission_scopes.is_empty() {
            return Err("permission_scope_required".to_string());
        }
        let server_id = candidate.config.server_id.clone();
        self.pool.import(candidate)?;
        let response = match self
            .pool
            .request(&server_id, "tools/list", json!({}), tokens)
        {
            Ok(response) => response,
            Err(error) => {
                let _ = self.pool.disconnect(&server_id);
                return Err(error);
            }
        };
        let surface = match McpToolSurface::from_tools_list(&server_id, &response) {
            Ok(surface) => surface,
            Err(error) => {
                let _ = self.pool.disconnect(&server_id);
                return Err(error);
            }
        };
        let mut connected = Vec::new();
        let mut ids = BTreeSet::new();
        for tool in surface.tools {
            let canonical_id = canonical_tool_id(&server_id, &tool.name);
            if self.tools.contains_key(&canonical_id) || !ids.insert(canonical_id.clone()) {
                let _ = self.pool.disconnect(&server_id);
                return Err("mcp_tool_id_collision".to_string());
            }
            let input_schema = normalized_input_schema(tool.input_schema)?;
            connected.push(ConnectedMcpTool {
                canonical_id,
                server_id: server_id.clone(),
                remote_name: tool.name,
                description: tool.description,
                input_schema,
                permission_scopes: permission_scopes.clone(),
                requires_approval,
                trusted_server,
            });
        }
        for tool in &connected {
            self.tools.insert(tool.canonical_id.clone(), tool.clone());
        }
        Ok(connected)
    }

    fn invoke(
        &mut self,
        canonical_id: &str,
        arguments: Value,
        approved: bool,
        tokens: &mut dyn McpTokenSource,
    ) -> Result<Value, String> {
        let tool = self
            .tools
            .get(canonical_id)
            .cloned()
            .ok_or_else(|| "mcp_tool_not_connected".to_string())?;
        if tool.requires_approval() && !approved {
            return Err("approval_required".to_string());
        }
        let response = self.pool.request(
            &tool.server_id,
            "tools/call",
            json!({"name":tool.remote_name,"arguments":arguments}),
            tokens,
        )?;
        if response.to_string().len() > MAX_MCP_RESPONSE_BYTES {
            return Err("mcp_response_too_large".to_string());
        }
        Ok(redact_json(response))
    }

    fn disconnect(&mut self, server_id: &str) -> Result<(), String> {
        self.pool.disconnect(server_id)?;
        self.tools.retain(|_, tool| tool.server_id != server_id);
        Ok(())
    }
}

fn canonical_tool_id(server_id: &str, tool_name: &str) -> String {
    format!("mcp.{}.{}", slug(server_id), slug(tool_name))
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

fn normalized_input_schema(mut schema: Value) -> Result<Value, String> {
    let object = schema
        .as_object_mut()
        .ok_or_else(|| "mcp_tool_schema_required".to_string())?;
    if object.get("type").and_then(Value::as_str) != Some("object") {
        return Err("mcp_tool_schema_invalid".to_string());
    }
    object.entry("properties").or_insert_with(|| json!({}));
    object.entry("required").or_insert_with(|| json!([]));
    object
        .entry("additionalProperties")
        .or_insert_with(|| json!(false));
    Ok(schema)
}

fn redact_json(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(redact_sensitive(&value)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact_json).collect()),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, redact_json(value)))
                .collect(),
        ),
        scalar => scalar,
    }
}
