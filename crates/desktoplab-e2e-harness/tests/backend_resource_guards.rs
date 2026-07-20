use std::time::Duration;

use desktoplab_backend_services::{
    BackendEventScope, BackendEventStreamService, EventReplayRequest,
};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    TerminalApproval, TerminalCommandRequest, TerminalToolExecutor, TerminalToolOutcome,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn resource_guards_bound_output_replay_api_workspace_and_downloads() {
    let workspace = TempDir::new().expect("temp workspace should be created");
    let mut terminal = TerminalToolExecutor::new(
        workspace.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(2),
        24,
    );
    let terminal_outcome = terminal.execute(
        TerminalCommandRequest::new(
            "workspace.fixture",
            "printf 'abcdefghijklmnopqrstuvwxyz0123456789'",
        ),
        TerminalApproval::Approved,
    );
    let TerminalToolOutcome::Completed(result) = terminal_outcome else {
        panic!("terminal should complete");
    };
    assert!(result.stdout().len() <= 24);
    assert_eq!(result.stdout(), "abcdefghijklmnopqrstuvwx");

    let mut events = BackendEventStreamService::default();
    for index in 0..300 {
        events.publish(BackendEventScope::Session, format!("event={index}"));
    }
    let replay = events.replay(EventReplayRequest::new());
    assert_eq!(replay.sequences().len(), 256);
    assert_eq!(replay.sequences().last(), Some(&256));
    let resumed = events.replay(EventReplayRequest::new().after_sequence(256));
    assert_eq!(resumed.sequences().first(), Some(&257));

    let payload = desktoplab_backend_services::ApiPayloadGuard::new(12).check("0123456789abcdef");
    assert_eq!(
        payload.expect_err("oversized api payload should be rejected"),
        "api_response_too_large"
    );

    let scan = desktoplab_backend_services::WorkspaceScanGuard::new(3).classify(8);
    assert!(scan.is_degraded());
    assert_eq!(scan.reason(), Some("workspace_scan_file_limit_exceeded"));

    let disk = desktoplab_backend_services::DownloadDiskGuard::new(1_000).check(2_000);
    assert_eq!(
        disk.expect_err("oversized download should be rejected"),
        "download_disk_limit_exceeded"
    );
}

#[test]
fn backend_resource_guards_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/tests/backend_resource_guards.rs",
        include_str!("backend_resource_guards.rs"),
        160,
    )
    .expect("backend resource guards source should stay below the line-count guard");
}
