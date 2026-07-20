use std::fs;

use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    FilesystemPatchApproval, FilesystemPatchExecutor, FilesystemPatchOutcome,
    FilesystemPatchRequest,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn approved_patch_replaces_only_expected_anchor_and_records_diff() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("notes.md"), "alpha\nbeta\ngamma\n").unwrap();
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let outcome = executor.apply(
        FilesystemPatchRequest::replace("notes.md", "beta\n", "beta updated\n"),
        FilesystemPatchApproval::Approved,
    );

    match outcome {
        FilesystemPatchOutcome::Patched(evidence) => {
            assert_eq!(
                fs::read_to_string(repo.path().join("notes.md")).unwrap(),
                "alpha\nbeta updated\ngamma\n"
            );
            assert!(evidence.before_diff().contains("-beta"));
            assert!(evidence.after_diff().contains("+beta updated"));
        }
        other => panic!("expected patch, got {other:?}"),
    }
}

#[test]
fn patch_conflict_preserves_user_changes() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("notes.md"), "alpha\nuser changed\ngamma\n").unwrap();
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let outcome = executor.apply(
        FilesystemPatchRequest::replace("notes.md", "beta\n", "beta updated\n"),
        FilesystemPatchApproval::Approved,
    );

    assert_eq!(outcome, FilesystemPatchOutcome::Blocked("patch_conflict"));
    assert_eq!(
        fs::read_to_string(repo.path().join("notes.md")).unwrap(),
        "alpha\nuser changed\ngamma\n"
    );
}

#[test]
fn duplicate_anchor_blocks_unless_replace_all_is_explicit() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("notes.md"), "same\nmiddle\nsame\n").unwrap();
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let ambiguous = executor.apply(
        FilesystemPatchRequest::replace("notes.md", "same", "changed"),
        FilesystemPatchApproval::Approved,
    );
    let replaced = executor.apply(
        FilesystemPatchRequest::replace("notes.md", "same", "changed").with_replace_all(),
        FilesystemPatchApproval::Approved,
    );

    assert_eq!(
        ambiguous,
        FilesystemPatchOutcome::Blocked("patch_ambiguous")
    );
    assert!(matches!(replaced, FilesystemPatchOutcome::Patched(_)));
    assert_eq!(
        fs::read_to_string(repo.path().join("notes.md")).unwrap(),
        "changed\nmiddle\nchanged\n"
    );
}

#[test]
fn patch_accepts_lf_anchor_and_preserves_crlf_file_endings() {
    let repo = TempDir::new().unwrap();
    fs::write(
        repo.path().join("windows.txt"),
        "alpha\r\nbeta\r\ngamma\r\n",
    )
    .unwrap();
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let outcome = executor.apply(
        FilesystemPatchRequest::replace("windows.txt", "beta\ngamma\n", "beta updated\ngamma\n"),
        FilesystemPatchApproval::Approved,
    );

    assert!(matches!(outcome, FilesystemPatchOutcome::Patched(_)));
    assert_eq!(
        fs::read(repo.path().join("windows.txt")).unwrap(),
        b"alpha\r\nbeta updated\r\ngamma\r\n"
    );
}

#[test]
fn patch_blocks_path_escape_protected_path_and_symlink_escape() {
    let repo = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(repo.path().join(".env"), "TOKEN=secret\n").unwrap();
    fs::write(outside.path().join("outside.md"), "outside\n").unwrap();
    create_directory_link(outside.path(), &repo.path().join("linked-outside"));
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    assert_eq!(
        executor.apply(
            FilesystemPatchRequest::replace("../escape.md", "a", "b"),
            FilesystemPatchApproval::Approved,
        ),
        FilesystemPatchOutcome::Blocked("path_escape")
    );
    assert_eq!(
        executor.apply(
            FilesystemPatchRequest::replace(".env", "TOKEN", "SAFE"),
            FilesystemPatchApproval::Approved,
        ),
        FilesystemPatchOutcome::Blocked("protected_path")
    );
    assert_eq!(
        executor.apply(
            FilesystemPatchRequest::replace("linked-outside/outside.md", "outside", "changed"),
            FilesystemPatchApproval::Approved,
        ),
        FilesystemPatchOutcome::Blocked("path_escape")
    );
}

#[test]
fn patch_blocks_sensitive_windows_style_paths_before_resolution() {
    let repo = TempDir::new().unwrap();
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    for path in [
        r"src\.git\config",
        r"config\.env.local",
        r"keys\release.pem",
    ] {
        assert_eq!(
            executor.apply(
                FilesystemPatchRequest::replace(path, "before", "after"),
                FilesystemPatchApproval::Approved,
            ),
            FilesystemPatchOutcome::Blocked("protected_path"),
            "{path} must remain protected"
        );
    }
}

#[test]
fn patch_requires_approval_before_mutating() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("notes.md"), "alpha\n").unwrap();
    let mut executor =
        FilesystemPatchExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let outcome = executor.apply(
        FilesystemPatchRequest::replace("notes.md", "alpha", "beta"),
        FilesystemPatchApproval::Pending,
    );

    assert_eq!(outcome, FilesystemPatchOutcome::ApprovalRequired);
    assert_eq!(
        fs::read_to_string(repo.path().join("notes.md")).unwrap(),
        "alpha\n"
    );
}

#[test]
fn filesystem_patch_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-tool-gateway/src/patch.rs",
            include_str!("../src/patch.rs"),
            260,
        ),
        (
            "crates/desktoplab-tool-gateway/tests/filesystem_patch.rs",
            include_str!("filesystem_patch.rs"),
            210,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("filesystem patch files should stay focused");
    }
}

#[cfg(unix)]
fn create_directory_link(target: &std::path::Path, link: &std::path::Path) {
    std::os::unix::fs::symlink(target, link).expect("directory symlink should be created");
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
