use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use desktoplab_runtime::{
    LmStudioManagedPhase, LmStudioManagedPlan, LmStudioManagedPlanError, LmStudioModelProbe,
    ProcessCommand, ProcessOutput, ProcessRunner, RuntimeEndpointError, RuntimeExecutionState,
    execute_managed_lm_studio, stop_managed_lm_studio,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[derive(Clone)]
struct RecordingRunner {
    outputs: Arc<Mutex<VecDeque<(Option<i32>, String, String)>>>,
    commands: Arc<Mutex<Vec<ProcessCommand>>>,
}

impl RecordingRunner {
    fn new(outputs: Vec<(Option<i32>, &str, &str)>) -> Self {
        Self {
            outputs: Arc::new(Mutex::new(
                outputs
                    .into_iter()
                    .map(|(code, stdout, stderr)| (code, stdout.to_string(), stderr.to_string()))
                    .collect(),
            )),
            commands: Arc::default(),
        }
    }

    fn commands(&self) -> Vec<ProcessCommand> {
        self.commands.lock().expect("commands").clone()
    }
}

impl ProcessRunner for RecordingRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        self.commands
            .lock()
            .expect("commands")
            .push(command.clone());
        let (code, stdout, stderr) = self
            .outputs
            .lock()
            .expect("outputs")
            .pop_front()
            .unwrap_or((Some(1), String::new(), "unexpected command".to_string()));
        ProcessOutput::new(code, stdout, stderr, command)
    }
}

struct Models(Vec<String>);

impl LmStudioModelProbe for Models {
    fn models(&self, _endpoint: &str) -> Result<Vec<String>, RuntimeEndpointError> {
        Ok(self.0.clone())
    }
}

fn plan(temp: &TempDir) -> LmStudioManagedPlan {
    LmStudioManagedPlan::new(
        temp.path()
            .join("DesktopLab")
            .join("runtimes")
            .join("lm-studio"),
        "darwin-arm64",
        "openai/gpt-oss-20b",
        "desktoplab-gpt-oss-20b",
        12_345,
        true,
    )
    .expect("managed plan")
}

fn successful_outputs(sha512: &str, pid: u64) -> Vec<(Option<i32>, String, String)> {
    vec![
        (Some(0), String::new(), String::new()),
        (Some(0), format!("{sha512}  llmster.tar.gz"), String::new()),
        (Some(0), String::new(), String::new()),
        (Some(0), String::new(), String::new()),
        (Some(0), "lms help".to_string(), String::new()),
        (
            Some(0),
            format!(r#"{{"status":"running","pid":{pid},"isDaemon":true}}"#),
            String::new(),
        ),
        (Some(0), String::new(), String::new()),
        (Some(0), String::new(), String::new()),
        (Some(0), String::new(), String::new()),
    ]
}

fn borrowed(outputs: &[(Option<i32>, String, String)]) -> Vec<(Option<i32>, &str, &str)> {
    outputs
        .iter()
        .map(|(code, stdout, stderr)| (*code, stdout.as_str(), stderr.as_str()))
        .collect()
}

#[test]
fn plan_is_pinned_bounded_and_complete() {
    let temp = TempDir::new().expect("temp");
    let plan = plan(&temp);
    assert_eq!(plan.artifact().version(), "0.0.20-1");
    assert!(
        plan.artifact()
            .url()
            .starts_with("https://llmster.lmstudio.ai/")
    );
    assert_eq!(plan.artifact().sha512().len(), 128);
    assert_eq!(plan.endpoint(), "http://127.0.0.1:12345");
    assert!(plan.lms_path().starts_with(plan.root()));
    assert_eq!(
        plan.phases(),
        [
            LmStudioManagedPhase::Detect,
            LmStudioManagedPhase::Acquire,
            LmStudioManagedPhase::VerifyArtifact,
            LmStudioManagedPhase::Install,
            LmStudioManagedPhase::StartDaemon,
            LmStudioManagedPhase::DownloadModel,
            LmStudioManagedPhase::LoadModel,
            LmStudioManagedPhase::StartServer,
            LmStudioManagedPhase::Health,
            LmStudioManagedPhase::PersistOwnership,
        ]
    );
}

#[test]
fn plan_fails_closed_for_terms_platform_root_model_and_port() {
    let temp = TempDir::new().expect("temp");
    let root = temp
        .path()
        .join("DesktopLab")
        .join("runtimes")
        .join("lm-studio");
    let build = |root, target, model, port, accepted| {
        LmStudioManagedPlan::new(root, target, model, "desktoplab-model", port, accepted)
    };
    assert_eq!(
        build(root.clone(), "darwin-arm64", "safe/model", 1234, false),
        Err(LmStudioManagedPlanError::VendorTermsNotAccepted)
    );
    assert_eq!(
        build(root.clone(), "windows-x64", "safe/model", 1234, true),
        Err(LmStudioManagedPlanError::UnsupportedPlatform)
    );
    assert_eq!(
        build(
            temp.path().join("other"),
            "darwin-arm64",
            "safe/model",
            1234,
            true
        ),
        Err(LmStudioManagedPlanError::UnsafeManagedRoot)
    );
    assert_eq!(
        build(root.clone(), "darwin-arm64", "../escape", 1234, true),
        Err(LmStudioManagedPlanError::UnsafeModelId)
    );
    assert_eq!(
        build(root, "darwin-arm64", "safe/model", 80, true),
        Err(LmStudioManagedPlanError::UnsafePort)
    );
}

#[test]
fn execution_uses_private_home_loopback_and_persists_ownership() {
    let temp = TempDir::new().expect("temp");
    let plan = plan(&temp);
    let outputs = successful_outputs(plan.artifact().sha512(), 42);
    let runner = RecordingRunner::new(borrowed(&outputs));

    let result = execute_managed_lm_studio(
        &runner,
        &Models(vec![plan.api_model_id().to_string()]),
        &plan,
    );

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert!(result.desktoplab_started_runtime());
    assert!(plan.marker_path().is_file());
    let commands = runner.commands();
    assert_eq!(commands.len(), 9);
    assert!(commands.iter().skip(3).all(|command| {
        command.env_value("HOME") == Some(plan.managed_home().to_string_lossy().as_ref())
    }));
    let server = &commands[8];
    assert_eq!(
        server.args(),
        ["server", "start", "--port", "12345", "--bind", "127.0.0.1"]
    );
}

#[test]
fn checksum_mismatch_stops_before_extract_or_execution() {
    let temp = TempDir::new().expect("temp");
    let plan = plan(&temp);
    let runner = RecordingRunner::new(vec![
        (Some(0), "", ""),
        (Some(0), "deadbeef  llmster.tar.gz", ""),
    ]);

    let result = execute_managed_lm_studio(&runner, &Models(Vec::new()), &plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "artifact_verification_failed");
    assert_eq!(runner.commands().len(), 2);
    assert!(!plan.marker_path().exists());
}

#[test]
fn recovery_reuses_only_matching_owned_daemon() {
    let temp = TempDir::new().expect("temp");
    let plan = plan(&temp);
    let outputs = successful_outputs(plan.artifact().sha512(), 77);
    let first = RecordingRunner::new(borrowed(&outputs));
    let models = Models(vec![plan.api_model_id().to_string()]);
    assert_eq!(
        execute_managed_lm_studio(&first, &models, &plan).state(),
        RuntimeExecutionState::Completed
    );
    let recovery = RecordingRunner::new(vec![(
        Some(0),
        r#"{"status":"running","pid":77,"isDaemon":true}"#,
        "",
    )]);

    let recovered = execute_managed_lm_studio(&recovery, &models, &plan);

    assert_eq!(recovered.state(), RuntimeExecutionState::Completed);
    assert!(!recovered.desktoplab_started_runtime());
    assert_eq!(recovery.commands().len(), 1);
}

#[test]
fn stop_refuses_unowned_or_pid_mismatched_daemon() {
    let temp = TempDir::new().expect("temp");
    let plan = plan(&temp);
    let unowned = RecordingRunner::new(Vec::new());
    assert_eq!(
        stop_managed_lm_studio(&unowned, &plan).state(),
        RuntimeExecutionState::Blocked
    );
    assert!(unowned.commands().is_empty());

    let outputs = successful_outputs(plan.artifact().sha512(), 91);
    let install = RecordingRunner::new(borrowed(&outputs));
    let models = Models(vec![plan.api_model_id().to_string()]);
    let _ = execute_managed_lm_studio(&install, &models, &plan);
    let mismatch = RecordingRunner::new(vec![(
        Some(0),
        r#"{"status":"running","pid":92,"isDaemon":true}"#,
        "",
    )]);
    assert_eq!(
        stop_managed_lm_studio(&mismatch, &plan).state(),
        RuntimeExecutionState::Blocked
    );
    assert_eq!(mismatch.commands().len(), 1);
}

#[test]
fn stop_shuts_down_only_the_marker_matched_daemon() {
    let temp = TempDir::new().expect("temp");
    let plan = plan(&temp);
    let outputs = successful_outputs(plan.artifact().sha512(), 101);
    let install = RecordingRunner::new(borrowed(&outputs));
    let models = Models(vec![plan.api_model_id().to_string()]);
    let _ = execute_managed_lm_studio(&install, &models, &plan);
    let stop = RecordingRunner::new(vec![
        (
            Some(0),
            r#"{"status":"running","pid":101,"isDaemon":true}"#,
            "",
        ),
        (Some(0), "", ""),
        (Some(0), "", ""),
    ]);

    let stopped = stop_managed_lm_studio(&stop, &plan);

    assert_eq!(stopped.state(), RuntimeExecutionState::Completed);
    assert!(!plan.marker_path().exists());
    let commands = stop.commands();
    assert_eq!(commands[1].args(), ["server", "stop"]);
    assert_eq!(commands[2].args(), ["daemon", "down"]);
}

#[test]
fn managed_runtime_sources_stay_small() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-runtime/src/lm_studio_managed_plan.rs",
            include_str!("../src/lm_studio_managed_plan.rs"),
            210,
        ),
        (
            "crates/desktoplab-runtime/src/lm_studio_managed_execution.rs",
            include_str!("../src/lm_studio_managed_execution.rs"),
            280,
        ),
        (
            "crates/desktoplab-runtime/src/lm_studio_managed_marker.rs",
            include_str!("../src/lm_studio_managed_marker.rs"),
            110,
        ),
        (
            "crates/desktoplab-runtime/src/lm_studio_managed_lifecycle.rs",
            include_str!("../src/lm_studio_managed_lifecycle.rs"),
            100,
        ),
        (
            "crates/desktoplab-runtime/src/lm_studio_managed_status.rs",
            include_str!("../src/lm_studio_managed_status.rs"),
            100,
        ),
    ] {
        check_logical_line_limit(path, source, limit).expect("managed LM Studio module guardrail");
    }
}
