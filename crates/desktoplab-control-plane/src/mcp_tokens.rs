use desktoplab_tool_gateway::McpTokenSource;

pub(crate) struct NativeMcpTokenSource;

impl McpTokenSource for NativeMcpTokenSource {
    fn access_token(&mut self, vault_ref: &str, _refresh: bool) -> Result<String, String> {
        let secret_ref = desktoplab_vault::SecretRef::from_uri(vault_ref)
            .map_err(|_| "invalid_mcp_vault_ref".to_string())?;
        desktoplab_vault::get_current_native_secret(&secret_ref)
            .map(|secret| secret.expose_for_adapter().to_string())
            .map_err(|_| "mcp_token_unavailable".to_string())
    }
}
