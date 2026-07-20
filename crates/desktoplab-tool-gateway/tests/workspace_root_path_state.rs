use desktoplab_tool_gateway::{WorkspacePathState, WorkspaceRoot};
use tempfile::TempDir;

#[test]
fn path_state_is_capability_safe_and_does_not_follow_links() {
    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("file.txt"), "proof").unwrap();
    std::fs::create_dir(workspace.path().join("folder")).unwrap();
    let root = WorkspaceRoot::open(workspace.path()).unwrap();

    assert_eq!(
        root.path_state("file.txt").unwrap(),
        WorkspacePathState::File
    );
    assert_eq!(
        root.path_state("folder").unwrap(),
        WorkspacePathState::Directory
    );
    assert_eq!(
        root.path_state("missing").unwrap(),
        WorkspacePathState::Missing
    );
    assert!(root.path_state("../outside").is_err());
}

#[cfg(unix)]
#[test]
fn path_state_rejects_symlink_aliases() {
    use std::os::unix::fs::symlink;

    let workspace = TempDir::new().unwrap();
    std::fs::write(workspace.path().join("target.txt"), "proof").unwrap();
    symlink("target.txt", workspace.path().join("alias.txt")).unwrap();
    let root = WorkspaceRoot::open(workspace.path()).unwrap();

    assert!(root.path_state("alias.txt").is_err());
}

#[test]
fn workspace_path_state_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/tests/workspace_root_path_state.rs",
        include_str!("workspace_root_path_state.rs"),
        80,
    )
    .expect("workspace path state tests should stay focused");
}
