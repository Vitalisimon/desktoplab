use desktoplab_domain::WorkspaceId;
use desktoplab_workspace::{
    CheckpointStatus, GitRepository, IsolationDecision, ParallelExecutionKind,
    WorkspaceRegistration, WorkspaceRegistry, WorktreePolicy,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn workspace_registration_keeps_repository_path_and_identity() {
    let repo = TestRepo::init();
    let registration = WorkspaceRegistration::new(
        WorkspaceId::new("workspace.test"),
        repo.path().to_path_buf(),
    );

    assert_eq!(registration.workspace_id().as_str(), "workspace.test");
    assert_eq!(registration.root_path(), repo.path());
}

#[test]
fn workspace_registry_returns_registered_workspace_by_id() {
    let repo = TestRepo::init();
    let registration = WorkspaceRegistration::new(
        WorkspaceId::new("workspace.registered"),
        repo.path().to_path_buf(),
    );
    let mut registry = WorkspaceRegistry::default();

    registry.register(registration.clone());

    assert_eq!(
        registry
            .get(&WorkspaceId::new("workspace.registered"))
            .expect("workspace should be registered"),
        &registration
    );
}

#[test]
fn git_repository_identity_uses_real_git_root() {
    let repo = TestRepo::init();
    let nested = repo.path().join("src");
    fs::create_dir_all(&nested).unwrap();
    let git = GitRepository::open(&nested).expect("nested path should resolve repository");
    let identity = git.identity();

    assert_eq!(identity.root_path(), repo.path().canonicalize().unwrap());
    assert!(identity.git_dir_path().ends_with(".git"));
}

#[test]
fn git_status_and_diff_are_read_from_repository() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("main.rs"), "fn main() {}\n").unwrap();
    let git = GitRepository::open(repo.path()).expect("repo should open");

    let status = git.status().expect("status should read");
    let diff = git.diff().expect("diff should read");

    assert!(status.is_dirty());
    assert!(
        status
            .entries()
            .iter()
            .any(|entry| entry.contains("main.rs"))
    );
    assert!(diff.as_text().contains("fn main()"));
}

#[test]
fn git_diff_combines_staged_unstaged_and_untracked_evidence() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("unstaged.txt"), "before\n").unwrap();
    run_git(repo.path(), &["add", "."]);
    run_git(
        repo.path(),
        &[
            "-c",
            "user.email=desktoplab@example.invalid",
            "-c",
            "user.name=DesktopLab Test",
            "commit",
            "-m",
            "seed",
        ],
    );
    fs::write(repo.path().join("unstaged.txt"), "unstaged evidence\n").unwrap();
    fs::write(repo.path().join("staged.txt"), "staged evidence\n").unwrap();
    run_git(repo.path(), &["add", "staged.txt"]);
    fs::write(repo.path().join("untracked.txt"), "untracked evidence\n").unwrap();
    fs::write(repo.path().join("binary.dat"), [0, 159, 146, 150]).unwrap();
    let git = GitRepository::open(repo.path()).unwrap();

    let combined = git.diff().unwrap();
    assert!(combined.as_text().contains("unstaged evidence"));
    assert!(combined.as_text().contains("staged evidence"));
    assert!(combined.as_text().contains("untracked evidence"));
    assert!(combined.as_text().contains("+++ binary.dat"));
    assert!(combined.as_text().contains("sha256:"));
    assert!(
        git.diff_path("staged.txt")
            .unwrap()
            .as_text()
            .contains("staged evidence")
    );
}

#[test]
fn git_status_preserves_renamed_paths_with_spaces() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("old name.md"), "content\n").unwrap();
    run_git(repo.path(), &["add", "."]);
    run_git(
        repo.path(),
        &[
            "-c",
            "user.email=desktoplab@example.invalid",
            "-c",
            "user.name=DesktopLab Test",
            "commit",
            "-m",
            "seed",
        ],
    );
    run_git(repo.path(), &["mv", "old name.md", "new name.md"]);

    let status = GitRepository::open(repo.path()).unwrap().status().unwrap();
    let renamed = status
        .files()
        .iter()
        .find(|file| file.code().contains('R'))
        .expect("rename should retain structured status");

    assert_eq!(renamed.path(), "new name.md");
}

#[test]
fn checkpoint_can_continue_when_worktree_is_dirty() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("dirty.txt"), "dirty\n").unwrap();
    let git = GitRepository::open(repo.path()).expect("repo should open");

    let checkpoint = git
        .prepare_checkpoint()
        .expect("checkpoint should evaluate");

    assert_eq!(checkpoint.status(), CheckpointStatus::Ready);
    assert!(checkpoint.can_continue_with_risky_execution());
}

#[test]
fn checkpoint_can_continue_when_worktree_is_clean() {
    let repo = TestRepo::init();
    let git = GitRepository::open(repo.path()).expect("repo should open");

    let checkpoint = git
        .prepare_checkpoint()
        .expect("checkpoint should evaluate");

    assert_eq!(checkpoint.status(), CheckpointStatus::Ready);
    assert!(checkpoint.can_continue_with_risky_execution());
}

#[test]
fn worktree_policy_requires_isolation_for_parallel_writes() {
    let policy = WorktreePolicy::strict();

    assert_eq!(
        policy.evaluate(ParallelExecutionKind::WriteCapableParallel),
        IsolationDecision::RequiresWorktree
    );
    assert_eq!(
        policy.evaluate(ParallelExecutionKind::ReadOnlyParallel),
        IsolationDecision::CanShareWorkspace
    );
}

#[test]
fn workspace_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-workspace/src/lib.rs",
        include_str!("../src/lib.rs"),
        250,
    )
    .expect("workspace lib should stay below the initial line-count guard");
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
