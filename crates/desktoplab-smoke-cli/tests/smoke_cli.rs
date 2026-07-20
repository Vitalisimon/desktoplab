use desktoplab_smoke_cli::{InProcessSmokeApi, SmokeCli, SmokeCommand};
use xtask::check_logical_line_limit;

#[test]
fn cli_talks_to_backend_health_version_and_readiness_api() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    assert_eq!(
        cli.run(SmokeCommand::Health).body(),
        r#"{"status":"healthy"}"#
    );
    assert!(
        cli.run(SmokeCommand::Version)
            .body()
            .contains("api_version")
    );
    assert!(cli.run(SmokeCommand::Readiness).body().contains("starting"));
}

#[test]
fn cli_can_run_backend_e2e_workflow_without_frontend() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    let outputs = [
        cli.run(SmokeCommand::WorkspaceOpen("workspace.demo".to_string())),
        cli.run(SmokeCommand::SetupPreview),
        cli.run(SmokeCommand::SessionStart("workspace.demo".to_string())),
        cli.run(SmokeCommand::ApprovalResolve("approval.1".to_string())),
        cli.run(SmokeCommand::Diagnostics),
        cli.run(SmokeCommand::MigrationStatus),
    ];

    assert!(outputs.iter().all(|output| output.is_json()));
    assert!(outputs[4].body().contains("offline=true"));
}

#[test]
fn cli_exposes_read_only_migration_status() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    let output = cli.run(SmokeCommand::MigrationStatus);

    assert_eq!(output.kind(), "migration.status");
    assert!(output.is_json());
    assert!(
        output
            .body()
            .contains("migration-001-local-storage-event-log")
    );
    assert!(output.body().contains("\"redacted\":true"));
}

#[test]
fn cli_output_is_structured_for_debugging() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    let output = cli.run(SmokeCommand::Diagnostics);

    assert_eq!(output.kind(), "diagnostics");
    assert!(output.body().starts_with('{'));
}

#[test]
fn cli_exposes_read_only_doctor_lint() {
    let mut cli = SmokeCli::new(InProcessSmokeApi::default());

    let output = cli.run(SmokeCommand::DoctorLint);

    assert_eq!(output.kind(), "doctor.lint");
    assert!(output.is_json());
    assert!(output.body().contains(r#""mode":"lint""#));
    assert!(output.body().contains("doctor.setup.runtime_model_ready"));
    assert!(!output.body().contains("/Users/"));
    assert!(!output.body().contains("token"));
}

#[test]
fn smoke_cli_has_no_frontend_dependency() {
    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("frontend"));
    assert!(!manifest.contains("tauri"));
}

#[test]
fn smoke_cli_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-smoke-cli/src/lib.rs",
        include_str!("../src/lib.rs"),
        300,
    )
    .expect("smoke cli source should stay below the line-count guard");
}
