use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{FilesystemToolExecutor, FilesystemToolOutcome};
use std::fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn read_reports_directories_separately_from_missing_files() {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("docs")).unwrap();
    fs::write(root.path().join("docs/readme.txt"), "test contents").unwrap();
    let mut executor =
        FilesystemToolExecutor::new(root.path(), PolicyEngine::default_conservative());

    for directory in [".", "./", "docs", "docs/."] {
        assert_eq!(
            executor.read(directory),
            FilesystemToolOutcome::Blocked("not_regular_file")
        );
    }
    assert_eq!(
        executor.read("absent.txt"),
        FilesystemToolOutcome::Blocked("read_failed")
    );
    assert_eq!(
        executor.read("docs/readme.txt"),
        FilesystemToolOutcome::Read("test contents".into())
    );
}

#[test]
fn diagnostics_do_not_relax_path_or_sensitive_file_boundaries() {
    let root = TempDir::new().unwrap();
    let mut executor =
        FilesystemToolExecutor::new(root.path(), PolicyEngine::default_conservative());
    assert_eq!(
        executor.read("../outside"),
        FilesystemToolOutcome::Blocked("path_escape")
    );
    assert_eq!(
        executor.read(".env"),
        FilesystemToolOutcome::Blocked("local_only_path")
    );
}

#[test]
fn read_diagnostics_tests_stay_focused() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/tests/read_diagnostics.rs",
        include_str!("read_diagnostics.rs"),
        100,
    )
    .unwrap();
}
