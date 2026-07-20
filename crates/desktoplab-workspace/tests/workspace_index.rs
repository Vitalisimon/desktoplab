use std::fs;

use desktoplab_workspace::{
    WorkspaceFileSafety, WorkspaceIndex, WorkspaceIndexLimits, WorkspaceSearch,
    WorkspaceSearchLimits,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn workspace_index_excludes_ignored_generated_binary_and_secret_surfaces() {
    let repo = index_fixture();
    let index = WorkspaceIndex::new(WorkspaceIndexLimits::new(32, 4096))
        .build(repo.path())
        .unwrap();
    let paths = index
        .entries()
        .iter()
        .map(|entry| entry.path())
        .collect::<Vec<_>>();

    assert!(paths.contains(&"src/lib.rs"));
    assert!(paths.contains(&".env"));
    assert!(!paths.iter().any(|path| path.starts_with(".git/")));
    assert!(!paths.iter().any(|path| path.contains("node_modules")));
    assert!(!paths.iter().any(|path| path.starts_with("ignored/")));
    assert!(!paths.iter().any(|path| path == &"image.bin"));
    assert!(
        index
            .skipped()
            .iter()
            .any(|reason| reason.contains("ignored:ignored"))
    );

    let env = index
        .entries()
        .iter()
        .find(|entry| entry.path() == ".env")
        .unwrap();
    assert_eq!(env.safety(), WorkspaceFileSafety::Protected);
    assert!(!env.text_preview_eligible());
}

#[test]
fn workspace_index_records_language_size_modified_time_and_preview_eligibility() {
    let repo = index_fixture();
    let index = WorkspaceIndex::new(WorkspaceIndexLimits::new(32, 4096))
        .build(repo.path())
        .unwrap();
    let source = index
        .entries()
        .iter()
        .find(|entry| entry.path() == "src/lib.rs")
        .unwrap();

    assert_eq!(source.language(), Some("rust"));
    assert!(source.size_bytes() > 0);
    assert!(source.modified_unix_secs().is_some());
    assert!(source.text_preview_eligible());
}

#[test]
fn workspace_index_truncates_with_explicit_evidence() {
    let repo = TempDir::new().unwrap();
    for index in 0..10 {
        fs::write(repo.path().join(format!("file-{index}.txt")), "hello").unwrap();
    }
    let snapshot = WorkspaceIndex::new(WorkspaceIndexLimits::new(4, 4096))
        .build(repo.path())
        .unwrap();

    assert!(snapshot.truncated());
    assert_eq!(snapshot.entries().len(), 4);
}

#[test]
fn workspace_search_uses_index_ignore_rules() {
    let repo = index_fixture();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(32, 16, 4096));

    let report = search.search(repo.path(), "ignored needle").unwrap();

    assert!(report.matches().is_empty());
}

#[test]
fn workspace_index_files_stay_small() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-workspace/src/index.rs",
            include_str!("../src/index.rs"),
            260,
        ),
        (
            "crates/desktoplab-workspace/tests/workspace_index.rs",
            include_str!("workspace_index.rs"),
            150,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("workspace index files should stay focused");
    }
}

fn index_fixture() -> TempDir {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join(".git")).unwrap();
    fs::create_dir_all(repo.path().join("node_modules/pkg")).unwrap();
    fs::create_dir_all(repo.path().join("ignored")).unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(repo.path().join(".gitignore"), "ignored/\n*.tmp\n").unwrap();
    fs::write(repo.path().join(".git/config"), "ignored needle").unwrap();
    fs::write(
        repo.path().join("node_modules/pkg/index.js"),
        "ignored needle",
    )
    .unwrap();
    fs::write(repo.path().join("ignored/file.txt"), "ignored needle").unwrap();
    fs::write(repo.path().join("ignored.tmp"), "ignored needle").unwrap();
    fs::write(repo.path().join(".env"), "OPENAI_API_KEY=sk-secret\n").unwrap();
    fs::write(repo.path().join("src/lib.rs"), "pub fn indexed() {}\n").unwrap();
    fs::write(repo.path().join("image.bin"), [0, 159, 146, 150]).unwrap();
    repo
}
