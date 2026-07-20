use desktoplab_backend_services::{
    ApprovalResolution, ApprovalService, ApprovalState, ApprovalStore, SessionWaitState,
};
use desktoplab_tool_gateway::TerminalCommandRequest;
use xtask::check_logical_line_limit;

#[test]
fn pending_approval_survives_service_restart() {
    let store = ApprovalStore::default();
    let mut first_service = ApprovalService::new(store.clone());

    let request = first_service.request("session.1", "filesystem.write");
    let restarted_service = ApprovalService::new(store);

    assert_eq!(
        restarted_service
            .get(request.id())
            .expect("request should persist")
            .state(),
        ApprovalState::Pending
    );
}

#[test]
fn approval_resolution_resumes_blocked_session() {
    let store = ApprovalStore::default();
    let mut service = ApprovalService::new(store);
    let request = service.request("session.1", "terminal.command");
    let mut session = SessionWaitState::blocked_on(request.id());

    service
        .resolve(request.id(), ApprovalResolution::Approve)
        .expect("approval should resolve");
    service.resume_waiting_session(&mut session);

    assert_eq!(session, SessionWaitState::Resumed);
    assert_eq!(service.audit_count(), 2);
}

#[test]
fn denial_keeps_session_blocked() {
    let store = ApprovalStore::default();
    let mut service = ApprovalService::new(store);
    let request = service.request("session.1", "git.push");
    let mut session = SessionWaitState::blocked_on(request.id());

    service
        .resolve(request.id(), ApprovalResolution::Deny)
        .expect("denial should resolve");
    service.resume_waiting_session(&mut session);

    assert!(matches!(session, SessionWaitState::Blocked { .. }));
}

#[test]
fn expired_approval_cannot_execute() {
    let store = ApprovalStore::default();
    let mut service = ApprovalService::new(store);
    let request = service.request("session.1", "filesystem.write");

    service.expire(request.id());
    let result = service.resolve(request.id(), ApprovalResolution::Approve);

    assert_eq!(result, Err("approval_expired"));
    assert_eq!(
        service
            .get(request.id())
            .expect("request should exist")
            .state(),
        ApprovalState::Expired
    );
}

#[test]
fn session_invalidation_expires_pending_and_approved_unconsumed_requests() {
    let mut service = ApprovalService::new(ApprovalStore::default());
    let pending = service.request("session.cancelled", "filesystem.write");
    let approved = service.request("session.cancelled", "terminal.command");
    service
        .resolve(approved.id(), ApprovalResolution::Approve)
        .unwrap();

    service.invalidate_unconsumed_for_session("session.cancelled");

    assert_eq!(
        service.get(pending.id()).unwrap().state(),
        ApprovalState::Expired
    );
    assert_eq!(
        service.get(approved.id()).unwrap().state(),
        ApprovalState::Expired
    );
}

#[test]
fn approved_payload_can_only_be_consumed_by_its_owner_session() {
    let mut service = ApprovalService::new(ApprovalStore::default());
    let request = service.request_operation_with_payload_hash(
        "session.owner",
        "filesystem.write",
        "filesystem.write:README.md",
        Some("payload.hash"),
    );
    service
        .resolve(request.id(), ApprovalResolution::Approve)
        .unwrap();

    assert!(!service.consume_approved_for_payload(
        request.id(),
        "session.attacker",
        "filesystem.write",
        "filesystem.write:README.md",
        Some("payload.hash"),
    ));
    assert!(service.consume_approved_for_payload(
        request.id(),
        "session.owner",
        "filesystem.write",
        "filesystem.write:README.md",
        Some("payload.hash"),
    ));
}

#[test]
fn approval_resolution_is_terminal_and_idempotent() {
    let mut service = ApprovalService::new(ApprovalStore::default());
    let request = service.request("session.1", "filesystem.write");

    service
        .resolve(request.id(), ApprovalResolution::Approve)
        .expect("first resolution should succeed");
    let repeated = service
        .resolve(request.id(), ApprovalResolution::Approve)
        .expect("same resolution should be idempotent");
    let conflicting = service.resolve(request.id(), ApprovalResolution::Deny);
    service.expire(request.id());

    assert_eq!(repeated.state(), ApprovalState::Approved);
    assert_eq!(conflicting, Err("approval_already_resolved"));
    assert_eq!(
        service.get(request.id()).unwrap().state(),
        ApprovalState::Approved
    );
    assert_eq!(service.audit_count(), 2);
}

#[test]
fn terminal_command_defaults_to_requires_approval_with_exact_copy() {
    let store = ApprovalStore::default();
    let mut service = ApprovalService::new(store);
    let request = TerminalCommandRequest::for_workspace("workspace.desktoplab", "npm test")
        .with_working_directory("apps/desktop");

    let approval = service.request_terminal_command("session.1", &request);

    assert_eq!(approval.state(), ApprovalState::Pending);
    assert!(!approval.can_run());
    assert!(approval.copy().contains("npm test"));
    assert!(approval.copy().contains("apps/desktop"));
}

#[test]
fn approved_terminal_command_moves_to_runnable_state() {
    let store = ApprovalStore::default();
    let mut service = ApprovalService::new(store);
    let request = TerminalCommandRequest::for_workspace("workspace.desktoplab", "npm test");
    let approval = service.request_terminal_command("session.1", &request);

    service
        .resolve(approval.approval_id(), ApprovalResolution::Approve)
        .expect("approval should resolve");
    let decision = service
        .terminal_command_decision(approval.approval_id(), &request)
        .expect("terminal approval should exist");

    assert_eq!(decision.state(), ApprovalState::Approved);
    assert!(decision.can_run());
}

#[test]
fn denied_terminal_command_records_evidence_and_cannot_run() {
    let store = ApprovalStore::default();
    let mut service = ApprovalService::new(store);
    let request =
        TerminalCommandRequest::for_workspace("workspace.desktoplab", "printf secret=abc");
    let approval = service.request_terminal_command("session.1", &request);

    service
        .resolve(approval.approval_id(), ApprovalResolution::Deny)
        .expect("approval should resolve");
    let decision = service
        .terminal_command_decision(approval.approval_id(), &request)
        .expect("terminal approval should exist");

    assert_eq!(decision.state(), ApprovalState::Denied);
    assert!(!decision.can_run());
    assert!(decision.evidence().contains("denied"));
    assert!(!decision.evidence().contains("abc"));
    assert_eq!(service.audit_count(), 2);
}

#[test]
fn approval_service_source_stays_below_current_extraction_guard() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-backend-services/src/approval.rs",
            include_str!("../src/approval.rs"),
            380,
        ),
        (
            "crates/desktoplab-backend-services/src/approval_consumption.rs",
            include_str!("../src/approval_consumption.rs"),
            80,
        ),
        (
            "crates/desktoplab-backend-services/src/approval_invalidation.rs",
            include_str!("../src/approval_invalidation.rs"),
            80,
        ),
    ] {
        check_logical_line_limit(path, source, limit)
            .expect("approval service source should stay below its extraction guard");
    }
}
