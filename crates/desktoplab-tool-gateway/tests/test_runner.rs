use std::time::Duration;

use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{
    FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome, TerminalCommandClass,
    TestRunApproval, TestRunOutcome, TestRunRequest, TestRunnerExecutor, classify_terminal_command,
};
use desktoplab_workspace::TestCommandDetector;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn detects_likely_test_commands_without_execution() {
    let fixture = TempDir::new().expect("fixture should exist");
    std::fs::write(
        fixture.path().join("package.json"),
        r#"{"scripts":{"test":"vitest"}}"#,
    )
    .expect("package fixture should write");
    std::fs::write(
        fixture.path().join("Cargo.toml"),
        "[package]\nname='demo'\n",
    )
    .expect("cargo fixture should write");
    std::fs::write(
        fixture.path().join("pyproject.toml"),
        "[tool.pytest.ini_options]\n",
    )
    .expect("python fixture should write");
    std::fs::write(fixture.path().join("go.mod"), "module example.test/demo\n")
        .expect("go fixture should write");
    std::fs::write(
        fixture.path().join("Package.swift"),
        "// swift-tools-version: 6.0\n",
    )
    .expect("swift fixture should write");

    let detected = TestCommandDetector::detect(fixture.path()).unwrap();

    assert!(detected.has_high_confidence("npm test"));
    assert!(detected.has_high_confidence("cargo test"));
    assert!(detected.has_low_confidence("pytest"));
    assert!(detected.has_high_confidence("go test ./..."));
    assert!(detected.has_high_confidence("swift test"));
    assert!(!detected.executed_any_command());
}

#[test]
fn test_runner_requires_approval_and_captures_redacted_output() {
    let fixture = TempDir::new().expect("fixture should exist");
    let mut runner = TestRunnerExecutor::new(
        fixture.path(),
        PolicyEngine::default_conservative(),
        Duration::from_secs(5),
        4096,
    );
    let command = redacted_output_command();
    let request = TestRunRequest::new("workspace.fixture", command, "validate generated change");

    assert_eq!(
        runner.run(request.clone(), TestRunApproval::Pending),
        TestRunOutcome::ApprovalRequired
    );
    let outcome = runner.run(request, TestRunApproval::Approved);

    let TestRunOutcome::Completed(evidence) = outcome else {
        panic!("test runner should execute after approval");
    };
    assert_eq!(evidence.command(), command);
    assert!(evidence.stdout().contains("ok [REDACTED]"));
    assert_eq!(evidence.redaction_status(), "redacted");
    assert!(evidence.duration_ms() < 5_000);
}

#[cfg(not(windows))]
fn redacted_output_command() -> &'static str {
    "printf 'ok sk-test-abc12345678901234567890123456789012'"
}

#[cfg(windows)]
fn redacted_output_command() -> &'static str {
    "[Console]::Write('ok sk-test-abc12345678901234567890123456789012')"
}

#[test]
fn dependency_and_generated_artifact_policy_classification_is_stable() {
    assert_eq!(
        classify_terminal_command("npm install left-pad"),
        TerminalCommandClass::DependencyInstall
    );
    assert_eq!(
        classify_terminal_command("cargo build --release"),
        TerminalCommandClass::GeneratedArtifact
    );
    assert_eq!(
        classify_terminal_command("cargo test -p desktoplab"),
        TerminalCommandClass::Routine
    );
}

#[test]
fn generated_artifact_write_enforces_budget_before_write() {
    let fixture = TempDir::new().expect("fixture should exist");
    std::fs::create_dir_all(fixture.path().join("dist")).expect("dist should exist");
    let mut filesystem =
        FilesystemToolExecutor::new(fixture.path(), PolicyEngine::default_conservative());
    let too_large = "x".repeat(1_048_577);

    assert_eq!(
        filesystem.write("dist/bundle.js", &too_large, FilesystemApproval::Approved),
        FilesystemToolOutcome::Blocked("generated_artifact_budget_exceeded")
    );
}

#[test]
fn test_runner_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/tests/test_runner.rs",
        include_str!("test_runner.rs"),
        150,
    )
    .expect("test runner test should stay focused");
}
