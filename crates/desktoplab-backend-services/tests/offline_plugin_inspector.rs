use std::fs;

use desktoplab_backend_services::{PluginCompatibilityInspector, PluginFindingSeverity};
use tempfile::TempDir;

#[test]
fn compatible_package_is_inspected_without_execution_or_network() {
    let fixture = plugin_fixture(
        r#"{
          "pluginId":"plugin.example",
          "contractVersion":"1",
          "entry":"dist/index.js",
          "sdkImports":["@desktoplab/plugin-sdk/v1"],
          "registrations":["tool"],
          "hooks":["tool","shutdown"],
          "permissions":["llm.chat"]
        }"#,
        r#"{"name":"plugin.example","version":"1.0.0"}"#,
    );

    let report = PluginCompatibilityInspector::inspect(fixture.path());
    assert!(report.compatible);
    assert!(!report.executed_plugin_code);
    assert!(!report.used_network);
    assert!(report.json().contains(r#""schemaVersion": 1"#));
    assert!(report.markdown().contains("executed plugin code: `false`"));
    assert!(report.sarif().contains("2.1.0"));
    assert!(report.junit().contains("failures=\"0\""));
}

#[test]
fn malformed_incompatible_and_unsafe_packages_fail_deterministically() {
    let malformed = TempDir::new().unwrap();
    fs::write(malformed.path().join("desktoplab-plugin.json"), "{").unwrap();
    let first = PluginCompatibilityInspector::inspect(malformed.path());
    let second = PluginCompatibilityInspector::inspect(malformed.path());
    assert_eq!(first, second);
    assert!(!first.compatible);
    assert_eq!(first.findings[0].code, "metadata_malformed");

    let unsafe_package = plugin_fixture(
        r#"{
          "pluginId":"plugin.unsafe",
          "contractVersion":"2",
          "entry":"../outside.js",
          "sdkImports":["desktoplab-control-plane/internal"],
          "registrations":["root"],
          "hooks":["unknown"],
          "permissions":["network.raw"]
        }"#,
        r#"{"name":"other","version":""}"#,
    );
    let report = PluginCompatibilityInspector::inspect(unsafe_package.path());
    assert!(!report.compatible);
    assert!(report.findings.iter().any(|finding| {
        finding.severity == PluginFindingSeverity::Incompatibility
            && finding.code == "unsafe_entry_path"
    }));
    assert!(report.sarif().contains("unsupported_contract_version"));
    assert!(report.junit().contains("<failure"));
}

#[test]
fn inspector_sources_stay_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/src/plugin_inspector/mod.rs",
        include_str!("../src/plugin_inspector/mod.rs"),
        260,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/src/plugin_inspector/report.rs",
        include_str!("../src/plugin_inspector/report.rs"),
        220,
    )
    .unwrap();
}

fn plugin_fixture(manifest: &str, package: &str) -> TempDir {
    let fixture = TempDir::new().unwrap();
    fs::create_dir_all(fixture.path().join("dist")).unwrap();
    fs::write(fixture.path().join("dist/index.js"), "export default {};").unwrap();
    fs::write(fixture.path().join("desktoplab-plugin.json"), manifest).unwrap();
    fs::write(fixture.path().join("package.json"), package).unwrap();
    fixture
}
