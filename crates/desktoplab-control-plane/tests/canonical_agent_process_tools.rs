use std::thread;
use std::time::Duration;

use desktoplab_agent_engine::{IterativeToolExecutor, ProviderToolCallNormalizer};
use desktoplab_control_plane::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};
use desktoplab_tool_gateway::SharedProcessRegistry;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn canonical_process_tools_start_write_poll_and_enforce_session_ownership() {
    let root = TempDir::new().unwrap();
    let registry = SharedProcessRegistry::default();
    let mut executor = CanonicalAgentToolExecutor::new(
        root.path(),
        "workspace.a",
        "session.a",
        CanonicalExecutionApproval::Approved,
    )
    .with_process_registry(registry.clone());
    let started = execute(
        &mut executor,
        "desktoplab.start_process",
        json!({"command":interactive_command()}),
    );
    let process_id = started["processId"].as_str().unwrap();
    execute(
        &mut executor,
        "desktoplab.write_process_stdin",
        json!({"processId":process_id,"input":"hello\n"}),
    );

    let mut output = String::new();
    let state = loop {
        let poll = execute(
            &mut executor,
            "desktoplab.poll_process",
            json!({"processId":process_id}),
        );
        output.push_str(poll["stdout"].as_str().unwrap_or_default());
        if poll["status"] != "running" {
            break poll["status"].as_str().unwrap().to_string();
        }
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(state, "exited");
    assert!(output.contains("received:hello"), "{output}");

    let mut foreign = CanonicalAgentToolExecutor::new(
        root.path(),
        "workspace.a",
        "session.b",
        CanonicalExecutionApproval::Approved,
    )
    .with_process_registry(registry);
    assert_eq!(
        execute_result(
            &mut foreign,
            "desktoplab.poll_process",
            json!({"processId":process_id})
        ),
        Err("process_ownership_denied".to_string())
    );
}

#[test]
fn process_start_cannot_bypass_approval() {
    let root = TempDir::new().unwrap();
    let mut executor = CanonicalAgentToolExecutor::new(
        root.path(),
        "workspace.a",
        "session.a",
        CanonicalExecutionApproval::Pending,
    );
    assert_eq!(
        execute_result(
            &mut executor,
            "desktoplab.start_process",
            json!({"command":interactive_command()})
        ),
        Err("approval_required".to_string())
    );
}

fn execute(executor: &mut CanonicalAgentToolExecutor, name: &str, arguments: Value) -> Value {
    execute_result(executor, name, arguments).unwrap()
}

fn execute_result(
    executor: &mut CanonicalAgentToolExecutor,
    name: &str,
    arguments: Value,
) -> Result<Value, String> {
    let call = ProviderToolCallNormalizer::default()
        .normalize(format!("call-{name}"), name, arguments)
        .unwrap();
    executor
        .execute(&call)
        .map(|result| result.output().clone())
}

#[cfg(not(windows))]
fn interactive_command() -> &'static str {
    "read line; printf 'received:%s' \"$line\""
}

#[cfg(windows)]
fn interactive_command() -> &'static str {
    "$line = [Console]::In.ReadLine(); [Console]::Write(\"received:$line\")"
}
