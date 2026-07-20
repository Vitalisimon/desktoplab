use std::thread;
use std::time::Duration;

use desktoplab_tool_gateway::{ManagedProcessState, SharedProcessRegistry};
use tempfile::TempDir;

#[test]
fn managed_process_supports_incremental_output_stdin_and_exit() {
    let root = TempDir::new().unwrap();
    let registry = SharedProcessRegistry::default();
    let started = registry
        .start(
            root.path(),
            "workspace.a",
            "session.a",
            interactive_command(),
            "",
        )
        .unwrap();
    let process_id = started.process_id().to_string();

    registry
        .write_stdin("workspace.a", "session.a", &process_id, "hello\n")
        .unwrap();
    let (state, stdout) = wait_until_finished(&registry, &process_id);

    assert_eq!(state, ManagedProcessState::Exited(0));
    assert!(stdout.contains("received:hello"));
}

#[test]
fn process_ownership_is_enforced_and_kill_terminates_the_tree() {
    let root = TempDir::new().unwrap();
    let registry = SharedProcessRegistry::default();
    let started = registry
        .start(root.path(), "workspace.a", "session.a", slow_command(), "")
        .unwrap();
    let process_id = started.process_id();

    assert_eq!(
        registry
            .poll("workspace.a", "session.b", process_id)
            .unwrap_err(),
        "process_ownership_denied"
    );
    let killed = registry
        .kill("workspace.a", "session.a", process_id)
        .unwrap();
    assert_eq!(killed.state(), &ManagedProcessState::Killed);
}

#[test]
fn process_cwd_must_exist_inside_workspace() {
    let root = TempDir::new().unwrap();
    let registry = SharedProcessRegistry::default();

    assert_eq!(
        registry
            .start(root.path(), "workspace.a", "session.a", "echo no", "../")
            .unwrap_err(),
        "path_escape"
    );
}

#[test]
fn cancelling_a_session_kills_only_its_owned_processes() {
    let root = TempDir::new().unwrap();
    let registry = SharedProcessRegistry::default();
    let first = registry
        .start(root.path(), "workspace.a", "session.a", slow_command(), "")
        .unwrap();
    let second = registry
        .start(root.path(), "workspace.a", "session.b", slow_command(), "")
        .unwrap();

    assert_eq!(
        registry.kill_session("workspace.a", "session.a").unwrap(),
        1
    );
    assert_eq!(
        registry
            .poll("workspace.a", "session.a", first.process_id())
            .unwrap()
            .state(),
        &ManagedProcessState::Killed
    );
    assert_eq!(
        registry
            .poll("workspace.a", "session.b", second.process_id())
            .unwrap()
            .state(),
        &ManagedProcessState::Running
    );
    registry
        .kill("workspace.a", "session.b", second.process_id())
        .unwrap();
}

#[test]
fn managed_process_source_stays_below_line_guard() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/managed_process.rs",
        include_str!("../src/managed_process.rs"),
        330,
    )
    .unwrap();
}

fn wait_until_finished(
    registry: &SharedProcessRegistry,
    process_id: &str,
) -> (ManagedProcessState, String) {
    let mut stdout = String::new();
    for _ in 0..100 {
        let snapshot = registry
            .poll("workspace.a", "session.a", process_id)
            .unwrap();
        stdout.push_str(snapshot.stdout());
        if snapshot.state() != &ManagedProcessState::Running {
            return (snapshot.state().clone(), stdout);
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("managed process did not finish");
}

#[cfg(not(windows))]
fn interactive_command() -> &'static str {
    "read line; printf 'received:%s' \"$line\""
}

#[cfg(windows)]
fn interactive_command() -> &'static str {
    "$line = [Console]::In.ReadLine(); [Console]::Write(\"received:$line\")"
}

#[cfg(not(windows))]
fn slow_command() -> &'static str {
    "sleep 30"
}

#[cfg(windows)]
fn slow_command() -> &'static str {
    "Start-Sleep -Seconds 30"
}
