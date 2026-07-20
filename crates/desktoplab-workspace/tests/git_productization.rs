use desktoplab_workspace::{
    CommitApproval, CommitOperation, ParallelAgentRouter, ProductWorktreeManager, PushApproval,
    PushOperation, RollbackApproval, RollbackOperation, SavePointManager, SessionIntent,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn savepoint_captures_dirty_worktree_without_mutating_it() {
    let repo = GitFixture::new();
    repo.write("README.md", "ready\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);

    let manager = SavePointManager::default();
    let savepoint = manager.create(repo.path(), "session.1").unwrap();
    assert_eq!(savepoint.session_id(), "session.1");
    assert!(savepoint.ref_name().starts_with("desktoplab/savepoints/"));

    repo.write("README.md", "changed\n");
    repo.write("dirty.txt", "dirty\n");
    let status_before = repo.git_stdout(&["status", "--porcelain"]);
    let index_before = repo.git_stdout(&["write-tree"]);

    let dirty = manager.create(repo.path(), "session.2").unwrap();

    assert_eq!(repo.git_stdout(&["status", "--porcelain"]), status_before);
    assert_eq!(repo.git_stdout(&["write-tree"]), index_before);
    assert_eq!(
        repo.git_stdout(&["show", &format!("refs/{}:README.md", dirty.ref_name())]),
        "changed\n"
    );
    assert_eq!(
        repo.git_stdout(&["show", &format!("refs/{}:dirty.txt", dirty.ref_name())]),
        "dirty\n"
    );
}

#[test]
fn savepoint_manager_lists_persisted_desktoplab_refs() {
    let repo = GitFixture::new();
    repo.write("README.md", "ready\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);
    let manager = SavePointManager::default();
    manager.create(repo.path(), "session.1").unwrap();
    manager.create(repo.path(), "session.2").unwrap();

    let savepoints = manager.list(repo.path()).unwrap();

    assert_eq!(
        savepoints
            .iter()
            .map(|savepoint| savepoint.session_id())
            .collect::<Vec<_>>(),
        vec!["session.1", "session.2"]
    );
}

#[test]
fn rollback_requires_approval_and_restores_fixture_repo() {
    let repo = GitFixture::new();
    repo.write("README.md", "before\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);
    let savepoint = SavePointManager::default()
        .create(repo.path(), "session.rollback")
        .unwrap();
    repo.write("README.md", "after\n");
    repo.write("notes.local", "keep me\n");

    let preview = RollbackOperation::new(RollbackApproval::Denied)
        .preview(repo.path(), &savepoint)
        .unwrap();
    assert_eq!(preview.changed_files(), &["README.md".to_string()]);
    assert_eq!(
        preview.protected_untracked_files(),
        &["notes.local".to_string()]
    );

    let denied = RollbackOperation::new(RollbackApproval::Denied)
        .rollback(repo.path(), &savepoint)
        .unwrap();
    assert_eq!(denied.status(), "denied");
    assert_eq!(
        fs::read_to_string(repo.path().join("README.md")).unwrap(),
        "after\n"
    );

    let restored = RollbackOperation::new(RollbackApproval::Approved)
        .rollback(repo.path(), &savepoint)
        .unwrap();
    assert_eq!(restored.status(), "restored");
    assert_eq!(
        fs::read_to_string(repo.path().join("README.md")).unwrap(),
        "before\n"
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("notes.local")).unwrap(),
        "keep me\n"
    );
}

#[test]
fn commit_and_push_operations_are_approval_gated() {
    let repo = GitFixture::new();
    repo.write("README.md", "initial\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);
    repo.write("README.md", "changed\n");
    repo.write("EXTRA.md", "staged separately\n");
    repo.git(&["add", "EXTRA.md"]);

    let changed_files = vec!["README.md".to_string()];
    let denied = CommitOperation::new(CommitApproval::Denied)
        .commit(
            repo.path(),
            "session.commit",
            "agent change",
            &changed_files,
        )
        .unwrap();
    assert_eq!(denied.status(), "denied");
    assert_eq!(
        repo.git_stdout(&["rev-list", "--count", "HEAD"]).trim(),
        "1"
    );

    let committed = CommitOperation::new(CommitApproval::Approved)
        .commit(
            repo.path(),
            "session.commit",
            "agent change",
            &changed_files,
        )
        .unwrap();
    assert_eq!(committed.status(), "committed");
    assert!(committed.message().contains("session.commit"));
    assert_eq!(
        repo.git_stdout(&["show", "--format=", "--name-only", "HEAD"])
            .trim(),
        "README.md"
    );
    assert!(
        repo.git_stdout(&["status", "--porcelain"])
            .contains("A  EXTRA.md")
    );

    let push = PushOperation::new(PushApproval::Denied)
        .push(repo.path(), "origin", "main")
        .unwrap();
    assert_eq!(push.status(), "denied");
    assert!(!push.had_network_side_effect());
}

#[test]
fn worktree_manager_routes_parallel_write_sessions_to_isolated_worktrees() {
    let repo = GitFixture::new();
    repo.write("README.md", "initial\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);
    let manager = ProductWorktreeManager::default();

    let routed = ParallelAgentRouter::new(manager).route(
        repo.path(),
        "session.write",
        SessionIntent::WriteCapable,
    );

    assert_eq!(
        routed.isolation_reason(),
        "write_capable_parallel_requires_worktree"
    );
    assert!(routed.worktree_path().is_some());

    let read_only =
        ParallelAgentRouter::default().route(repo.path(), "session.read", SessionIntent::ReadOnly);
    assert!(read_only.can_share_workspace());
}

#[test]
fn worktree_manager_cleans_up_only_desktoplab_owned_worktrees() {
    let repo = GitFixture::new();
    repo.write("README.md", "ready\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);
    let manager = ProductWorktreeManager::default();
    let route = manager.create(repo.path(), "session.cleanup").unwrap();
    let worktree_path = route.worktree_path().unwrap().to_path_buf();
    assert!(worktree_path.exists());

    let cleaned = manager.cleanup(repo.path(), "session.cleanup").unwrap();

    assert_eq!(cleaned.status(), "cleaned");
    assert!(!worktree_path.exists());
    assert!(manager.cleanup(repo.path(), "manual.worktree").is_err());
}

struct GitFixture {
    temp_dir: TempDir,
}

impl GitFixture {
    fn new() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let fixture = Self { temp_dir };
        fixture.git(&["init", "-b", "main"]);
        fixture
    }

    fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    fn write(&self, relative: &str, contents: &str) {
        let path = self.path().join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args([
                "-c",
                "user.name=DesktopLab",
                "-c",
                "user.email=desktoplab@example.invalid",
            ])
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8_lossy(&output.stdout).to_string()
    }
}
