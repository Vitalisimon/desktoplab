use desktoplab_smoke_cli::{InProcessSmokeApi, SmokeCli, SmokeCommand, SmokeOutputFormat};
use xtask::check_logical_line_limit;

#[test]
fn cli_exposes_read_only_operator_commands_as_json() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    let commands = [
        (SmokeCommand::Status, "status", "readiness"),
        (
            SmokeCommand::DoctorLint,
            "doctor.lint",
            "doctor.setup.runtime_model_ready",
        ),
        (
            SmokeCommand::DiagnosticsExport,
            "diagnostics.export",
            "desktoplab.diagnostics.export",
        ),
        (
            SmokeCommand::RuntimeInspect,
            "runtime.inspect",
            "runtime_and_model_not_verified",
        ),
        (
            SmokeCommand::SecurityAudit,
            "security.audit",
            "security.plugins.provenance",
        ),
        (
            SmokeCommand::MigrationStatus,
            "migration.status",
            "migration-001-local-storage-event-log",
        ),
    ];

    for (command, kind, needle) in commands {
        let output = cli.run(command);
        assert_eq!(output.kind(), kind);
        assert!(output.is_json(), "{kind} should emit json");
        assert!(output.body().contains(needle), "{kind} missing {needle}");
        assert!(!output.body().contains("/Users/"));
        assert!(!output.body().contains("sk-live"));
    }
}

#[test]
fn cli_operator_commands_support_plain_output_without_enabling_repairs() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    let output = cli.run_with_format(SmokeCommand::SecurityAudit, SmokeOutputFormat::Plain);

    assert_eq!(output.kind(), "security.audit");
    assert!(!output.is_json());
    assert!(output.body().starts_with("security.audit:"));
    assert!(!output.body().contains("repair accepted"));
}

#[test]
fn operator_command_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-smoke-cli/tests/operator_commands.rs",
        include_str!("operator_commands.rs"),
        120,
    )
    .expect("operator command tests should stay focused");
}
