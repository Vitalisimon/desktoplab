use std::sync::{Arc, Mutex};

use desktoplab_runtime::{
    DeterministicProcessRunner, LmStudioModelProbe, MlxLmManagedPhase, MlxLmManagedPlan,
    MlxLmManagedPlanError, MlxLmManagedState, ProcessCommand, ProcessSpawner, RuntimeEndpointError,
    RuntimeExecutionState, execute_managed_mlx_lm, inspect_managed_mlx_lm, stop_managed_mlx_lm,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

const MODEL_ID: &str = "mlx-community/SmolLM3-3B-4bit";
const MODEL_REVISION: &str = "d3a7e0594d6642dbcfb7d149bed8b0bdf49f95ce";

#[derive(Clone, Debug)]
struct FixedProbe(Vec<String>);

impl LmStudioModelProbe for FixedProbe {
    fn models(&self, _: &str) -> Result<Vec<String>, RuntimeEndpointError> {
        Ok(self.0.clone())
    }
}

#[derive(Clone, Debug)]
struct RecordingSpawner {
    pid: u32,
    commands: Arc<Mutex<Vec<ProcessCommand>>>,
}

impl RecordingSpawner {
    fn new(pid: u32) -> Self {
        Self {
            pid,
            commands: Arc::default(),
        }
    }
}

impl ProcessSpawner for RecordingSpawner {
    fn spawn(&self, command: ProcessCommand) -> Result<u32, String> {
        self.commands.lock().expect("commands").push(command);
        Ok(self.pid)
    }
}

#[test]
fn managed_plan_is_exact_apple_silicon_and_private() {
    let temp = TempDir::new().expect("temp root");
    let plan = plan(&temp);

    assert_eq!(plan.artifact().version(), "0.12.0");
    assert_eq!(plan.python_version(), "3.14.6");
    assert_eq!(plan.mlx_lm_version(), "0.31.3");
    assert_eq!(
        plan.lock_sha256(),
        "60b758378744dd31603ccf389ada3b80a3a483d75be2b336dd0b4532d59568c6"
    );
    assert_eq!(plan.model_id(), MODEL_ID);
    assert_eq!(plan.model_revision(), MODEL_REVISION);
    assert_eq!(plan.model_license(), "apache-2.0");
    assert_eq!(plan.model_storage_bytes(), 1_747_260_812);
    assert_eq!(plan.endpoint(), "http://127.0.0.1:18080");
    assert_eq!(plan.health_retry_attempts(), 120);
    assert_eq!(plan.health_retry_delay(), std::time::Duration::from_secs(1));
    assert_eq!(
        plan.phases().last(),
        Some(&MlxLmManagedPhase::PersistOwnership)
    );
    assert!(plan.python_path().starts_with(plan.root()));
    assert!(plan.server_path().starts_with(plan.root()));
    assert!(plan.model_root().starts_with(plan.root()));
}

#[test]
fn managed_plan_fails_closed_for_unsupported_or_unsafe_inputs() {
    let temp = TempDir::new().expect("temp root");
    let root = managed_root(&temp);
    assert_eq!(
        MlxLmManagedPlan::new(
            &root,
            "linux-arm64",
            MODEL_ID,
            MODEL_REVISION,
            "apache-2.0",
            1,
            18080,
            true,
        ),
        Err(MlxLmManagedPlanError::UnsupportedPlatform)
    );
    assert_eq!(
        MlxLmManagedPlan::new(
            root.join("..").join("mlx-lm"),
            "darwin-arm64",
            MODEL_ID,
            MODEL_REVISION,
            "apache-2.0",
            1,
            18080,
            true,
        ),
        Err(MlxLmManagedPlanError::UnsafeManagedRoot)
    );
    assert_eq!(
        MlxLmManagedPlan::new(
            &root,
            "darwin-arm64",
            MODEL_ID,
            MODEL_REVISION,
            "apache-2.0",
            1,
            18080,
            false,
        ),
        Err(MlxLmManagedPlanError::ModelLicenseNotAccepted)
    );
    assert_eq!(
        MlxLmManagedPlan::new(
            root,
            "darwin-arm64",
            "../../escape",
            MODEL_REVISION,
            "apache-2.0",
            1,
            80,
            true,
        ),
        Err(MlxLmManagedPlanError::UnsafeModelIdentity)
    );
}

#[test]
fn execution_uses_locked_environment_exact_revision_and_loopback_server() {
    let temp = TempDir::new().expect("temp root");
    let plan = plan(&temp);
    let runner = successful_execution_runner();
    let spawner = RecordingSpawner::new(4242);
    let result = execute_managed_mlx_lm(
        &runner,
        &spawner,
        &FixedProbe(vec!["selected".to_string()]),
        &plan,
    );

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert!(result.evidence().contains("ownership=desktoplab_managed"));
    assert!(
        result.evidence().contains("venv") && result.evidence().contains("--allow-existing"),
        "managed MLX retries must preserve and re-verify an existing locked environment"
    );
    assert!(plan.lock_path().is_file());
    let hf_hub_cache = plan.cache_root().join("huggingface").join("hub");
    assert!(hf_hub_cache.is_dir());
    let commands = spawner.commands.lock().expect("commands");
    assert_eq!(commands.len(), 1);
    let command = &commands[0];
    assert_eq!(command.program(), plan.server_path().to_string_lossy());
    assert_eq!(
        command.args(),
        &[
            "--model",
            plan.model_root().to_string_lossy().as_ref(),
            "--host",
            "127.0.0.1",
            "--port",
            "18080",
        ]
        .map(ToString::to_string)
    );
    assert_eq!(
        command.env_value("UV_PYTHON_INSTALL_DIR"),
        Some(plan.python_root().to_string_lossy().as_ref())
    );
    assert_eq!(
        command.env_value("HF_HUB_CACHE"),
        Some(hf_hub_cache.to_string_lossy().as_ref())
    );
    assert_eq!(command.env_value("HF_HUB_DISABLE_TELEMETRY"), Some("1"));
}

#[test]
fn lock_mismatch_fails_before_network_or_process_start() {
    let temp = TempDir::new().expect("temp root");
    let plan = plan(&temp);
    let runner = DeterministicProcessRunner::succeeds("wrong-digest  environment.lock", "");
    let spawner = RecordingSpawner::new(4242);

    let result = execute_managed_mlx_lm(
        &runner,
        &spawner,
        &FixedProbe(vec!["selected".to_string()]),
        &plan,
    );

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "lock_verification_failed");
    assert!(spawner.commands.lock().expect("commands").is_empty());
}

#[test]
fn recovery_and_stop_require_matching_owned_process() {
    let temp = TempDir::new().expect("temp root");
    let plan = plan(&temp);
    let spawner = RecordingSpawner::new(4242);
    let installed = execute_managed_mlx_lm(
        &successful_execution_runner(),
        &spawner,
        &FixedProbe(vec!["selected".to_string()]),
        &plan,
    );
    assert_eq!(installed.state(), RuntimeExecutionState::Completed);

    let process = format!(
        "{} --model {} --host 127.0.0.1 --port 18080",
        plan.server_path().display(),
        plan.model_root().display()
    );
    let status_runner = DeterministicProcessRunner::sequence(vec![(Some(0), process.as_str(), "")]);
    let status = inspect_managed_mlx_lm(
        &status_runner,
        &FixedProbe(vec!["selected".to_string()]),
        &plan,
    )
    .expect("managed status");
    assert_eq!(status.state(), MlxLmManagedState::Ready);
    assert_eq!(status.ownership(), "desktoplab_managed");

    let stale = DeterministicProcessRunner::succeeds("/usr/bin/python other-server", "");
    assert_eq!(
        stop_managed_mlx_lm(&stale, &plan).state(),
        RuntimeExecutionState::Blocked
    );
    assert!(plan.marker_path().is_file());

    let stop = DeterministicProcessRunner::sequence(vec![
        (Some(0), process.as_str(), ""),
        (Some(0), "", ""),
    ]);
    assert_eq!(
        stop_managed_mlx_lm(&stop, &plan).state(),
        RuntimeExecutionState::Completed
    );
    assert!(!plan.marker_path().exists());
}

#[test]
fn managed_mlx_sources_stay_focused() {
    for (path, source, limit) in [
        (
            "crates/desktoplab-runtime/src/mlx_lm_managed_plan.rs",
            include_str!("../src/mlx_lm_managed_plan.rs"),
            280,
        ),
        (
            "crates/desktoplab-runtime/src/mlx_lm_managed_execution.rs",
            include_str!("../src/mlx_lm_managed_execution.rs"),
            280,
        ),
        (
            "crates/desktoplab-runtime/src/mlx_lm_managed_lifecycle.rs",
            include_str!("../src/mlx_lm_managed_lifecycle.rs"),
            130,
        ),
        (
            "crates/desktoplab-runtime/src/mlx_lm_managed_marker.rs",
            include_str!("../src/mlx_lm_managed_marker.rs"),
            150,
        ),
        (
            "crates/desktoplab-runtime/src/process_spawn.rs",
            include_str!("../src/process_spawn.rs"),
            70,
        ),
    ] {
        check_logical_line_limit(path, source, limit).expect("managed MLX module stays focused");
    }
}

fn plan(temp: &TempDir) -> MlxLmManagedPlan {
    MlxLmManagedPlan::new(
        managed_root(temp),
        "darwin-arm64",
        MODEL_ID,
        MODEL_REVISION,
        "apache-2.0",
        1_747_260_812,
        18080,
        true,
    )
    .expect("valid managed plan")
}

fn managed_root(temp: &TempDir) -> std::path::PathBuf {
    temp.path()
        .join("DesktopLab")
        .join("runtimes")
        .join("mlx-lm")
}

fn successful_execution_runner() -> DeterministicProcessRunner {
    DeterministicProcessRunner::sequence(vec![
        (
            Some(0),
            "60b758378744dd31603ccf389ada3b80a3a483d75be2b336dd0b4532d59568c6  environment.lock",
            "",
        ),
        (Some(0), "", ""),
        (
            Some(0),
            "2b9e582af54f84fa50c115427451a6c13e80f43b52f8282b8af5791077317bbf  uv.tar.gz",
            "",
        ),
        (Some(0), "", ""),
        (Some(0), "", ""),
        (Some(0), "", ""),
        (Some(0), "", ""),
        (Some(0), "0.31.3", ""),
        (Some(0), "", ""),
    ])
}
