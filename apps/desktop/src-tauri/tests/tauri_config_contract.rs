use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn bundle_is_enabled_with_explicit_dev_targets_and_window_contract() {
    let config = read("tauri.conf.json");

    assert_contains(&config, r#""productName": "DesktopLab""#);
    assert_contains(&config, r#""identifier": "ai.desktoplab.desktop""#);
    assert_contains(&config, r#""version": "0.1.0""#);
    assert_contains(&config, r#""minWidth": 980"#);
    assert_contains(&config, r#""minHeight": 680"#);
    assert_contains(&config, r#""active": true"#);
    assert_contains(&config, r#""targets": ["app", "dmg"]"#);
    assert_not_contains(&config, r#""targets": "all""#);
    assert_contains(&config, r#""infoPlist": "Info.plist""#);
    assert_contains(&config, r#""minimumSystemVersion": "13.0""#);
}

#[test]
fn security_config_has_csp_and_narrow_command_capabilities() {
    let config = read("tauri.conf.json");
    let macos_config = read("tauri.macos.conf.json");
    let capability = read("capabilities/default.json");
    let permission = read("permissions/local-api-bootstrap.toml");

    assert_not_contains(&config, r#""csp": null"#);
    assert_contains(&config, "default-src 'self'");
    assert_contains(&config, "connect-src http://127.0.0.1:* http://localhost:*");
    assert_not_contains(&config, r#""trafficLightPosition""#);
    assert_contains(&macos_config, r#""trafficLightPosition""#);
    assert_contains(&macos_config, r#""y": 18"#);

    assert_contains(&capability, r#""local-api-bootstrap""#);
    assert_contains(&capability, r#""window-shell""#);
    assert_not_contains(&capability, "shell:");
    assert_not_contains(&capability, "opener:");
    assert_not_contains(&capability, "path:");
    assert_contains(&permission, r#"commands.allow = ["local_api_bootstrap"]"#);
    assert_contains(
        &read("permissions/window-shell.toml"),
        r#"commands.allow = ["start_window_drag", "toggle_window_maximized", "open_repository_in_file_manager", "repository_open_targets", "open_repository_in_target", "open_external_url"]"#,
    );
}

#[test]
fn repository_selection_uses_the_official_in_process_dialog() {
    let cargo = read("Cargo.toml");
    let library = read("src/lib.rs");
    let commands = read("src/commands.rs");
    let capability = read("capabilities/default.json");
    let picker = read("../src/features/workspaces/repositoryFolderPicker.ts");

    assert_contains(&cargo, "tauri-plugin-dialog");
    assert_contains(&library, ".plugin(tauri_plugin_dialog::init())");
    assert_contains(&capability, r#""dialog:allow-open""#);
    assert_contains(&picker, r#"from "@tauri-apps/plugin-dialog""#);
    assert_contains(&picker, "directory: true");
    assert_contains(&picker, "multiple: false");

    for external_picker in [
        "choose_repository_folder",
        "osascript",
        "zenity",
        "kdialog",
        "FolderBrowserDialog",
    ] {
        assert_not_contains(&commands, external_picker);
        assert_not_contains(&library, external_picker);
    }
    assert_not_contains(&capability, "repository-folder-picker");
}

#[test]
fn window_chrome_is_native_except_for_the_macos_overlay() {
    let config = read("tauri.conf.json");
    let macos_config = read("tauri.macos.conf.json");
    let windows_config = read("tauri.windows.conf.json");
    let linux_config = read("tauri.linux.conf.json");
    let html = read("../index.html");

    assert_contains(&config, r#""title": "DesktopLab""#);
    assert_not_contains(&config, r#""titleBarStyle""#);
    assert_not_contains(&config, r#""hiddenTitle""#);
    assert_contains(&macos_config, r#""titleBarStyle": "Overlay""#);
    assert_contains(&macos_config, r#""hiddenTitle": true"#);
    assert_not_contains(&windows_config, r#""titleBarStyle""#);
    assert_not_contains(&linux_config, r#""titleBarStyle""#);
    assert_contains(&html, "<title>DesktopLab</title>");
}

#[test]
fn app_metadata_and_icons_cover_desktop_bundle_needs() {
    let config = read("tauri.conf.json");
    let plist = read("Info.plist");

    assert_contains(&config, r#""category": "DeveloperTool""#);
    assert_contains(&config, r#""shortDescription": "Local-first AI development agent workbench""#);
    assert_contains(&config, r#""copyright": "Copyright 2026 DesktopLab contributors""#);
    assert_contains(&config, r#""contentTypes": ["public.folder"]"#);
    assert_contains(&config, r#""DesktopLab Repository Folder""#);
    assert_contains(&plist, "CFBundleURLSchemes");
    assert_contains(&plist, "desktoplab");
    assert_contains(&plist, "NSDocumentsFolderUsageDescription");
    assert_contains(&plist, "NSDesktopFolderUsageDescription");
    assert_contains(&plist, "NSDownloadsFolderUsageDescription");
    assert_not_contains(&plist, "NSMicrophoneUsageDescription");
    assert_not_contains(&plist, "NSCameraUsageDescription");
    assert_not_contains(&plist, "NSSpeechRecognitionUsageDescription");

    for icon in [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/128x128@2x.png",
        "icons/icon.icns",
        "icons/icon.ico",
        "icons/icon.png",
    ] {
        assert!(repo_path(icon).exists(), "missing bundle icon: {icon}");
        assert_contains(&config, icon);
    }
}

#[test]
fn windows_nsis_metadata_is_explicit_and_current_user_scoped() {
    let config = read("tauri.conf.json");

    assert_contains(&config, r#""identifier": "ai.desktoplab.desktop""#);
    assert_contains(&config, r#""windows""#);
    assert_contains(&config, r#""digestAlgorithm": "sha256""#);
    assert_contains(&config, r#""allowDowngrades": false"#);
    assert_contains(&config, r#""nsis""#);
    assert_contains(&config, r#""installerIcon": "icons/icon.ico""#);
    assert_contains(&config, r#""uninstallerIcon": "icons/icon.ico""#);
    assert_contains(&config, r#""installMode": "currentUser""#);
    assert_contains(&config, r#""startMenuFolder": "DesktopLab""#);
    assert_not_contains(&config, r#""installMode": "perMachine""#);
    assert_not_contains(&config, r#""installMode": "both""#);
}

#[test]
fn linux_package_metadata_and_desktop_integration_are_explicit() {
    let config = read("tauri.conf.json");
    let linux_config = read("tauri.linux.conf.json");

    assert_contains(&config, r#""category": "DeveloperTool""#);
    assert_contains(&config, r#""icons/icon.png""#);
    assert_contains(&config, r#""linux""#);
    assert_contains(&linux_config, r#""targets": ["appimage", "deb"]"#);
    assert_contains(&config, r#""appimage""#);
    assert_contains(&config, r#""bundleMediaFramework": false"#);
    assert_contains(&config, r#""deb""#);
    assert_contains(&config, r#""section": "devel""#);
    assert_contains(&config, r#""priority": "optional""#);
    assert_not_contains(&linux_config, r#""rpm""#);
}

#[test]
fn windows_package_overlay_builds_nsis_only() {
    let windows_config = read("tauri.windows.conf.json");

    assert_contains(&windows_config, r#""targets": ["nsis"]"#);
    assert_not_contains(&windows_config, r#""msi""#);
}

#[test]
fn tauri_config_contract_test_stays_small() {
    xtask::check_logical_line_limit(
        "apps/desktop/src-tauri/tests/tauri_config_contract.rs",
        include_str!("tauri_config_contract.rs"),
        250,
    )
    .expect("tauri config contract test should stay focused");
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
