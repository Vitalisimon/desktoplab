use desktoplab_workspace::{
    FileTreeEntry, FileTreeEntryKind, FileTreeProtection, WorkspaceFileTree,
    WorkspaceFileTreeLimits,
};
use std::fs;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn file_tree_contract_represents_safe_repository_entries_without_contents() {
    let tree = WorkspaceFileTree::new(
        "workspace.demo",
        vec![
            FileTreeEntry::directory("src"),
            FileTreeEntry::file("src/main.rs"),
            FileTreeEntry::symlink("docs/current"),
            FileTreeEntry::hidden_file(".gitignore"),
        ],
        WorkspaceFileTreeLimits::new(4, 100),
    );

    assert_eq!(tree.workspace_id(), "workspace.demo");
    assert_eq!(tree.entries()[0].kind(), FileTreeEntryKind::Directory);
    assert_eq!(tree.entries()[1].kind(), FileTreeEntryKind::File);
    assert_eq!(tree.entries()[2].kind(), FileTreeEntryKind::Symlink);
    assert_eq!(tree.entries()[3].kind(), FileTreeEntryKind::HiddenFile);
    assert!(
        tree.entries()
            .iter()
            .all(|entry| entry.preview_text().is_none())
    );
}

#[test]
fn file_tree_contract_marks_local_only_paths_as_protected() {
    let tree = WorkspaceFileTree::new(
        "workspace.secure",
        vec![
            FileTreeEntry::file(".env"),
            FileTreeEntry::file(".git/config"),
            FileTreeEntry::file(".ssh/id_rsa"),
            FileTreeEntry::file("src/app.ts"),
        ],
        WorkspaceFileTreeLimits::new(4, 100),
    );

    let protections: Vec<_> = tree
        .entries()
        .iter()
        .map(|entry| (entry.path(), entry.protection()))
        .collect();

    assert_eq!(
        protections,
        vec![
            (".env", FileTreeProtection::Protected),
            (".git/config", FileTreeProtection::Protected),
            (".ssh/id_rsa", FileTreeProtection::Protected),
            ("src/app.ts", FileTreeProtection::Readable),
        ]
    );
}

#[test]
fn file_tree_contract_carries_degraded_reasons_for_large_repositories() {
    let tree = WorkspaceFileTree::new(
        "workspace.large",
        vec![
            FileTreeEntry::file("a.rs"),
            FileTreeEntry::file("b.rs"),
            FileTreeEntry::file("c.rs"),
        ],
        WorkspaceFileTreeLimits::new(2, 100),
    );

    assert!(tree.is_degraded());
    assert_eq!(tree.entries().len(), 2);
    assert_eq!(
        tree.degraded_reasons(),
        &["workspace_file_tree_entry_limit_exceeded"]
    );
}

#[test]
fn workspace_file_tree_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-workspace/src/file_tree.rs",
        include_str!("../src/file_tree.rs"),
        260,
    )
    .expect("workspace file tree source should stay below the line-count guard");
}

#[test]
fn scanner_reads_bounded_tree_inside_workspace_root() {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(repo.path().join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(repo.path().join(".env"), "SECRET=value\n").unwrap();

    let tree = WorkspaceFileTree::scan(
        "workspace.scan",
        repo.path(),
        WorkspaceFileTreeLimits::new(20, 4),
    )
    .expect("tree should scan");

    assert!(entry(&tree, "src").is_some_and(|entry| entry.kind() == FileTreeEntryKind::Directory));
    assert!(
        entry(&tree, "src/main.rs")
            .is_some_and(|entry| entry.protection() == FileTreeProtection::Readable)
    );
    assert!(
        entry(&tree, ".env")
            .is_some_and(|entry| entry.protection() == FileTreeProtection::Protected)
    );
}

#[test]
fn scanner_skips_regenerable_and_local_reference_trees_before_applying_limits() {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join(".external-references/openclaw")).unwrap();
    fs::create_dir_all(repo.path().join("target/debug")).unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    for index in 0..20 {
        fs::write(
            repo.path()
                .join(format!(".external-references/openclaw/{index}.rs")),
            "reference\n",
        )
        .unwrap();
    }
    fs::write(repo.path().join("target/debug/cache"), "generated\n").unwrap();
    fs::write(repo.path().join(".DS_Store"), "metadata\n").unwrap();
    fs::write(repo.path().join("src/lib.rs"), "pub mod app;\n").unwrap();

    let tree = WorkspaceFileTree::scan(
        "workspace.filtered",
        repo.path(),
        WorkspaceFileTreeLimits::new(4, 4),
    )
    .expect("tree should scan filtered repository paths");

    assert!(entry(&tree, "src").is_some());
    assert!(entry(&tree, "src/lib.rs").is_some());
    assert!(entry(&tree, ".external-references").is_none());
    assert!(entry(&tree, "target").is_none());
    assert!(entry(&tree, ".DS_Store").is_none());
    assert!(!tree.is_degraded());
}

#[test]
fn scanner_degrades_when_depth_or_entry_limits_are_exceeded() {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("a/b")).unwrap();
    fs::write(repo.path().join("a/b/c.rs"), "mod c;\n").unwrap();
    fs::write(repo.path().join("root.rs"), "mod root;\n").unwrap();

    let tree = WorkspaceFileTree::scan(
        "workspace.limited",
        repo.path(),
        WorkspaceFileTreeLimits::new(1, 1),
    )
    .expect("tree should scan with degraded limits");

    assert!(tree.is_degraded());
    assert_eq!(tree.entries().len(), 1);
    assert!(
        tree.degraded_reasons()
            .contains(&"workspace_file_tree_entry_limit_exceeded")
    );
    assert!(
        tree.degraded_reasons()
            .contains(&"workspace_file_tree_depth_limit_exceeded")
    );
}

#[cfg(unix)]
#[test]
fn scanner_does_not_traverse_symlink_escape_paths() {
    let repo = TempDir::new().unwrap();
    let outside = TempDir::new().unwrap();
    fs::write(outside.path().join("secret.txt"), "outside\n").unwrap();
    unix_fs::symlink(outside.path(), repo.path().join("outside-link")).unwrap();

    let tree = WorkspaceFileTree::scan(
        "workspace.symlink",
        repo.path(),
        WorkspaceFileTreeLimits::new(20, 4),
    )
    .expect("tree should scan symlink metadata");

    assert!(
        entry(&tree, "outside-link")
            .is_some_and(|entry| entry.kind() == FileTreeEntryKind::Symlink)
    );
    assert!(entry(&tree, "outside-link/secret.txt").is_none());
}

fn entry<'a>(tree: &'a WorkspaceFileTree, path: &str) -> Option<&'a FileTreeEntry> {
    tree.entries().iter().find(|entry| entry.path() == path)
}
