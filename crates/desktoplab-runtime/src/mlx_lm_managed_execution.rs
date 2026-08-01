use std::fs;

use crate::{
    LmStudioModelProbe, MlxLmManagedPlan, ProcessCommand, ProcessRunner, ProcessSpawner,
    RuntimeInstallExecutionResult,
    mlx_lm_managed_marker::{MlxLmOwnershipMarker, write_marker},
};

#[must_use]
pub fn execute_managed_mlx_lm(
    runner: &impl ProcessRunner,
    spawner: &impl ProcessSpawner,
    model_probe: &impl LmStudioModelProbe,
    plan: &MlxLmManagedPlan,
) -> RuntimeInstallExecutionResult {
    if let Some(recovered) =
        crate::mlx_lm_managed_lifecycle::recover_managed_mlx_lm(runner, model_probe, plan)
    {
        return recovered;
    }
    if prepare_environment(plan).is_err() {
        return failed(
            "managed_root_unavailable",
            "mlx-lm managed root preparation failed",
            "DesktopLab could not prepare its private MLX-LM runtime directory.",
        );
    }
    let lock = runner.run(
        ProcessCommand::new("shasum")
            .arg("-a")
            .arg("256")
            .arg(plan.lock_path().to_string_lossy()),
    );
    let mut evidence = lock.evidence().evidence();
    if !lock.succeeded() || lock.stdout().split_whitespace().next() != Some(plan.lock_sha256()) {
        return failed(
            "lock_verification_failed",
            evidence,
            "The embedded MLX-LM environment lock did not match its trusted checksum.",
        );
    }
    let download = runner.run(
        ProcessCommand::new("curl")
            .arg("--fail")
            .arg("--location")
            .arg("--output")
            .arg(plan.archive_path().to_string_lossy())
            .arg(plan.artifact().url()),
    );
    evidence = format!(
        "{evidence}; {}; source={}; uv={}",
        download.evidence().evidence(),
        plan.artifact().url(),
        plan.artifact().version()
    );
    if !download.succeeded() {
        return failed(
            "bootstrap_download_failed",
            evidence,
            "The pinned uv bootstrap artifact could not be downloaded.",
        );
    }
    let verify = runner.run(
        ProcessCommand::new("shasum")
            .arg("-a")
            .arg("256")
            .arg(plan.archive_path().to_string_lossy()),
    );
    evidence = append(evidence, &verify);
    if !verify.succeeded()
        || verify.stdout().split_whitespace().next() != Some(plan.artifact().sha256())
    {
        return failed(
            "bootstrap_verification_failed",
            evidence,
            "The uv bootstrap archive did not match DesktopLab's trusted checksum.",
        );
    }
    let extract = runner.run(
        ProcessCommand::new("tar")
            .arg("-xzf")
            .arg(plan.archive_path().to_string_lossy())
            .arg("-C")
            .arg(plan.bootstrap_root().to_string_lossy()),
    );
    evidence = append(evidence, &extract);
    if !extract.succeeded() {
        return failed(
            "bootstrap_extract_failed",
            evidence,
            "The verified uv archive could not be extracted.",
        );
    }
    for command in environment_commands(plan) {
        let verifies_version = command
            .args()
            .iter()
            .any(|argument| argument.contains("importlib.metadata"));
        let output = runner.run(managed(command, plan));
        evidence = append(evidence, &output);
        if !output.succeeded()
            || (verifies_version && output.stdout().trim() != plan.mlx_lm_version())
        {
            return failed(
                "environment_setup_failed",
                evidence,
                "The isolated Python or locked MLX-LM environment could not be created.",
            );
        }
    }
    let model = runner.run(managed(model_download_command(plan), plan));
    evidence = append(evidence, &model);
    if !model.succeeded() {
        return failed(
            "model_download_failed",
            evidence,
            "The exact MLX model revision could not be acquired from Hugging Face.",
        );
    }
    let pid = match spawner.spawn(managed(server_command(plan), plan)) {
        Ok(pid) => pid,
        Err(_) => {
            return failed(
                "server_start_failed",
                evidence,
                "The DesktopLab-managed MLX-LM server could not be started.",
            );
        }
    };
    let Some(_models) = wait_for_models(model_probe, plan) else {
        stop_new_process(runner, pid);
        return failed(
            "health_failed_retryable",
            evidence,
            "The managed MLX-LM loopback endpoint did not answer /v1/models.",
        );
    };
    if write_marker(&plan.marker_path(), &MlxLmOwnershipMarker::new(plan, pid)).is_err() {
        stop_new_process(runner, pid);
        return failed(
            "ownership_persistence_failed",
            evidence,
            "DesktopLab could not persist MLX-LM ownership evidence.",
        );
    }
    RuntimeInstallExecutionResult::completed_after_desktoplab_start(format!(
        "{evidence}; ownership=desktoplab_managed; endpoint={}; model={}; revision={}",
        plan.endpoint(),
        plan.model_id(),
        plan.model_revision()
    ))
}

fn wait_for_models(
    model_probe: &impl LmStudioModelProbe,
    plan: &MlxLmManagedPlan,
) -> Option<Vec<String>> {
    for attempt in 0..30 {
        if let Ok(models) = model_probe.models(&plan.endpoint()) {
            if !models.is_empty() {
                return Some(models);
            }
        }
        if attempt < 29 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    None
}

fn stop_new_process(runner: &impl ProcessRunner, pid: u32) {
    let _ = runner.run(
        ProcessCommand::new("kill")
            .arg("-TERM")
            .arg(pid.to_string()),
    );
}

fn prepare_environment(plan: &MlxLmManagedPlan) -> std::io::Result<()> {
    fs::create_dir_all(plan.archive_path().parent().unwrap_or(plan.root()))?;
    fs::create_dir_all(plan.bootstrap_root())?;
    fs::create_dir_all(plan.model_root())?;
    fs::create_dir_all(plan.cache_root().join("huggingface").join("hub"))?;
    fs::write(plan.lock_path(), plan.lock_content())
}

fn environment_commands(plan: &MlxLmManagedPlan) -> Vec<ProcessCommand> {
    vec![
        ProcessCommand::new(plan.uv_path().to_string_lossy())
            .arg("python")
            .arg("install")
            .arg(plan.python_version())
            .arg("--install-dir")
            .arg(plan.python_root().to_string_lossy()),
        ProcessCommand::new(plan.uv_path().to_string_lossy())
            .arg("venv")
            .arg(plan.environment_root().to_string_lossy())
            .arg("--python")
            .arg(plan.python_version())
            .arg("--allow-existing"),
        ProcessCommand::new(plan.uv_path().to_string_lossy())
            .arg("pip")
            .arg("sync")
            .arg(plan.lock_path().to_string_lossy())
            .arg("--require-hashes")
            .arg("--python")
            .arg(plan.python_path().to_string_lossy()),
        ProcessCommand::new(plan.python_path().to_string_lossy())
            .arg("-c")
            .arg("import importlib.metadata as m, mlx_lm; print(m.version('mlx-lm'))"),
    ]
}

fn model_download_command(plan: &MlxLmManagedPlan) -> ProcessCommand {
    ProcessCommand::new(plan.hf_path().to_string_lossy())
        .arg("download")
        .arg(plan.model_id())
        .arg("--revision")
        .arg(plan.model_revision())
        .arg("--local-dir")
        .arg(plan.model_root().to_string_lossy())
}

fn server_command(plan: &MlxLmManagedPlan) -> ProcessCommand {
    ProcessCommand::new(plan.server_path().to_string_lossy())
        .arg("--model")
        .arg(plan.model_root().to_string_lossy())
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(plan.port().to_string())
}

pub(crate) fn managed(command: ProcessCommand, plan: &MlxLmManagedPlan) -> ProcessCommand {
    command
        .with_env(
            "UV_PYTHON_INSTALL_DIR",
            plan.python_root().to_string_lossy(),
        )
        .with_env(
            "UV_CACHE_DIR",
            plan.cache_root().join("uv").to_string_lossy(),
        )
        .with_env(
            "HF_HOME",
            plan.cache_root().join("huggingface").to_string_lossy(),
        )
        .with_env(
            "HF_HUB_CACHE",
            plan.cache_root()
                .join("huggingface")
                .join("hub")
                .to_string_lossy(),
        )
        .with_env("HF_HUB_DISABLE_TELEMETRY", "1")
}

fn append(evidence: String, output: &crate::ProcessOutput) -> String {
    format!("{evidence}; {}", output.evidence().evidence())
}

fn failed(
    state: &str,
    evidence: impl Into<String>,
    remediation: &str,
) -> RuntimeInstallExecutionResult {
    RuntimeInstallExecutionResult::failed(state, evidence, remediation)
}
