use desktoplab_tool_gateway::{McpImportCandidate, McpServerConfig, McpTransportConfig};
use serde_json::{Value, json};

use super::mcp_persistence::McpServerRegistration;
use super::{ApiRouteResponse, LocalApiRouter};

use crate::canonical_tool_executor::registry_with_mcp_tools;
use crate::mcp_tokens::NativeMcpTokenSource;

impl LocalApiRouter {
    pub(super) fn agent_tool_registry(
        &self,
    ) -> Result<desktoplab_agent_engine::DesktopLabToolRegistry, String> {
        registry_with_mcp_tools(&self.mcp_runtime)
    }

    pub(super) fn backend_tool_schemas(
        &self,
    ) -> Result<Vec<desktoplab_backends::BackendToolSchema>, String> {
        Ok(self
            .agent_tool_registry()?
            .tools()
            .iter()
            .map(|tool| {
                desktoplab_backends::BackendToolSchema::new(
                    tool.id(),
                    tool.description(),
                    tool.input_schema().clone(),
                )
            })
            .collect())
    }

    pub(super) fn agent_tool_ids(&self) -> Result<String, String> {
        Ok(self
            .agent_tool_registry()?
            .tools()
            .iter()
            .map(|tool| tool.id())
            .collect::<Vec<_>>()
            .join(", "))
    }

    pub(super) fn mcp_tools(&self) -> ApiRouteResponse {
        let tools = self
            .mcp_runtime
            .tools()
            .into_iter()
            .map(|tool| {
                let requires_approval = tool.requires_approval();
                json!({
                    "toolId":tool.canonical_id(),
                    "serverId":tool.server_id(),
                    "remoteName":tool.remote_name(),
                    "displayName":tool.description(),
                    "inputSchema":tool.input_schema(),
                    "permissionScopes":tool.permission_scopes(),
                    "status":if requires_approval { "approval_required" } else { "connected" },
                    "requiresApproval":requires_approval,
                    "approvalAction":"mcp.tool.invoke",
                    "auditAction":format!("invoke {} through {}", tool.canonical_id(), tool.server_id()),
                    "blockedReason":Value::Null
                })
            })
            .collect::<Vec<_>>();
        ApiRouteResponse::ok(json!({
            "source":"runtime_backed",
            "status":"connected",
            "sessionOwner":"desktoplab",
            "servers":self.mcp_servers.values().map(|server| {
                let failure = self.mcp_reconnect_failures.get(&server.server_id);
                json!({
                    "serverId":server.server_id,
                    "status":if failure.is_some() { "degraded" } else { "connected" },
                    "detail":failure
                })
            }).collect::<Vec<_>>(),
            "tools":tools
        }))
    }

    pub(super) fn import_mcp_server(&mut self, body: &str) -> ApiRouteResponse {
        let request = match parse_import_request(body) {
            Ok(request) => request,
            Err(error) => return mcp_bad_request(error),
        };
        let mut tokens = NativeMcpTokenSource;
        let registration = McpServerRegistration {
            server_id: request.candidate.config.server_id.clone(),
            transport: request.candidate.config.transport.clone(),
            permission_scopes: request.permission_scopes.clone(),
            requires_approval: request.requires_approval,
            trusted_server: request.trusted_server,
        };
        match self.mcp_runtime.connect_with_tokens(
            request.candidate,
            request.permission_scopes,
            request.requires_approval,
            request.trusted_server,
            &mut tokens,
        ) {
            Ok(_) => {
                self.mcp_servers
                    .insert(registration.server_id.clone(), registration.clone());
                self.mcp_reconnect_failures.remove(&registration.server_id);
                if let Err(error) = self.persist_mcp_servers() {
                    self.mcp_servers.remove(&registration.server_id);
                    let _ = self.mcp_runtime.disconnect(&registration.server_id);
                    return ApiRouteResponse::state_journal_failed(error);
                }
                self.mcp_tools()
            }
            Err(error) => mcp_bad_request(error),
        }
    }

    pub(super) fn disconnect_mcp_server(&mut self, path: &str) -> ApiRouteResponse {
        let server_id = path
            .strip_prefix("/v1/mcp/servers/")
            .and_then(|value| value.strip_suffix("/disconnect"))
            .filter(|value| !value.is_empty());
        let Some(server_id) = server_id else {
            return mcp_bad_request("mcp_server_id_required");
        };
        match self.mcp_runtime.disconnect(server_id) {
            Ok(()) => {
                self.mcp_servers.remove(server_id);
                self.mcp_reconnect_failures.remove(server_id);
                match self.persist_mcp_servers() {
                    Ok(()) => self.mcp_tools(),
                    Err(error) => ApiRouteResponse::state_journal_failed(error),
                }
            }
            Err(error) => mcp_bad_request(error),
        }
    }
}

struct McpImportRequest {
    candidate: McpImportCandidate,
    permission_scopes: Vec<String>,
    requires_approval: bool,
    trusted_server: bool,
}

fn parse_import_request(body: &str) -> Result<McpImportRequest, String> {
    let value: Value = serde_json::from_str(body).map_err(|_| "invalid_json".to_string())?;
    let server_id = required_string(&value, "serverId")?;
    let reviewed = value
        .get("reviewed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let permission_scopes = value
        .get("permissionScopes")
        .and_then(Value::as_array)
        .ok_or_else(|| "permission_scope_required".to_string())?
        .iter()
        .map(|scope| {
            scope
                .as_str()
                .filter(|scope| !scope.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| "invalid_permission_scope".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if permission_scopes.is_empty() {
        return Err("permission_scope_required".to_string());
    }
    let transport = value
        .get("transport")
        .ok_or_else(|| "mcp_transport_required".to_string())?;
    let transport = match required_string(transport, "kind")?.as_str() {
        "http" => McpTransportConfig::Http {
            endpoint: required_string(transport, "endpoint")?,
            vault_ref: optional_string(transport, "vaultRef"),
            streaming: transport
                .get("streaming")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        },
        "stdio" => McpTransportConfig::Stdio {
            program: required_string(transport, "program")?,
            args: optional_string_array(transport, "args")?,
        },
        _ => return Err("unsupported_mcp_transport".to_string()),
    };
    Ok(McpImportRequest {
        candidate: McpImportCandidate {
            config: McpServerConfig {
                server_id,
                transport,
            },
            reviewed,
        },
        permission_scopes,
        requires_approval: value
            .get("requiresApproval")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        trusted_server: value
            .get("trustedServer")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    optional_string(value, field).ok_or_else(|| format!("{field}_required"))
}

fn optional_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn optional_string_array(value: &Value, field: &str) -> Result<Vec<String>, String> {
    value
        .get(field)
        .map(|items| {
            items
                .as_array()
                .ok_or_else(|| format!("invalid_{field}"))?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| format!("invalid_{field}"))
                })
                .collect()
        })
        .unwrap_or_else(|| Ok(Vec::new()))
}

fn mcp_bad_request(error: impl Into<String>) -> ApiRouteResponse {
    ApiRouteResponse::bad_request(json!({
        "code":"MCP_REQUEST_FAILED",
        "message":"DesktopLab could not complete the MCP request.",
        "detail":error.into()
    }))
}
