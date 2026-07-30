use std::fs;

use serde_json::Value;

use crate::{
    LmStudioManagedPlan, LmStudioModelProbe, ProcessCommand, ProcessRunner,
    RuntimeInstallExecutionResult,
    lm_studio_managed_marker::{LmStudioOwnershipMarker, write_marker},
};

#[must_use]
pub fn execute_managed_lm_studio(
    runner: &impl ProcessRunner,
    model_probe: &impl LmStudioModelProbe,
    plan: &LmStudioManagedPlan,
) -> RuntimeInstallExecutionResult {
    if let Some(recovered) =
        crate::lm_studio_managed_lifecycle::recover_managed_lm_studio(runner, model_probe, plan)
    {
        return recovered;
    }
    if prepare_directories(plan).is_err() {
        return RuntimeInstallExecutionResult::failed(
            "managed_root_unavailable",
            "lm-studio managed root preparation failed",
            "DesktopLab could not prepare its private LM Studio runtime directory.",
        );
    }
    let download = runner.run(
        ProcessCommand::new("curl")
            .arg("--fail")
            .arg("--location")
            .arg("--output")
            .arg(plan.cache_archive().to_string_lossy())
            .arg(plan.artifact().url()),
    );
    let mut evidence = format!(
        "{}; source={}; version={}",
        download.evidence().evidence(),
        plan.artifact().url(),
        plan.artifact().version()
    );
    if !download.succeeded() {
        return failed(
            "download_failed_retryable",
            evidence,
            "The official llmster artifact could not be downloaded.",
        );
    }
    let verify = runner.run(
        ProcessCommand::new("shasum")
            .arg("-a")
            .arg("512")
            .arg(plan.cache_archive().to_string_lossy()),
    );
    evidence = format!("{evidence}; {}", verify.evidence().evidence());
    let actual = verify
        .stdout()
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if !verify.succeeded() || actual != plan.artifact().sha512() {
        return failed(
            "artifact_verification_failed",
            evidence,
            "The llmster archive did not match DesktopLab's trusted vendor checksum.",
        );
    }
    let extract = runner.run(
        ProcessCommand::new("tar")
            .arg("-xzf")
            .arg(plan.cache_archive().to_string_lossy())
            .arg("-C")
            .arg(plan.extract_root().to_string_lossy()),
    );
    evidence = format!("{evidence}; {}", extract.evidence().evidence());
    if !extract.succeeded() {
        return failed(
            "install_failed_retryable",
            evidence,
            "The verified llmster archive could not be extracted.",
        );
    }
    let bootstrap = runner.run(managed(
        ProcessCommand::new(plan.bootstrap_path().to_string_lossy())
            .arg("bootstrap")
            .with_env("LMS_BOOTSTRAP_INSTALL_SH", "1"),
        plan,
    ));
    evidence = format!("{evidence}; {}", bootstrap.evidence().evidence());
    if !bootstrap.succeeded() {
        return failed(
            "install_failed_retryable",
            evidence,
            "llmster could not be installed into DesktopLab's private runtime home.",
        );
    }
    let cli = runner.run(managed(
        ProcessCommand::new(plan.lms_path().to_string_lossy()).arg("--help"),
        plan,
    ));
    evidence = format!("{evidence}; {}", cli.evidence().evidence());
    if !cli.succeeded() {
        return failed(
            "cli_verification_failed",
            evidence,
            "The managed lms CLI was not available after installation.",
        );
    }
    let daemon = runner.run(managed(
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("daemon")
            .arg("up")
            .arg("--json"),
        plan,
    ));
    evidence = format!("{evidence}; {}", daemon.evidence().evidence());
    let Some(pid) = managed_daemon_pid(&daemon) else {
        return failed(
            "daemon_start_failed",
            evidence,
            "The managed llmster daemon did not report a DesktopLab-owned process.",
        );
    };
    for command in model_and_server_commands(plan) {
        let output = runner.run(managed(command, plan));
        evidence = format!("{evidence}; {}", output.evidence().evidence());
        if !output.succeeded() {
            return failed(
                "runtime_setup_failed_retryable",
                evidence,
                "LM Studio model or loopback server setup did not complete.",
            );
        }
    }
    let Ok(models) = model_probe.models(&plan.endpoint()) else {
        return failed(
            "health_failed_retryable",
            evidence,
            "The managed LM Studio endpoint did not return a valid model inventory.",
        );
    };
    if !models.iter().any(|model| model == plan.api_model_id()) {
        return failed(
            "model_not_ready",
            evidence,
            "The selected model was not ready at the managed LM Studio endpoint.",
        );
    }
    if write_marker(plan.marker_path(), &LmStudioOwnershipMarker::new(plan, pid)).is_err() {
        return failed(
            "ownership_persistence_failed",
            evidence,
            "DesktopLab could not persist managed-runtime ownership evidence.",
        );
    }
    RuntimeInstallExecutionResult::completed_after_desktoplab_start(format!(
        "{evidence}; ownership=desktoplab_managed; endpoint={}",
        plan.endpoint()
    ))
}

fn prepare_directories(plan: &LmStudioManagedPlan) -> std::io::Result<()> {
    fs::create_dir_all(plan.cache_archive().parent().unwrap_or(plan.root()))?;
    fs::create_dir_all(plan.extract_root())?;
    fs::create_dir_all(plan.managed_home())
}

fn model_and_server_commands(plan: &LmStudioManagedPlan) -> Vec<ProcessCommand> {
    vec![
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("get")
            .arg(plan.model_id()),
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("load")
            .arg(plan.model_id())
            .arg("--identifier")
            .arg(plan.api_model_id())
            .arg("--context-length")
            .arg("32768")
            .arg("--ttl")
            .arg("3600"),
        ProcessCommand::new(plan.lms_path().to_string_lossy())
            .arg("server")
            .arg("start")
            .arg("--port")
            .arg(plan.port().to_string())
            .arg("--bind")
            .arg("127.0.0.1"),
    ]
}

pub(crate) fn managed(command: ProcessCommand, plan: &LmStudioManagedPlan) -> ProcessCommand {
    command
        .with_env("HOME", plan.managed_home().to_string_lossy())
        .with_env("LMS_NO_MODIFY_PATH", "1")
}

pub(crate) fn managed_daemon_pid(output: &crate::ProcessOutput) -> Option<u64> {
    output
        .succeeded()
        .then(|| serde_json::from_str::<Value>(output.stdout()).ok())
        .flatten()
        .filter(|value| value.get("status").and_then(Value::as_str) == Some("running"))
        .filter(|value| value.get("isDaemon").and_then(Value::as_bool) == Some(true))
        .and_then(|value| value.get("pid").and_then(Value::as_u64))
}

fn failed(
    state: &str,
    evidence: impl Into<String>,
    remediation: &str,
) -> RuntimeInstallExecutionResult {
    RuntimeInstallExecutionResult::failed(state, evidence, remediation)
}
