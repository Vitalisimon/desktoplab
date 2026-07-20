use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome};
use std::fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[cfg(unix)]
use desktoplab_tool_gateway::{
    TerminalApproval, TerminalCommandRequest, TerminalToolExecutor, TerminalToolOutcome,
};
#[cfg(unix)]
use std::os::unix::fs::symlink;
#[cfg(unix)]
use std::time::Duration;

#[test]
#[cfg(unix)]
fn filesystem_write_denies_symlink_that_escapes_workspace() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "do not overwrite").unwrap();
    symlink(
        outside.join("secret.txt"),
        workspace.join("linked-secret.txt"),
    )
    .unwrap();
    let mut executor =
        FilesystemToolExecutor::new(&workspace, PolicyEngine::default_conservative());

    let outcome = executor.write(
        "linked-secret.txt",
        "overwritten",
        FilesystemApproval::Approved,
    );

    assert_eq!(outcome, FilesystemToolOutcome::Blocked("path_escape"));
    assert_eq!(
        fs::read_to_string(outside.join("secret.txt")).unwrap(),
        "do not overwrite"
    );
}

#[test]
#[cfg(unix)]
fn filesystem_read_denies_symlink_that_escapes_workspace() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "do not read").unwrap();
    symlink(
        outside.join("secret.txt"),
        workspace.join("linked-secret.txt"),
    )
    .unwrap();
    let mut executor =
        FilesystemToolExecutor::new(&workspace, PolicyEngine::default_conservative());

    let outcome = executor.read("linked-secret.txt");

    assert_eq!(outcome, FilesystemToolOutcome::Blocked("path_escape"));
}

#[test]
#[cfg(unix)]
fn terminal_cwd_denies_symlink_that_escapes_workspace() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, workspace.join("linked-outside")).unwrap();
    let mut executor = TerminalToolExecutor::new(
        &workspace,
        PolicyEngine::default_conservative(),
        Duration::from_secs(1),
        1024,
    );

    let outcome = executor.execute(
        TerminalCommandRequest::new("workspace.fixture", "pwd")
            .with_working_directory("linked-outside"),
        TerminalApproval::Approved,
    );

    assert_eq!(outcome, TerminalToolOutcome::Blocked("path_escape"));
}

#[test]
#[cfg(any(unix, windows))]
fn filesystem_write_denies_new_descendant_below_outside_directory_link() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    create_directory_link(&outside, &workspace.join("linked-outside"));
    let mut executor =
        FilesystemToolExecutor::new(&workspace, PolicyEngine::default_conservative());

    let outcome = executor.write(
        "linked-outside/new/escape.txt",
        "must stay contained",
        FilesystemApproval::Approved,
    );

    assert_eq!(outcome, FilesystemToolOutcome::Blocked("path_escape"));
    assert!(!outside.join("new/escape.txt").exists());
}

#[test]
fn path_security_source_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/path_security.rs",
        include_str!("../src/path_security.rs"),
        140,
    )
    .expect("path security source should stay below the line-count guard");
}

#[cfg(unix)]
fn create_directory_link(target: &std::path::Path, link: &std::path::Path) {
    symlink(target, link).expect("directory symlink should be created");
}

#[cfg(windows)]
fn create_directory_link(target: &std::path::Path, link: &std::path::Path) {
    let status = std::process::Command::new("cmd.exe")
        .args([
            "/C",
            "mklink",
            "/J",
            link.to_str().unwrap(),
            target.to_str().unwrap(),
        ])
        .status()
        .expect("junction command should start");
    assert!(status.success(), "junction should be created");
}
