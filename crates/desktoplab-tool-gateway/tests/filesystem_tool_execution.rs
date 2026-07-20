use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome};
use std::fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn filesystem_writes_require_approval_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor =
        FilesystemToolExecutor::new(temp_dir.path(), PolicyEngine::default_conservative());

    let outcome = executor.write("README.md", "hello", FilesystemApproval::Pending);

    assert_eq!(outcome, FilesystemToolOutcome::ApprovalRequired);
    assert!(!temp_dir.path().join("README.md").exists());
    assert_eq!(executor.approval_count(), 1);
    assert_eq!(executor.audit_count(), 1);
}

#[test]
fn approved_write_mutates_file_inside_workspace() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor =
        FilesystemToolExecutor::new(temp_dir.path(), PolicyEngine::default_conservative());

    let outcome = executor.write(
        "src/lib.rs",
        "pub fn ok() {}\n",
        FilesystemApproval::Approved,
    );

    assert_eq!(outcome, FilesystemToolOutcome::Written);
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("src/lib.rs")).unwrap(),
        "pub fn ok() {}\n"
    );
}

#[test]
fn approved_write_reports_when_contents_are_unchanged() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("README.md"), "already current\n").unwrap();
    let mut executor =
        FilesystemToolExecutor::new(temp_dir.path(), PolicyEngine::default_conservative());

    let outcome = executor.write(
        "README.md",
        "already current\n",
        FilesystemApproval::Approved,
    );

    assert_eq!(outcome, FilesystemToolOutcome::Unchanged);
    assert_eq!(
        fs::read_to_string(temp_dir.path().join("README.md")).unwrap(),
        "already current\n"
    );
}

#[test]
fn denied_write_does_not_mutate_file() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor =
        FilesystemToolExecutor::new(temp_dir.path(), PolicyEngine::default_conservative());

    let outcome = executor.write("README.md", "blocked", FilesystemApproval::Denied);

    assert_eq!(outcome, FilesystemToolOutcome::Denied);
    assert!(!temp_dir.path().join("README.md").exists());
}

#[test]
fn local_only_paths_are_blocked_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor =
        FilesystemToolExecutor::new(temp_dir.path(), PolicyEngine::default_conservative());

    for path in [".git/config", ".env", ".ssh/id_rsa", "credentials/token"] {
        let outcome = executor.write(path, "secret", FilesystemApproval::Approved);
        assert_eq!(outcome, FilesystemToolOutcome::Blocked("local_only_path"));
        assert!(!temp_dir.path().join(path).exists());
    }
}

#[test]
fn path_normalization_blocks_workspace_escape() {
    let temp_dir = TempDir::new().unwrap();
    let mut executor =
        FilesystemToolExecutor::new(temp_dir.path(), PolicyEngine::default_conservative());

    let outcome = executor.write("../outside.txt", "escape", FilesystemApproval::Approved);

    assert_eq!(outcome, FilesystemToolOutcome::Blocked("path_escape"));
    assert!(!temp_dir.path().join("../outside.txt").exists());
}

#[test]
fn filesystem_tool_execution_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/filesystem.rs",
        include_str!("../src/filesystem.rs"),
        280,
    )
    .expect("filesystem tool source should stay below the line-count guard");
}
