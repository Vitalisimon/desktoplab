use std::collections::BTreeMap;

use desktoplab_storage::{ProductizationRecordKind, SqliteStore, StorageError};
use desktoplab_tool_gateway::{McpImportCandidate, McpServerConfig, McpTransportConfig};
use serde_json::{Value, json};

use crate::mcp_tokens::NativeMcpTokenSource;

use super::LocalApiRouter;
use super::persistence_payloads;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct McpServerRegistration {
    pub(crate) server_id: String,
    pub(crate) transport: McpTransportConfig,
    pub(crate) permission_scopes: Vec<String>,
    pub(crate) requires_approval: bool,
    pub(crate) trusted_server: bool,
}

impl McpServerRegistration {
    pub(crate) fn candidate(&self) -> McpImportCandidate {
        McpImportCandidate {
            config: McpServerConfig {
                server_id: self.server_id.clone(),
                transport: self.transport.clone(),
            },
            reviewed: true,
        }
    }

    pub(crate) fn to_json(&self) -> Value {
        let transport = match &self.transport {
            McpTransportConfig::Http {
                endpoint,
                vault_ref,
                streaming,
            } => json!({
                "kind":"http","endpoint":endpoint,"vaultRef":vault_ref,"streaming":streaming
            }),
            McpTransportConfig::Stdio { program, args } => {
                json!({"kind":"stdio","program":program,"args":args})
            }
        };
        json!({
            "serverId":self.server_id,
            "reviewed":true,
            "permissionScopes":self.permission_scopes,
            "requiresApproval":self.requires_approval,
            "trustedServer":self.trusted_server,
            "transport":transport
        })
    }

    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        let transport = value.get("transport")?;
        let transport = match transport.get("kind")?.as_str()? {
            "http" => McpTransportConfig::Http {
                endpoint: transport.get("endpoint")?.as_str()?.to_string(),
                vault_ref: transport
                    .get("vaultRef")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                streaming: transport
                    .get("streaming")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            },
            "stdio" => McpTransportConfig::Stdio {
                program: transport.get("program")?.as_str()?.to_string(),
                args: transport
                    .get("args")
                    .and_then(Value::as_array)?
                    .iter()
                    .map(|arg| arg.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()?,
            },
            _ => return None,
        };
        Some(Self {
            server_id: value.get("serverId")?.as_str()?.to_string(),
            transport,
            permission_scopes: value
                .get("permissionScopes")?
                .as_array()?
                .iter()
                .map(|scope| scope.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()?,
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
}

impl LocalApiRouter {
    pub(crate) fn persist_mcp_servers(&self) -> Result<(), StorageError> {
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        persistence_payloads::persist_state(
            storage,
            ProductizationRecordKind::McpServerRegistry,
            "local",
            json!({
                "servers":self.mcp_servers.values().map(McpServerRegistration::to_json).collect::<Vec<_>>()
            }),
        )
    }

    pub(crate) fn reconnect_mcp_servers(&mut self) {
        for registration in self.mcp_servers.values().cloned().collect::<Vec<_>>() {
            let result = self.mcp_runtime.connect_with_tokens(
                registration.candidate(),
                registration.permission_scopes.clone(),
                registration.requires_approval,
                registration.trusted_server,
                &mut NativeMcpTokenSource,
            );
            match result {
                Ok(_) => {
                    self.mcp_reconnect_failures.remove(&registration.server_id);
                }
                Err(error) => {
                    self.mcp_reconnect_failures
                        .insert(registration.server_id, error);
                }
            }
        }
    }
}

pub(crate) fn load_mcp_servers(
    storage: &SqliteStore,
) -> Result<BTreeMap<String, McpServerRegistration>, StorageError> {
    let Some(record) =
        storage.get_productization_state(ProductizationRecordKind::McpServerRegistry, "local")?
    else {
        return Ok(BTreeMap::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(McpServerRegistration::from_json)
        .map(|server| (server.server_id.clone(), server))
        .collect())
}
