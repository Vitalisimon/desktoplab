use std::fs;

use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    FilesystemApproval, FilesystemMutationExecutor, FilesystemMutationOutcome,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn approved_directory_move_and_delete_use_workspace_capabilities() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("source.txt"), "content").unwrap();
    let mut executor = executor(&root);

    assert_eq!(
        executor.create_directory("nested/empty", FilesystemApproval::Approved),
        FilesystemMutationOutcome::Changed
    );
    assert_eq!(
        executor.move_path(
            "source.txt",
            "nested/moved.txt",
            FilesystemApproval::Approved
        ),
        FilesystemMutationOutcome::Changed
    );
    assert_eq!(
        executor.delete_path("nested", true, FilesystemApproval::Approved),
        FilesystemMutationOutcome::Changed
    );
    assert!(!root.path().join("source.txt").exists());
    assert!(!root.path().join("nested").exists());
}

#[test]
fn mutations_require_approval_and_never_overwrite_destination() {
    let root = TempDir::new().unwrap();
    fs::write(root.path().join("source.txt"), "source").unwrap();
    fs::write(root.path().join("destination.txt"), "destination").unwrap();
    let mut executor = executor(&root);

    assert_eq!(
        executor.delete_path("source.txt", false, FilesystemApproval::Pending),
        FilesystemMutationOutcome::ApprovalRequired
    );
    assert!(matches!(
        executor.move_path(
            "source.txt",
            "destination.txt",
            FilesystemApproval::Approved
        ),
        FilesystemMutationOutcome::Blocked(reason) if reason.contains("destination_exists")
    ));
    assert_eq!(
        fs::read_to_string(root.path().join("destination.txt")).unwrap(),
        "destination"
    );
}

#[test]
fn recursive_delete_is_explicit_and_sensitive_paths_are_blocked() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("nonempty")).unwrap();
    fs::write(root.path().join("nonempty/file.txt"), "content").unwrap();
    fs::create_dir(root.path().join(".git")).unwrap();
    let mut executor = executor(&root);

    assert!(matches!(
        executor.delete_path("nonempty", false, FilesystemApproval::Approved),
        FilesystemMutationOutcome::Blocked(_)
    ));
    assert_eq!(
        executor.delete_path(".git", true, FilesystemApproval::Approved),
        FilesystemMutationOutcome::Blocked("local_only_path".to_string())
    );
    assert!(root.path().join("nonempty/file.txt").exists());
    assert!(root.path().join(".git").exists());
}

#[test]
fn filesystem_mutation_sources_stay_below_line_guard() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-tool-gateway/src/filesystem_mutation.rs",
            include_str!("../src/filesystem_mutation.rs"),
            140,
        ),
        (
            "crates/desktoplab-tool-gateway/tests/filesystem_mutation.rs",
            include_str!("filesystem_mutation.rs"),
            130,
        ),
    ] {
        check_logical_line_limit(path, source, limit).unwrap();
    }
}

fn executor(root: &TempDir) -> FilesystemMutationExecutor {
    FilesystemMutationExecutor::new(root.path(), PolicyEngine::default_conservative())
}
