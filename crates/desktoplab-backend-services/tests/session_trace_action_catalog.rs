use desktoplab_agent_session::SessionEvent;
use desktoplab_backend_services::{SessionService, SessionServiceStore};

#[test]
fn persisted_trace_recognizes_every_native_action_family() {
    let cases = [
        (
            "filesystem.create_directory",
            "filesystem.create_directory:docs",
            "desktoplab.create_directory",
            true,
        ),
        (
            "filesystem.move",
            "filesystem.move:old:new",
            "desktoplab.move_path",
            true,
        ),
        (
            "filesystem.delete",
            "filesystem.delete:old:recursive=true",
            "desktoplab.delete_path",
            true,
        ),
        (
            "process.start",
            "process.start:npm-run-dev",
            "desktoplab.start_process",
            true,
        ),
        (
            "process.poll",
            "process.poll:process-1",
            "desktoplab.poll_process",
            false,
        ),
        (
            "process.stdin",
            "process.stdin:process-1",
            "desktoplab.write_process_stdin",
            true,
        ),
        (
            "process.kill",
            "process.kill:process-1",
            "desktoplab.kill_process",
            true,
        ),
        (
            "mcp.tool.invoke",
            "mcp.invoke:mcp.browser.open",
            "mcp.browser.open",
            true,
        ),
    ];

    for (ordinal, (source, evidence, canonical, mutation)) in cases.into_iter().enumerate() {
        let mut service = SessionService::new(SessionServiceStore::default());
        let session = service.create_session(format!("workspace-{ordinal}"), "backend.ollama");
        service.append_events(
            session.session_id(),
            &[SessionEvent::tool_decision_recorded(format!(
                "state=observed source={source} tool={evidence}"
            ))],
        );
        let event = service.trace(session.session_id()).unwrap().events()[1].to_value();
        assert_eq!(event["source"], canonical, "{source}");
        assert_eq!(event["mutation"], mutation, "{source}");
    }
}

#[test]
fn persisted_trace_accepts_canonical_router_tool_records() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace", "backend.ollama");
    service.append_events(
        session.session_id(),
        &[SessionEvent::tool_decision_recorded(
            "state=observed source=agent.iterative tool=desktoplab.spawn_subagent call_id=call-1",
        )],
    );
    let event = service.trace(session.session_id()).unwrap().events()[1].to_value();
    assert_eq!(event["source"], "desktoplab.spawn_subagent");
    assert_eq!(event["mutation"], true);
}

#[test]
fn policy_preflight_and_failures_keep_distinct_trace_semantics() {
    let mut service = SessionService::new(SessionServiceStore::default());
    let session = service.create_session("workspace", "backend.ollama");
    service.append_events(
        session.session_id(),
        &[
            SessionEvent::tool_decision_recorded(
                "event=before_tool source=filesystem.read canonical=desktoplab.read_file tool=filesystem.read:README.md approval_state=not_required",
            ),
            SessionEvent::tool_decision_recorded(
                "event=before_tool source=filesystem.write canonical=desktoplab.write_file tool=filesystem.write:notes.md approval_state=pending",
            ),
            SessionEvent::tool_decision_recorded(
                "state=failed source=filesystem.write canonical=desktoplab.write_file tool=filesystem.write:notes.md",
            ),
        ],
    );

    let trace = service.trace(session.session_id()).unwrap();
    assert_eq!(trace.events()[1].kind(), "tool_requested");
    assert_eq!(trace.events()[1].to_value()["mutation"], false);
    assert_eq!(trace.events()[2].kind(), "approval_required");
    assert_eq!(trace.events()[2].to_value()["mutation"], false);
    assert_eq!(trace.events()[3].kind(), "tool_observed");
    assert_eq!(trace.events()[3].to_value()["mutation"], true);
    assert_eq!(trace.events()[3].success(), Some(false));
}
