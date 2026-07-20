use std::fs;

use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome};
use tempfile::TempDir;

#[test]
#[cfg(unix)]
fn root_capability_rejects_hardlink_alias_before_mutation() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside.txt");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(&outside, "outside-before").unwrap();
    fs::hard_link(&outside, workspace.join("alias.txt")).unwrap();
    let mut executor =
        FilesystemToolExecutor::new(&workspace, PolicyEngine::default_conservative());

    let outcome = executor.write("alias.txt", "outside-mutated", FilesystemApproval::Approved);

    assert_eq!(outcome, FilesystemToolOutcome::Blocked("path_escape"));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside-before");
}

#[test]
fn audit_ledger_stays_bounded() {
    xtask::check_logical_line_limit(
        "scripts/security/filesystem-race-audit.mjs",
        include_str!("../../../scripts/security/filesystem-race-audit.mjs"),
        160,
    )
    .unwrap();
}
