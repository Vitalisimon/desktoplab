use desktoplab_runtime::{
    RuntimeHealth, RuntimeId, RuntimeManagementMode, RuntimeProcessSpec, RuntimeProcessSupervisor,
    RuntimeState, RuntimeSupervisorError,
};
use xtask::check_logical_line_limit;

#[test]
fn managed_start_transitions_through_starting_running_and_ready() {
    let mut supervisor = RuntimeProcessSupervisor::new(RuntimeProcessSpec::managed(
        RuntimeId::new("runtime.ollama"),
        "Ollama",
    ))
    .with_health_checks([RuntimeHealth::healthy()]);

    let report = supervisor.start().expect("managed runtime should start");

    assert_eq!(
        report.state_transitions(),
        vec![
            RuntimeState::Starting,
            RuntimeState::Running,
            RuntimeState::Ready,
        ]
    );
    assert_eq!(supervisor.status().state(), RuntimeState::Ready);
}

#[test]
fn failed_health_check_blocks_readiness() {
    let mut supervisor = RuntimeProcessSupervisor::new(RuntimeProcessSpec::managed(
        RuntimeId::new("runtime.ollama"),
        "Ollama",
    ))
    .with_health_checks([RuntimeHealth::unhealthy("health endpoint unavailable")]);

    let report = supervisor
        .start()
        .expect("process can start but fail health");

    assert_eq!(
        report.state_transitions(),
        vec![
            RuntimeState::Starting,
            RuntimeState::Running,
            RuntimeState::VerificationFailed,
        ]
    );
    assert_eq!(
        supervisor.status().state(),
        RuntimeState::VerificationFailed
    );
    assert_eq!(
        supervisor.status().failure_reason(),
        Some("health endpoint unavailable")
    );
}

#[test]
fn externally_managed_runtime_cannot_be_forcibly_stopped() {
    let mut supervisor = RuntimeProcessSupervisor::new(RuntimeProcessSpec::external(
        RuntimeId::new("runtime.lm-studio"),
        "LM Studio",
    ));

    let error = supervisor
        .stop()
        .expect_err("external runtime should not be force-stopped");

    assert_eq!(
        error,
        RuntimeSupervisorError::ExternallyManaged(RuntimeId::new("runtime.lm-studio"))
    );
    assert_eq!(
        supervisor.spec().management_mode(),
        RuntimeManagementMode::External
    );
}

#[test]
fn runtime_logs_are_bounded_and_redacted() {
    let mut supervisor = RuntimeProcessSupervisor::new(
        RuntimeProcessSpec::managed(RuntimeId::new("runtime.ollama"), "Ollama").with_log_limit(2),
    );

    supervisor.record_log("first line");
    supervisor.record_log("token=abc123");
    supervisor.record_log("api_key=sk-live-secret");

    assert_eq!(supervisor.logs().len(), 2);
    assert!(!supervisor.logs().join("\n").contains("abc123"));
    assert!(!supervisor.logs().join("\n").contains("sk-live-secret"));
    assert!(
        supervisor
            .logs()
            .iter()
            .any(|line| line.contains("[REDACTED]"))
    );
}

#[test]
fn runtime_supervisor_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/supervisor.rs",
        include_str!("../src/supervisor.rs"),
        280,
    )
    .expect("runtime supervisor source should stay below the initial line-count guard");
}
