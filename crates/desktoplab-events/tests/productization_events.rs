use desktoplab_events::{ProductizationEventFamily, ProductizationEventKind};
use xtask::check_logical_line_limit;

#[test]
fn productization_event_families_have_stable_stream_names() {
    assert_eq!(ProductizationEventFamily::Provider.as_str(), "provider");
    assert_eq!(ProductizationEventFamily::Runtime.as_str(), "runtime");
    assert_eq!(ProductizationEventFamily::Model.as_str(), "model");
    assert_eq!(ProductizationEventFamily::AgentTool.as_str(), "agent_tool");
    assert_eq!(
        ProductizationEventFamily::GitWorktree.as_str(),
        "git_worktree"
    );
    assert_eq!(
        ProductizationEventFamily::PluginTrust.as_str(),
        "plugin_trust"
    );
    assert_eq!(
        ProductizationEventFamily::DiagnosticsRepair.as_str(),
        "diagnostics_repair"
    );
}

#[test]
fn productization_event_kind_declares_redaction_requirement() {
    assert!(ProductizationEventKind::ProviderCredentialValidated.requires_redaction());
    assert!(ProductizationEventKind::ProviderConnectivityChecked.requires_redaction());
    assert!(!ProductizationEventKind::RuntimeInstallStarted.requires_redaction());
    assert!(!ProductizationEventKind::ModelDownloadProgress.requires_redaction());
}

#[test]
fn productization_event_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-events/src/productization.rs",
        include_str!("../src/productization.rs"),
        180,
    )
    .expect("productization events source should stay focused");
}
