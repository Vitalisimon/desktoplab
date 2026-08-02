use std::{fs, path::Path};

use crate::{
    InstallPlan, ProcessCommand, ProcessRunner, RuntimeInstallExecutionResult,
    installer_flow::prepare_installer_cache_path,
};

pub(crate) fn run_macos_ollama_install<R>(
    runner: &R,
    plan: &InstallPlan,
    detect_evidence: &str,
) -> RuntimeInstallExecutionResult
where
    R: ProcessRunner,
{
    let Some(source) = plan.installer_source() else {
        return RuntimeInstallExecutionResult::blocked(
            detect_evidence,
            "Ollama installer metadata is missing, so DesktopLab cannot download it safely.",
        );
    };
    let Ok(target) = prepare_installer_cache_path("ollama", "Ollama-darwin-arm64.zip") else {
        return RuntimeInstallExecutionResult::failed(
            "installer_cache_unavailable",
            detect_evidence,
            "DesktopLab could not prepare the local installer cache. Check disk permissions and retry.",
        );
    };
    let download = runner.run(
        ProcessCommand::new("curl")
            .arg("--fail")
            .arg("--location")
            .arg("--output")
            .arg(target.to_string_lossy())
            .arg(source.url()),
    );
    let trust = format!(
        "checksum={} signature={}",
        source.checksum(),
        source.signature().unwrap_or("missing")
    );
    let mut evidence = format!(
        "{detect_evidence}; {}; cache={}; {trust}",
        download.evidence().evidence(),
        target.display()
    );
    if !download.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "download_failed_retryable",
            evidence,
            "Network connection failed while downloading Ollama. Check the connection and retry.",
        );
    }

    let extraction = target
        .parent()
        .expect("installer cache target should have a parent")
        .join("extracted");
    if let Err(error) = reset_extraction(&extraction) {
        return RuntimeInstallExecutionResult::failed(
            "installer_cache_unavailable",
            format!("{evidence}; extraction_error={error}"),
            "DesktopLab could not prepare the Ollama archive extraction directory. Check disk permissions and retry.",
        );
    }
    let unpack = runner.run(
        ProcessCommand::new("unzip")
            .arg("-q")
            .arg(target.to_string_lossy())
            .arg("-d")
            .arg(extraction.to_string_lossy()),
    );
    evidence = format!("{evidence}; {}", unpack.evidence().evidence());
    if !unpack.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "install_failed_retryable",
            evidence,
            "The Ollama archive could not be extracted. Check disk permissions and retry.",
        );
    }

    let source_app = extraction.join("Ollama.app");
    for verification in macos_verifications(&source_app) {
        let result = runner.run(verification);
        evidence = format!("{evidence}; {}", result.evidence().evidence());
        if !result.succeeded() {
            return RuntimeInstallExecutionResult::failed(
                "installer_verification_failed",
                evidence,
                "The extracted Ollama app failed macOS verification. DesktopLab will not install it.",
            );
        }
    }

    let target_available = runner.run(
        ProcessCommand::new("test")
            .arg("!")
            .arg("-e")
            .arg("/Applications/Ollama.app"),
    );
    evidence = format!("{evidence}; {}", target_available.evidence().evidence());
    if !target_available.succeeded() {
        return RuntimeInstallExecutionResult::blocked_with_state(
            "existing_runtime_preserved",
            evidence,
            "Ollama already exists in Applications. Keep the current local setup, or remove it explicitly before requesting a fresh managed install.",
        );
    }
    let install = runner.run(
        ProcessCommand::new("mv")
            .arg("-n")
            .arg(source_app.to_string_lossy())
            .arg("/Applications/Ollama.app"),
    );
    evidence = format!("{evidence}; {}", install.evidence().evidence());
    if !install.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "install_failed_retryable",
            evidence,
            "Ollama could not be moved into Applications. Check macOS permissions and retry.",
        );
    }
    let source_moved = runner.run(
        ProcessCommand::new("test")
            .arg("!")
            .arg("-e")
            .arg(source_app.to_string_lossy()),
    );
    evidence = format!("{evidence}; {}", source_moved.evidence().evidence());
    if !source_moved.succeeded() {
        return RuntimeInstallExecutionResult::blocked_with_state(
            "existing_runtime_preserved",
            evidence,
            "Another Ollama app appeared in Applications during installation. DesktopLab preserved it and did not claim ownership.",
        );
    }

    for verification in macos_verifications(Path::new("/Applications/Ollama.app")) {
        let result = runner.run(verification);
        evidence = format!("{evidence}; {}", result.evidence().evidence());
        if !result.succeeded() {
            let quarantine = runner.run(
                ProcessCommand::new("mv")
                    .arg("/Applications/Ollama.app")
                    .arg(extraction.join("rejected-Ollama.app").to_string_lossy()),
            );
            evidence = format!("{evidence}; {}", quarantine.evidence().evidence());
            return RuntimeInstallExecutionResult::failed(
                "installed_runtime_verification_failed",
                evidence,
                "The installed Ollama app failed macOS verification and was moved back into DesktopLab's private cache.",
            );
        }
    }

    let start = runner.run(ProcessCommand::new("open").arg("/Applications/Ollama.app"));
    evidence = format!("{evidence}; {}", start.evidence().evidence());
    if !start.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "start_failed_retryable",
            evidence,
            "Ollama could not be started. Open Ollama manually or retry from DesktopLab.",
        );
    }

    let (health_ready, health_evidence) = wait_for_health(runner, evidence);
    if !health_ready {
        return RuntimeInstallExecutionResult::failed(
            "health_failed_retryable",
            health_evidence,
            "Ollama started but its local API is not ready yet. Retry after it finishes launching.",
        );
    }
    RuntimeInstallExecutionResult::completed_after_desktoplab_start(health_evidence)
}

fn macos_verifications(app: &Path) -> [ProcessCommand; 2] {
    [
        ProcessCommand::new("codesign")
            .arg("--verify")
            .arg("--deep")
            .arg("--strict")
            .arg(app.to_string_lossy()),
        ProcessCommand::new("spctl")
            .arg("--assess")
            .arg("--type")
            .arg("execute")
            .arg(app.to_string_lossy()),
    ]
}

fn wait_for_health<R: ProcessRunner>(runner: &R, mut evidence: String) -> (bool, String) {
    for attempt in 1..=30 {
        let health = runner.run(
            ProcessCommand::new("curl")
                .arg("--fail")
                .arg("http://127.0.0.1:11434/api/tags"),
        );
        evidence = format!("{evidence}; {}", health.evidence().evidence());
        if health.succeeded() {
            return (true, evidence);
        }
        if attempt < 30 {
            let wait = runner.run(ProcessCommand::new("sleep").arg("1"));
            evidence = format!("{evidence}; {}", wait.evidence().evidence());
            if !wait.succeeded() {
                return (false, evidence);
            }
        }
    }
    (false, evidence)
}

fn reset_extraction(path: &Path) -> std::io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    fs::create_dir_all(path)
}
