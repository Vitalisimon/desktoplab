use desktoplab_agent_engine::{
    AgentToolSchema, DesktopLabToolRegistry, ProviderToolCallNormalizer, ToolCallNormalizationError,
};
use serde_json::json;

#[test]
fn runtime_mcp_tools_extend_the_canonical_registry() {
    let registry = registry();
    assert!(registry.get("mcp.server.search").is_some());
    assert!(
        registry
            .provider_tool_schemas()
            .iter()
            .any(|schema| schema["function"]["name"] == "mcp.server.search")
    );
}

#[test]
fn mcp_schema_and_typed_arguments_fail_closed() {
    assert!(
        AgentToolSchema::mcp(
            "desktoplab.fake",
            "invalid namespace",
            false,
            json!({"type":"object","properties":{}}),
        )
        .is_err()
    );
    let normalizer = ProviderToolCallNormalizer::new(registry());
    assert_eq!(
        normalizer.normalize(
            "call.1",
            "mcp.server.search",
            json!({"query":"rust","limit":"ten"}),
        ),
        Err(ToolCallNormalizationError::InvalidArgumentType(
            "limit".to_string()
        ))
    );
    assert!(
        normalizer
            .normalize(
                "call.2",
                "mcp.server.search",
                json!({"query":"rust","limit":10}),
            )
            .is_ok()
    );
}

fn registry() -> DesktopLabToolRegistry {
    let mcp = AgentToolSchema::mcp(
        "mcp.server.search",
        "Search the connected MCP server.",
        true,
        json!({
            "type":"object",
            "properties":{"query":{"type":"string"},"limit":{"type":"integer"}},
            "required":["query"],"additionalProperties":false
        }),
    )
    .unwrap();
    DesktopLabToolRegistry::default()
        .with_mcp_tools([mcp])
        .unwrap()
}
