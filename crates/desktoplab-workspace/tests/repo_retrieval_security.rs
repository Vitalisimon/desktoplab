use desktoplab_workspace::{
    HybridRepoRetriever, RepoCodeIndexer, RepoIndexFreshnessGuard, RepoIndexFreshnessState,
    RepoIndexLimits,
};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::tempdir;
use xtask::check_logical_line_limit;

#[test]
fn retrieval_fails_closed_after_file_change_or_delete() {
    let repo = repository();
    write(
        repo.path(),
        "src/lib.rs",
        "pub fn stable_value() -> u8 { 1 }\n",
    );
    let index = build(repo.path());
    let fresh = RepoIndexFreshnessGuard::validate(&index, repo.path());
    assert_eq!(fresh.state(), RepoIndexFreshnessState::Fresh);

    write(
        repo.path(),
        "src/lib.rs",
        "pub fn stable_value() -> u8 { 2 }\n",
    );
    let changed = RepoIndexFreshnessGuard::validate(&index, repo.path());
    let blocked = HybridRepoRetriever::new(&index).retrieve("stable_value", 3, &changed);
    assert_eq!(changed.state(), RepoIndexFreshnessState::Stale);
    assert!(
        changed
            .reasons()
            .iter()
            .any(|reason| reason.contains("changed"))
    );
    assert!(blocked.items().is_empty());

    fs::remove_file(repo.path().join("src/lib.rs")).unwrap();
    let deleted = RepoIndexFreshnessGuard::validate(&index, repo.path());
    assert!(
        deleted
            .reasons()
            .iter()
            .any(|reason| reason.contains("deleted"))
    );
}

#[test]
fn branch_change_and_repo_relink_invalidate_previous_index() {
    let repo = repository();
    write(repo.path(), "src/lib.rs", "pub fn branch_value() {}\n");
    run_git(repo.path(), &["add", "."]);
    run_git(repo.path(), &["commit", "-m", "seed"]);
    let index = build(repo.path());
    run_git(repo.path(), &["switch", "-c", "other"]);

    let branch = RepoIndexFreshnessGuard::validate(&index, repo.path());
    assert!(branch.reasons().contains(&"git_branch_changed".to_string()));

    let other = repository();
    let relink = RepoIndexFreshnessGuard::validate(&index, other.path());
    assert_eq!(relink.reasons(), &["workspace_relinked"]);
}

#[test]
fn secrets_ignored_generated_and_binary_files_never_enter_context() {
    let repo = repository();
    write(
        repo.path(),
        "src/config.rs",
        "pub fn endpoint() {}\nlet api_key=sk-live-secret;\n",
    );
    write(repo.path(), ".env", "TOKEN=env-secret\n");
    write(
        repo.path(),
        ".aws/credentials",
        "aws_secret_access_key=raw\n",
    );
    write(repo.path(), "dist/generated.rs", "pub fn generated() {}\n");
    fs::write(repo.path().join("binary.bin"), [0, 159, 146, 150]).unwrap();
    let index = build(repo.path());

    let paths: Vec<_> = index
        .documents()
        .iter()
        .map(|document| document.path())
        .collect();
    assert_eq!(paths, vec!["src/config.rs"]);
    let freshness = RepoIndexFreshnessGuard::validate(&index, repo.path());
    let report = HybridRepoRetriever::new(&index).retrieve("endpoint api_key", 3, &freshness);
    assert_eq!(report.items().len(), 1);
    assert!(report.items()[0].was_redacted());
    assert!(report.items()[0].snippet().contains("[REDACTED]"));
    assert!(!report.items()[0].snippet().contains("sk-live-secret"));
}

#[test]
fn missing_relinked_root_produces_unknown_and_zero_context() {
    let repo = repository();
    write(repo.path(), "src/lib.rs", "pub fn value() {}\n");
    let index = build(repo.path());
    let missing = repo.path().join("deleted-root");

    let freshness = RepoIndexFreshnessGuard::validate(&index, &missing);
    let report = HybridRepoRetriever::new(&index).retrieve("value", 2, &freshness);
    assert_eq!(freshness.state(), RepoIndexFreshnessState::Unknown);
    assert!(report.items().is_empty());
    assert_eq!(
        report.freshness_blocked_reasons(),
        &["workspace_root_unavailable"]
    );
}

#[test]
fn retrieval_security_sources_stay_below_line_guards() {
    check_logical_line_limit(
        "crates/desktoplab-workspace/src/retrieval_freshness.rs",
        include_str!("../src/retrieval_freshness.rs"),
        220,
    )
    .expect("retrieval freshness should stay focused");
}

fn build(root: &Path) -> desktoplab_workspace::RepoCodeIndexSnapshot {
    RepoCodeIndexer::new(RepoIndexLimits::new(100, 100_000))
        .build(root)
        .unwrap()
}

fn repository() -> tempfile::TempDir {
    let repo = tempdir().unwrap();
    run_git(repo.path(), &["init", "-b", "main"]);
    run_git(
        repo.path(),
        &["config", "user.email", "desktoplab@example.invalid"],
    );
    run_git(repo.path(), &["config", "user.name", "DesktopLab Test"]);
    repo
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn run_git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}
