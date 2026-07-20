use desktoplab_acp_plugin::{AcpBackendPlugin, AcpPluginLoader, PluginTrust};
use desktoplab_execution_router::{ExecutionRouter, RoutePolicy, RouteRequest, RouteStatus};

#[test]
fn acp_bridge_loads_through_plugin_trust_not_core_assumption() {
    let loader = AcpPluginLoader::default();
    let plugin = loader.load(AcpBackendPlugin::new_unverified("plugin.acp"));

    assert!(!plugin.is_core_component());
    assert_eq!(plugin.trust(), PluginTrust::Unverified);
    assert!(loader.loaded_through_plugin_host());
}

#[test]
fn acp_capabilities_map_to_known_backend_vocabulary() {
    let plugin = AcpBackendPlugin::new_verified("plugin.acp");
    let route = ExecutionRouter::new(RoutePolicy::local_only()).select(
        RouteRequest::new(&["llm.chat", "agent.events.stream", "plugin.acp"]),
        vec![plugin.route_candidate()],
    );

    assert_eq!(route.status(), RouteStatus::Selected);
}
