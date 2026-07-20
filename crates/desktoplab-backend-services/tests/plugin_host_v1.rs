use desktoplab_backend_services::{
    PluginCompatibility, PluginHost, PluginManifest, PluginRouteStatus, PluginTrustState,
};
use xtask::check_logical_line_limit;

#[test]
fn community_plugins_default_to_unverified_trust() {
    let mut host = PluginHost::new("1.0.0");

    let plugin = host.load(PluginManifest::community("plugin.acp", &["llm.chat"]));

    assert_eq!(plugin.trust(), PluginTrustState::Unverified);
}

#[test]
fn unverified_plugins_cannot_silently_escalate_capabilities() {
    let mut host = PluginHost::new("1.0.0");
    host.load(PluginManifest::community(
        "plugin.shell",
        &["tools.terminal.execute"],
    ));

    let route = host.route("plugin.shell");

    assert_eq!(route.status(), PluginRouteStatus::Blocked);
    assert!(
        route
            .reasons()
            .contains(&"unverified_plugin_requires_trust_approval".to_string())
    );
}

#[test]
fn disabled_plugin_cannot_route_execution() {
    let mut host = PluginHost::new("1.0.0");
    host.load(PluginManifest::verified("plugin.acp", &["llm.chat"]));
    host.disable("plugin.acp");

    let route = host.route("plugin.acp");

    assert_eq!(route.status(), PluginRouteStatus::Blocked);
    assert!(route.reasons().contains(&"plugin_disabled".to_string()));
}

#[test]
fn plugin_compatibility_is_manifest_driven() {
    let mut host = PluginHost::new("1.0.0");
    host.load(
        PluginManifest::verified("plugin.future", &["llm.chat"])
            .with_compatibility(PluginCompatibility::requires_desktoplab("2.0.0")),
    );

    let route = host.route("plugin.future");

    assert_eq!(route.status(), PluginRouteStatus::Blocked);
    assert!(
        route
            .reasons()
            .contains(&"plugin_incompatible:requires_desktoplab>=2.0.0".to_string())
    );
}

#[test]
fn plugin_host_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/plugin_host.rs",
        include_str!("../src/plugin_host.rs"),
        300,
    )
    .expect("plugin host source should stay below the line-count guard");
}
