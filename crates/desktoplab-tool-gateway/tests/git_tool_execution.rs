use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{GitToolExecutor, GitToolOutcome, ParallelGitExecution};
use desktoplab_workspace::IsolationDecision;
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn status_and_diff_can_run_read_only() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("main.rs"), "fn main() {}\n").unwrap();
    let mut executor = GitToolExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let GitToolOutcome::Status(status) = executor.status() else {
        panic!("status should return git status");
    };
    let GitToolOutcome::Diff(diff) = executor.diff() else {
        panic!("diff should return git diff");
    };

    assert!(
        status
            .entries()
            .iter()
            .any(|entry| entry.contains("main.rs"))
    );
    assert!(diff.as_text().contains("fn main()"));
}

#[test]
fn status_observation_names_every_tracked_and_untracked_change() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("README.md"), "before\n").unwrap();
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "initial"]);
    fs::write(repo.path().join("README.md"), "after\n").unwrap();
    fs::write(repo.path().join("new.md"), "new\n").unwrap();
    let mut executor = GitToolExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let observation = executor.status_observation().unwrap();

    assert!(
        observation.contains("- modified: README.md"),
        "{observation}"
    );
    assert!(observation.contains("- untracked: new.md"), "{observation}");
    assert!(!observation.contains("?? new.md"), "{observation}");
}

#[test]
fn write_capable_parallel_git_execution_requires_worktree_isolation() {
    let repo = TestRepo::init();
    let executor = GitToolExecutor::new(repo.path(), PolicyEngine::default_conservative());

    assert_eq!(
        executor.parallel_execution_policy(ParallelGitExecution::WriteCapable),
        IsolationDecision::RequiresWorktree
    );
    assert_eq!(
        executor.parallel_execution_policy(ParallelGitExecution::ReadOnly),
        IsolationDecision::CanShareWorkspace
    );
}

#[test]
fn checkpoint_reference_snapshots_dirty_workspace() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("README.md"), "before\n").unwrap();
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "initial"]);
    fs::write(repo.path().join("README.md"), "after\n").unwrap();
    fs::write(repo.path().join("dirty.txt"), "dirty\n").unwrap();
    let mut executor = GitToolExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let outcome = executor.prepare_checkpoint_ref("checkpoint/test");

    let GitToolOutcome::CheckpointReady(reference) = outcome else {
        panic!("dirty workspace should produce a real checkpoint");
    };
    assert!(reference.starts_with("desktoplab/savepoints/"));
    assert_eq!(
        git_stdout(
            repo.path(),
            &["show", &format!("refs/{reference}:README.md")]
        ),
        "after\n"
    );
    assert_eq!(
        git_stdout(
            repo.path(),
            &["show", &format!("refs/{reference}:dirty.txt")]
        ),
        "dirty\n"
    );
    assert!(repo.path().join("dirty.txt").exists());
}

#[test]
fn rollback_preview_separates_tracked_changes_from_protected_untracked_files() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("README.md"), "before\n").unwrap();
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "initial"]);
    fs::write(repo.path().join("README.md"), "after\n").unwrap();
    fs::write(repo.path().join("scratch.md"), "local\n").unwrap();
    let mut executor = GitToolExecutor::new(repo.path(), PolicyEngine::default_conservative());

    let outcome = executor.rollback_preview();

    assert_eq!(
        outcome,
        GitToolOutcome::RollbackPreview {
            changed_files: vec!["README.md".to_string()],
            protected_untracked_files: vec!["scratch.md".to_string()]
        }
    );
}

#[test]
fn git_tool_execution_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/git.rs",
        include_str!("../src/git.rs"),
        300,
    )
    .expect("git tool execution source should stay below the line-count guard");
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
        .args([
            "-c",
            "user.name=DesktopLab",
            "-c",
            "user.email=desktoplab@example.invalid",
        ])
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

fn git_stdout(cwd: &Path, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).unwrap()
}
