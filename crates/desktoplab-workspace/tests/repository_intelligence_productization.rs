use desktoplab_workspace::{
    ContextRefreshScheduler, MemoryVisibility, RepositoryInspector, TestCommandDetector,
    WorkspaceIntelligenceApi, WorkspaceMemoryStore, WorkspacePolicyClassifier,
};
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn repository_inspection_excludes_protected_files_and_detects_package_facts() {
    let repo = FixtureRepo::new();
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.write(".env", "OPENAI_API_KEY=secret\n");
    repo.write(".ssh/id_rsa", "private-key\n");

    let report = RepositoryInspector::new(64).inspect(repo.path()).unwrap();

    assert!(report.has_language("rust"));
    assert!(report.has_package_manager("cargo"));
    assert!(report.protected_files_excluded());
    assert!(!report.summary_text().contains("OPENAI_API_KEY"));
    assert!(!report.summary_text().contains("private-key"));
}

#[test]
fn repository_inspection_excludes_generated_artifacts_from_agent_context() {
    let repo = FixtureRepo::new();
    repo.write("README.md", "# Demo\n");
    repo.write("src/main.py", "print('hello')\n");
    repo.write(".DS_Store", "finder metadata\n");
    repo.write("build/app/PYZ-00.pyz", "generated archive\n");
    repo.write("dist/app/_internal/native.dylib", "binary artifact\n");
    repo.write("node_modules/pkg/index.js", "vendor code\n");
    repo.write("target/debug/app", "compiled binary\n");

    let report = RepositoryInspector::new(64).inspect(repo.path()).unwrap();
    let summary = report.summary_text();

    assert!(summary.contains("README.md"));
    assert!(summary.contains("src/main.py"));
    assert!(!summary.contains(".DS_Store"));
    assert!(!summary.contains("build/"));
    assert!(!summary.contains("dist/"));
    assert!(!summary.contains("node_modules/"));
    assert!(!summary.contains("target/"));
}

#[test]
fn repository_inspection_degrades_when_file_limit_is_exceeded() {
    let repo = FixtureRepo::new();
    for index in 0..8 {
        repo.write(&format!("src/file_{index}.rs"), "fn main() {}\n");
    }

    let report = RepositoryInspector::new(3).inspect(repo.path()).unwrap();

    assert!(report.is_degraded());
    assert_eq!(
        report.degraded_reason(),
        Some("workspace_scan_file_limit_exceeded")
    );
}

#[test]
fn test_commands_are_detected_from_fixture_files_without_execution() {
    let repo = FixtureRepo::new();
    repo.write(
        "package.json",
        r#"{"scripts":{"test":"vitest run","lint":"eslint ."}}"#,
    );
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write("pyproject.toml", "[tool.pytest.ini_options]\n");
    repo.write("go.mod", "module example.test/demo\n");
    repo.write("Package.swift", "// swift-tools-version: 6.0\n");

    let commands = TestCommandDetector::detect(repo.path()).unwrap();

    assert!(commands.has_high_confidence("npm test"));
    assert!(commands.has_high_confidence("cargo test"));
    assert!(commands.has_low_confidence("pytest"));
    assert!(commands.has_high_confidence("go test ./..."));
    assert!(commands.has_high_confidence("swift test"));
    assert!(commands.requires_confirmation("pytest"));
    assert!(!commands.executed_any_command());
}

#[test]
fn workspace_policy_marks_local_only_files_and_filters_provider_context() {
    let classifier = WorkspacePolicyClassifier::default();
    let classified =
        classifier.classify_paths([".git/config", ".env", ".ssh/id_rsa", "src/lib.rs"]);

    assert!(classified.is_local_only(".git/config"));
    assert!(classified.is_local_only(".env"));
    assert!(classified.is_local_only(".ssh/id_rsa"));
    assert!(classified.is_shareable("src/lib.rs"));

    let provider_context = classified.provider_context_paths();
    assert_eq!(provider_context, vec!["src/lib.rs"]);
    assert!(
        classified
            .override_for_provider(".env", "user approved diagnostics")
            .is_audited()
    );
}

#[test]
fn workspace_intelligence_api_uses_scan_output_and_marks_stale_facts() {
    let repo = FixtureRepo::new();
    repo.write("Cargo.toml", "[package]\nname = \"demo\"\n");
    repo.write(".env", "SECRET=value\n");

    let scan = RepositoryInspector::new(32).inspect(repo.path()).unwrap();
    let commands = TestCommandDetector::detect(repo.path()).unwrap();
    let snapshot = WorkspaceIntelligenceApi::snapshot(scan, commands).mark_stale();

    assert!(snapshot.has_language("rust"));
    assert!(snapshot.has_test_command("cargo test"));
    assert!(snapshot.protected_file_summary().contains("[REDACTED]"));
    assert!(snapshot.is_stale());
}

#[test]
fn memory_store_keeps_provenance_and_excludes_local_only_memory_from_providers() {
    let mut store = WorkspaceMemoryStore::default();

    let public = store.remember(
        "workspace.demo",
        "uses cargo",
        "repository scan",
        MemoryVisibility::ProviderShareable,
    );
    let local = store.remember(
        "workspace.demo",
        "secret is in .env",
        "protected file",
        MemoryVisibility::LocalOnly,
    );

    assert_eq!(store.get(public).unwrap().provenance(), "repository scan");
    assert_eq!(store.provider_context("workspace.demo"), vec!["uses cargo"]);
    assert!(store.delete(local));
    assert!(!store.export("workspace.demo").contains("secret is in .env"));
}

#[test]
fn context_refresh_scheduler_is_bounded_and_degrades_on_failure() {
    let scheduler = ContextRefreshScheduler::new(2);

    let report = scheduler.refresh(["workspace.a", "workspace.b", "workspace.c"], true);
    assert_eq!(report.refreshed_count(), 2);
    assert!(report.is_bounded());

    let failed = scheduler.refresh(["workspace.a"], false);
    assert!(failed.diagnostics().contains("refresh_failed"));
}

#[test]
fn workspace_productization_sources_stay_below_line_count_guards() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-workspace/src/inspection.rs",
            include_str!("../src/inspection.rs"),
            280,
        ),
        (
            "crates/desktoplab-workspace/src/memory.rs",
            include_str!("../src/memory.rs"),
            260,
        ),
        (
            "crates/desktoplab-workspace/src/product_git.rs",
            include_str!("../src/product_git.rs"),
            180,
        ),
        (
            "crates/desktoplab-workspace/src/product_git/savepoint.rs",
            include_str!("../src/product_git/savepoint.rs"),
            220,
        ),
        (
            "crates/desktoplab-workspace/src/product_git/savepoint/snapshot.rs",
            include_str!("../src/product_git/savepoint/snapshot.rs"),
            180,
        ),
        (
            "crates/desktoplab-workspace/src/product_git/commit_push.rs",
            include_str!("../src/product_git/commit_push.rs"),
            120,
        ),
        (
            "crates/desktoplab-workspace/src/product_git/worktree.rs",
            include_str!("../src/product_git/worktree.rs"),
            220,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("workspace productization modules should stay focused");
    }
}

struct FixtureRepo {
    temp_dir: TempDir,
}

impl FixtureRepo {
    fn new() -> Self {
        Self {
            temp_dir: TempDir::new().unwrap(),
        }
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
}
