use desktoplab_tool_gateway::{
    TerminalProcessAdapter, TerminalProcessRequest, TerminalProcessStatus,
};
use std::time::Duration;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn executes_harmless_command_and_captures_exit_code_and_output() {
    let temp_dir = TempDir::new().unwrap();
    let adapter = TerminalProcessAdapter::new(Duration::from_secs(1), 1024);

    let output = adapter.run(TerminalProcessRequest::new(
        write_stdout("ok"),
        temp_dir.path(),
    ));

    assert_eq!(output.status(), TerminalProcessStatus::Exited(0));
    assert_eq!(output.stdout(), "ok");
    assert_eq!(output.stderr(), "");
}

#[test]
fn timeout_cancels_long_running_command() {
    let temp_dir = TempDir::new().unwrap();
    let adapter = TerminalProcessAdapter::new(Duration::from_millis(50), 1024);

    let output = adapter.run(TerminalProcessRequest::new(slow_command(), temp_dir.path()));

    assert_eq!(output.status(), TerminalProcessStatus::TimedOut);
}

#[test]
fn output_byte_limits_report_truncation_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let adapter = TerminalProcessAdapter::new(Duration::from_secs(1), 5);

    let output = adapter.run(TerminalProcessRequest::new(
        write_stdout("123456789"),
        temp_dir.path(),
    ));

    assert_eq!(output.stdout(), "12345");
    assert!(output.stdout_truncated());
    assert_eq!(output.stdout_original_bytes(), 9);
}

#[test]
fn environment_allowlist_excludes_secrets_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let adapter = TerminalProcessAdapter::new(Duration::from_secs(1), 1024);

    let output = adapter.run(
        TerminalProcessRequest::new(print_secret_or_missing(), temp_dir.path())
            .with_env("DESKTOPLAB_SECRET_TOKEN", "raw-secret"),
    );

    assert_eq!(output.stdout(), "missing");
    assert!(!output.stdout().contains("raw-secret"));
}

#[test]
fn output_larger_than_pipe_capacity_is_drained_without_timeout() {
    let temp_dir = TempDir::new().unwrap();
    let adapter = TerminalProcessAdapter::new(Duration::from_secs(3), 4096);

    let output = adapter.run(TerminalProcessRequest::new(large_output(), temp_dir.path()));

    assert_eq!(output.status(), TerminalProcessStatus::Exited(0));
    assert_eq!(output.stdout().len(), 4096);
    assert!(output.stdout_original_bytes() >= 200_000);
    assert!(output.stdout_truncated());
}

#[test]
fn terminal_process_adapter_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/process.rs",
        include_str!("../src/process.rs"),
        240,
    )
    .expect("terminal process adapter should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/process_platform.rs",
        include_str!("../src/process_platform.rs"),
        140,
    )
    .expect("terminal platform adapter should stay below the line-count guard");
}

#[cfg(not(windows))]
fn write_stdout(value: &str) -> String {
    format!("printf '{value}'")
}

#[cfg(windows)]
fn write_stdout(value: &str) -> String {
    format!("[Console]::Write('{value}')")
}

#[cfg(not(windows))]
fn slow_command() -> &'static str {
    "sleep 2"
}

#[cfg(windows)]
fn slow_command() -> &'static str {
    "Start-Sleep -Seconds 2"
}

#[cfg(not(windows))]
fn print_secret_or_missing() -> &'static str {
    "printf \"${DESKTOPLAB_SECRET_TOKEN:-missing}\""
}

#[cfg(windows)]
fn print_secret_or_missing() -> &'static str {
    "if ($env:DESKTOPLAB_SECRET_TOKEN) { [Console]::Write($env:DESKTOPLAB_SECRET_TOKEN) } else { [Console]::Write('missing') }"
}

#[cfg(not(windows))]
fn large_output() -> &'static str {
    "yes x | head -c 200000"
}

#[cfg(windows)]
fn large_output() -> &'static str {
    "[Console]::Write(('x' * 200000))"
}
