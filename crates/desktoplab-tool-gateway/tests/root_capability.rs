use std::fs;

use desktoplab_tool_gateway::{BatchPatchItem, BatchPatchOutcome, FilesystemBatchPatchExecutor};
use tempfile::TempDir;

#[cfg(unix)]
use desktoplab_policy::PolicyEngine;
#[cfg(unix)]
use desktoplab_tool_gateway::{
    FilesystemApproval, FilesystemPatchApproval, FilesystemPatchExecutor, FilesystemPatchOutcome,
    FilesystemPatchRequest, FilesystemToolExecutor, FilesystemToolOutcome,
};

#[test]
#[cfg(unix)]
fn every_filesystem_mutation_executor_rejects_hardlink_aliases() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside.txt");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(&outside, "before").unwrap();
    for name in ["write.txt", "patch.txt", "batch.txt"] {
        fs::hard_link(&outside, workspace.join(name)).unwrap();
    }

    let mut write = FilesystemToolExecutor::new(&workspace, PolicyEngine::default_conservative());
    assert_eq!(
        write.write("write.txt", "after", FilesystemApproval::Approved),
        FilesystemToolOutcome::Blocked("path_escape")
    );
    let mut patch = FilesystemPatchExecutor::new(&workspace, PolicyEngine::default_conservative());
    assert_eq!(
        patch.apply(
            FilesystemPatchRequest::replace("patch.txt", "before", "after"),
            FilesystemPatchApproval::Approved,
        ),
        FilesystemPatchOutcome::Blocked("path_escape")
    );
    let batch = FilesystemBatchPatchExecutor::new(&workspace);
    assert!(matches!(
        batch.apply(&[BatchPatchItem {
            path: "batch.txt".to_string(),
            expected: "before".to_string(),
            replacement: "after".to_string(),
        }]),
        BatchPatchOutcome::Blocked(_)
    ));
    assert_eq!(fs::read_to_string(outside).unwrap(), "before");
}

#[test]
fn batch_patch_validates_every_open_target_before_writing_any_file() {
    let fixture = TempDir::new().unwrap();
    fs::write(fixture.path().join("first.txt"), "old").unwrap();
    fs::write(fixture.path().join("second.txt"), "different").unwrap();
    let executor = FilesystemBatchPatchExecutor::new(fixture.path());

    let outcome = executor.apply(&[
        BatchPatchItem {
            path: "first.txt".to_string(),
            expected: "old".to_string(),
            replacement: "new".to_string(),
        },
        BatchPatchItem {
            path: "second.txt".to_string(),
            expected: "missing".to_string(),
            replacement: "new".to_string(),
        },
    ]);

    assert_eq!(
        outcome,
        BatchPatchOutcome::Conflict("second.txt".to_string())
    );
    assert_eq!(
        fs::read_to_string(fixture.path().join("first.txt")).unwrap(),
        "old"
    );
}

#[test]
fn root_capability_source_stays_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/workspace_root.rs",
        include_str!("../src/workspace_root.rs"),
        240,
    )
    .unwrap();
}
