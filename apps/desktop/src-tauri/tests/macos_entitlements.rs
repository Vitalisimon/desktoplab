use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn macos_hardened_runtime_does_not_embed_empty_entitlements() {
    let config = read("tauri.conf.json");
    let entitlements = read("entitlements/macos.plist");

    assert_contains(&config, r#""macOS""#);
    assert_contains(&config, r#""hardenedRuntime": true"#);
    assert_not_contains(&config, r#""entitlements""#);

    assert_contains(&entitlements, "<plist version=\"1.0\">");
    assert_contains(&entitlements, "<dict/>");
    assert_not_contains(&entitlements, "<key>");
}

#[test]
fn broad_file_network_and_automation_entitlements_require_review() {
    let entitlements = read("entitlements/macos.plist");

    for forbidden in [
        "com.apple.security.app-sandbox",
        "com.apple.security.files.",
        "com.apple.security.network.",
        "com.apple.security.device.",
        "com.apple.security.automation.apple-events",
        "com.apple.security.cs.disable-library-validation",
    ] {
        assert_not_contains(&entitlements, forbidden);
    }
}

#[test]
fn macos_entitlements_test_stays_small() {
    xtask::check_logical_line_limit(
        "apps/desktop/src-tauri/tests/macos_entitlements.rs",
        include_str!("macos_entitlements.rs"),
        120,
    )
    .expect("macos entitlements test should stay focused");
}

fn read(path: &str) -> String {
    fs::read_to_string(repo_path(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn repo_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn assert_contains(haystack: &str, needle: &str) {
    assert!(haystack.contains(needle), "missing {needle}");
}

fn assert_not_contains(haystack: &str, needle: &str) {
    assert!(!haystack.contains(needle), "unexpected {needle}");
}
