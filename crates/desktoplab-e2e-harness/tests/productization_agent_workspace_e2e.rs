use desktoplab_agent_engine::{
    AgentContextBuilder, AgentPlanStore, AgentPlanner, ApprovalDecision,
    ExecutionBackendAvailability, FileEditEngine, TestFeedbackLoop,
};
use desktoplab_workspace::{
    CommitApproval, CommitOperation, MemoryVisibility, ParallelAgentRouter, RepositoryInspector,
    RollbackApproval, RollbackOperation, SavePointManager, SessionIntent, TestCommandDetector,
    WorkspaceIntelligenceApi, WorkspaceMemoryStore,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn agent_execution_productization_e2e_records_diff_test_and_replayable_context() {
    let repo = GitFixture::new();
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/lib.rs", "pub fn answer() -> i32 { 1 }\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);

    let scan = RepositoryInspector::new(64).inspect(repo.path()).unwrap();
    let tests = TestCommandDetector::detect(repo.path()).unwrap();
    let intelligence = WorkspaceIntelligenceApi::snapshot(scan, tests);
    let context = AgentContextBuilder::new(512)
        .with_workspace_fact(intelligence.summary(), "workspace intelligence")
        .build();
    let plan = AgentPlanner::new(AgentPlanStore::default()).plan(
        "session.e2e",
        context.text(),
        ExecutionBackendAvailability::Available("backend.ollama".into()),
    );
    assert_eq!(plan.status(), "planned");

    let savepoint = SavePointManager::default()
        .create(repo.path(), "session.e2e")
        .unwrap();
    let edit = FileEditEngine::new(repo.path())
        .apply(
            "src/lib.rs",
            "pub fn answer() -> i32 { 1 }\n",
            "pub fn answer() -> i32 { 2 }\n",
        )
        .unwrap();
    let feedback = TestFeedbackLoop::new(120).capture(
        "cargo test",
        Some(ApprovalDecision::Approved),
        "status=0 stdout=ok",
    );

    assert!(savepoint.ref_name().contains("session.e2e"));
    assert!(
        edit.diff_evidence()
            .contains("+pub fn answer() -> i32 { 2 }")
    );
    assert_eq!(feedback.status(), "captured");
}

#[test]
fn memory_context_e2e_reuses_approved_memory_without_provider_local_only_leak() {
    let mut store = WorkspaceMemoryStore::default();
    store.remember(
        "workspace.e2e",
        "uses cargo",
        "scan",
        MemoryVisibility::ProviderShareable,
    );
    store.remember(
        "workspace.e2e",
        "secret lives in .env",
        "protected file",
        MemoryVisibility::LocalOnly,
    );

    let context = AgentContextBuilder::new(256)
        .with_workspace_fact(store.provider_context("workspace.e2e").join("\n"), "memory")
        .build();

    assert!(context.text().contains("uses cargo"));
    assert!(!context.text().contains("secret lives in .env"));
}

#[test]
fn git_productization_e2e_savepoint_rollback_commit_and_worktree_route() {
    let repo = GitFixture::new();
    repo.write("README.md", "before\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);

    let savepoint = SavePointManager::default()
        .create(repo.path(), "session.git")
        .unwrap();
    repo.write("README.md", "after\n");
    let restored = RollbackOperation::new(RollbackApproval::Approved)
        .rollback(repo.path(), &savepoint)
        .unwrap();
    assert_eq!(restored.status(), "restored");
    assert_eq!(
        fs::read_to_string(repo.path().join("README.md")).unwrap(),
        "before\n"
    );

    repo.write("README.md", "committed\n");
    let commit = CommitOperation::new(CommitApproval::Approved)
        .commit(
            repo.path(),
            "session.git",
            "agent update",
            &["README.md".to_string()],
        )
        .unwrap();
    assert_eq!(commit.status(), "committed");

    let route = ParallelAgentRouter::default().route(
        repo.path(),
        "session.git.2",
        SessionIntent::WriteCapable,
    );
    assert_eq!(
        route.isolation_reason(),
        "write_capable_parallel_requires_worktree"
    );
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
}
