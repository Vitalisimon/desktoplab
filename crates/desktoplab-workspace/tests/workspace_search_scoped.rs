use desktoplab_workspace::{WorkspaceSearch, WorkspaceSearchLimits};
use std::fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn scoped_search_sources_stay_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-workspace/src/search/scoped.rs",
            include_str!("../src/search/scoped.rs"),
            100,
        ),
        (
            "crates/desktoplab-workspace/tests/workspace_search_scoped.rs",
            include_str!("workspace_search_scoped.rs"),
            120,
        ),
    ] {
        check_logical_line_limit(path, source, limit).unwrap();
    }
}

#[test]
fn file_scoped_search_respects_safety_even_with_a_small_index_limit() {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("docs")).unwrap();
    fs::write(repo.path().join("docs/guide.md"), "line one\nNeedle\n").unwrap();
    fs::write(repo.path().join("docs/other.md"), "Needle\n").unwrap();
    fs::write(repo.path().join(".env"), "Needle=private\n").unwrap();
    fs::write(repo.path().join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(repo.path().join("ignored.txt"), "Needle\n").unwrap();
    fs::write(repo.path().join("binary.bin"), b"Needle\0private").unwrap();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(1, 16, 4096));
    let found = search
        .search_file_with_options(repo.path(), "docs/guide.md", "^needle$", true, false)
        .unwrap();
    assert_eq!(found.matches().len(), 1);
    assert_eq!(found.matches()[0].path(), "docs/guide.md");
    assert_eq!(found.matches()[0].line_number(), Some(2));
    for path in [".env", "ignored.txt", "binary.bin"] {
        assert!(
            search
                .search_file_with_options(repo.path(), path, "Needle", false, true)
                .unwrap()
                .matches()
                .is_empty()
        );
    }
    assert!(
        search
            .search_file_with_options(repo.path(), "../outside", "Needle", false, true)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn file_scoped_search_does_not_follow_links_into_protected_or_external_files() {
    use std::os::unix::fs::symlink;
    let repo = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(repo.path().join(".env"), "Needle secret").unwrap();
    fs::write(outside.path().join("private.txt"), "Needle external").unwrap();
    symlink(repo.path().join(".env"), repo.path().join("alias.txt")).unwrap();
    symlink(
        outside.path().join("private.txt"),
        repo.path().join("outside.txt"),
    )
    .unwrap();
    let search = WorkspaceSearch::new(WorkspaceSearchLimits::new(1, 16, 4096));
    assert!(
        search
            .search_file_with_options(repo.path(), "alias.txt", "Needle", false, true)
            .unwrap()
            .matches()
            .is_empty()
    );
    assert!(
        search
            .search_file_with_options(repo.path(), "outside.txt", "Needle", false, true)
            .is_err()
    );
}
