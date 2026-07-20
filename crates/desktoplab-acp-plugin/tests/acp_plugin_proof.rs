use desktoplab_acp_plugin::{AcpBackendPlugin, PluginTrust};
use desktoplab_execution_router::BackendTrust;
use xtask::check_logical_line_limit;

#[test]
fn acp_is_a_plugin_not_required_by_core() {
    let plugin = AcpBackendPlugin::new_unverified("plugin.acp");

    assert_eq!(plugin.plugin_id(), "plugin.acp");
    assert!(!plugin.is_core_component());
}

#[test]
fn acp_plugin_depends_on_core_backend_contracts() {
    let plugin = AcpBackendPlugin::new_unverified("plugin.acp");
    let manifest = plugin.backend_manifest();

    assert_eq!(manifest.backend_id(), "backend.acp-plugin");
}

#[test]
fn untrusted_acp_plugin_defaults_apply_until_verified() {
    let plugin = AcpBackendPlugin::new_unverified("plugin.acp");
    let candidate = plugin.route_candidate();

    assert_eq!(plugin.trust(), PluginTrust::Unverified);
    assert_eq!(candidate.trust(), BackendTrust::Unverified);
}

#[test]
fn verified_acp_plugin_can_declare_backend_capabilities() {
    let plugin = AcpBackendPlugin::new_verified("plugin.acp");
    let candidate = plugin.route_candidate();

    assert_eq!(plugin.trust(), PluginTrust::Verified);
    assert_eq!(candidate.trust(), BackendTrust::Verified);
}

#[test]
fn acp_plugin_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-acp-plugin/src/lib.rs",
        include_str!("../src/lib.rs"),
        250,
    )
    .expect("acp plugin source should stay below the initial line-count guard");
}
