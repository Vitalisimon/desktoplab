use desktoplab_domain::WorkspaceId;
use desktoplab_workspace::{
    CheckpointStatus, WorkspaceApiErrorCode, WorkspaceApiService, WorkspaceApiState,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn opening_non_git_folder_returns_structured_error() {
    let temp_dir = TempDir::new().unwrap();
    let mut service = WorkspaceApiService::default();

    let error = service
        .open_existing(WorkspaceId::new("workspace.invalid"), temp_dir.path())
        .expect_err("non git folder must not open as repository");

    assert_eq!(error.code(), WorkspaceApiErrorCode::NotGitRepository);
    assert!(error.message().contains("not a git repository"));
}

#[test]
fn git_status_and_diff_are_real_backend_state() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("main.rs"), "fn main() {}\n").unwrap();
    let mut service = WorkspaceApiService::default();

    let state = service
        .open_existing(WorkspaceId::new("workspace.repo"), repo.path())
        .expect("repo should open");

    assert_eq!(state.status_entries().len(), 1);
    assert!(state.diff_text().contains("fn main()"));
    assert_eq!(state.api_state(), WorkspaceApiState::Dirty);
}

#[test]
fn dirty_workspace_supports_non_destructive_checkpoint() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("dirty.txt"), "dirty\n").unwrap();
    let mut service = WorkspaceApiService::default();

    let state = service
        .open_existing(WorkspaceId::new("workspace.dirty"), repo.path())
        .expect("repo should open");

    assert_eq!(state.checkpoint_status(), CheckpointStatus::Ready);
    assert!(state.can_checkpoint_risky_execution());
}

#[test]
fn create_repository_registers_workspace_and_git_identity() {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().join("created");
    let mut service = WorkspaceApiService::default();

    let state = service
        .create_repository(WorkspaceId::new("workspace.created"), &repo_path)
        .expect("repository should be created");

    assert_eq!(state.workspace_id().as_str(), "workspace.created");
    assert!(state.root_path().ends_with("created"));
    assert!(state.git_dir_path().ends_with(".git"));
    assert_eq!(state.api_state(), WorkspaceApiState::Clean);
    assert!(
        service
            .get(&WorkspaceId::new("workspace.created"))
            .is_some()
    );
}

#[test]
fn workspace_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-workspace/src/api.rs",
        include_str!("../src/api.rs"),
        280,
    )
    .expect("workspace api source should stay below the line-count guard");
}

struct TestRepo {
    temp_dir: TempDir,
}

impl TestRepo {
    fn init() -> Self {
        let temp_dir = TempDir::new().unwrap();
        run_git(temp_dir.path(), &["init"]);
        Self { temp_dir }
    }

    fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

fn run_git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git command should run");

    assert!(
        output.status.success(),
        "git command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
