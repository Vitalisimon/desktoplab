use desktoplab_e2e_harness::{SetupToFirstPromptHarness, SetupToFirstPromptMode};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn setup_to_first_prompt_harness_runs_without_external_network() {
    let (_fixture, workspace) = workspace_fixture();
    let outcome = SetupToFirstPromptHarness::new(SetupToFirstPromptMode::DryRun)
        .open_workspace(workspace.to_string_lossy())
        .accept_setup()
        .start_runtime_install()
        .start_model_download()
        .verify_model_ready()
        .create_agent_session("Review repository architecture")
        .run()
        .expect("dry-run setup to first prompt should pass");

    assert_eq!(outcome.workspace_id(), "workspace.desktoplab");
    assert_eq!(outcome.runtime_job_id(), "");
    assert_eq!(outcome.model_job_id(), "");
    assert_eq!(outcome.session_id(), "");
    assert_eq!(outcome.evidence_label(), "fixture-dry-run");
    assert_eq!(outcome.route_status(), "not_run");
    assert_eq!(outcome.runtime_state(), "not_run");
    assert_eq!(outcome.model_state(), "not_run");
    assert!(!outcome.certifying());
    assert!(!outcome.used_external_network());
}

#[test]
fn setup_to_first_prompt_harness_fails_explicitly_before_session_creation() {
    let (_fixture, workspace) = workspace_fixture();
    let failure = SetupToFirstPromptHarness::new(SetupToFirstPromptMode::DryRun)
        .open_workspace(workspace.to_string_lossy())
        .accept_setup()
        .start_runtime_install()
        .start_model_download()
        .create_agent_session("Review repository architecture")
        .run()
        .expect_err("missing model readiness should block session creation");

    assert_eq!(failure, "model_not_ready");
}

#[test]
fn local_services_mode_records_service_evidence_and_honest_blocked_route() {
    let (_fixture, workspace) = workspace_fixture();
    let outcome = SetupToFirstPromptHarness::new(SetupToFirstPromptMode::LocalServices)
        .open_workspace(workspace.to_string_lossy())
        .accept_setup()
        .start_runtime_install()
        .start_model_download()
        .create_agent_session("Review repository architecture")
        .run()
        .expect("local services setup should reach a session or honest blocked route");

    assert_eq!(outcome.workspace_id(), "workspace.desktoplab");
    assert_eq!(outcome.evidence_label(), "fixture-local-services");
    assert!(outcome.setup_preview_observed());
    assert_eq!(outcome.runtime_state(), "blocked");
    assert_eq!(outcome.model_state(), "blocked");
    assert_eq!(outcome.route_status(), "blocked");
    assert!(outcome.blocked_route_observed());
    assert!(!outcome.loop_event_observed());
    assert!(!outcome.used_external_network());
    assert!(!outcome.certifying());
}

#[test]
fn local_services_mode_can_record_ready_model_loop_event() {
    let (_fixture, workspace) = workspace_fixture();
    let outcome = SetupToFirstPromptHarness::new(SetupToFirstPromptMode::LocalServices)
        .open_workspace(workspace.to_string_lossy())
        .accept_setup()
        .start_runtime_install()
        .start_model_download()
        .verify_model_ready()
        .create_agent_session("Review repository architecture")
        .run()
        .expect("ready local services setup should run the minimal loop");

    assert_eq!(outcome.evidence_label(), "fixture-local-services");
    assert_eq!(outcome.workspace_id(), "workspace.desktoplab");
    assert_eq!(outcome.runtime_state(), "ready");
    assert_eq!(outcome.model_state(), "ready");
    assert_eq!(outcome.route_status(), "ready");
    assert!(outcome.setup_preview_observed());
    assert!(outcome.loop_event_observed());
    assert!(!outcome.blocked_route_observed());
    assert!(!outcome.certifying());
}

fn workspace_fixture() -> (TempDir, std::path::PathBuf) {
    let fixture = TempDir::new().expect("temp dir should exist");
    let workspace = fixture.path().join("desktoplab");
    std::fs::create_dir(&workspace).expect("workspace should exist");
    let initialized = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&workspace)
        .output()
        .expect("git init should run");
    assert!(initialized.status.success());
    (fixture, workspace)
}

#[test]
fn setup_to_first_prompt_harness_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/src/lib.rs",
        include_str!("../src/lib.rs"),
        180,
    )
    .expect("e2e harness source should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/src/setup_to_first_prompt.rs",
        include_str!("../src/setup_to_first_prompt.rs"),
        220,
    )
    .expect("setup-to-first-prompt harness should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-e2e-harness/src/setup_to_first_prompt_outcome.rs",
        include_str!("../src/setup_to_first_prompt_outcome.rs"),
        140,
    )
    .expect("setup-to-first-prompt outcome should stay focused");
}
