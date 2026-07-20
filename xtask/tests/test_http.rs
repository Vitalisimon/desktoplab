use std::path::Path;

use xtask::test_http::{workspace_initialize_body, workspace_open_body};

#[test]
fn workspace_payload_escapes_windows_path_separators() {
    let body = workspace_open_body(Path::new(r"C:\Users\DesktopLab\project"));
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert_eq!(parsed["path"], r"C:\Users\DesktopLab\project");
}

#[test]
fn initialize_payload_adds_only_the_explicit_git_flag() {
    let body = workspace_initialize_body(Path::new("/tmp/project"));
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert_eq!(
        parsed,
        serde_json::json!({
            "path": "/tmp/project",
            "initializeGit": true
        })
    );
}
