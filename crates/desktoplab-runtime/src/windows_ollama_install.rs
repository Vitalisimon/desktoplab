use crate::{InstallPlan, ProcessCommand, ProcessRunner, RuntimeInstallExecutionResult};
use std::path::Path;

const VERIFY_AUTHENTICODE: &str = concat!(
    "$signature = Get-AuthenticodeSignature -LiteralPath $installerPath; ",
    "$subject = $signature.SignerCertificate.Subject; ",
    "if ($signature.Status -ne 'Valid' -or $subject -notmatch '(?i)Ollama') { ",
    "Write-Error ('Untrusted Ollama installer: status=' + $signature.Status + '; subject=' + $subject); exit 1 }; ",
    "Write-Output ('Status=Valid; Subject=' + $subject)"
);
const START_OLLAMA: &str = concat!(
    "$ollama = Join-Path $env:LOCALAPPDATA 'Programs\\Ollama\\ollama.exe'; ",
    "if (-not (Test-Path -LiteralPath $ollama)) { Write-Error 'Ollama binary missing'; exit 1 }; ",
    "Start-Process -FilePath $ollama -ArgumentList 'serve' -WindowStyle Hidden"
);
const WAIT_FOR_HEALTH: &str = concat!(
    "for ($attempt = 0; $attempt -lt 60; $attempt++) { ",
    "try { Invoke-WebRequest -Uri 'http://127.0.0.1:11434/api/tags' -UseBasicParsing -TimeoutSec 2 | Out-Null; exit 0 } ",
    "catch { Start-Sleep -Milliseconds 500 } }; exit 1"
);

pub(crate) fn run_windows_ollama_install<R>(
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
    let Ok(target) =
        crate::installer_flow::prepare_installer_cache_path("ollama", "OllamaSetup.exe")
    else {
        return RuntimeInstallExecutionResult::failed(
            "installer_cache_unavailable",
            detect_evidence,
            "DesktopLab could not prepare the local installer cache. Check disk permissions and retry.",
        );
    };

    let download = runner.run(
        ProcessCommand::new("curl.exe")
            .arg("--fail")
            .arg("--location")
            .arg("--output")
            .arg(target.to_string_lossy())
            .arg(source.url()),
    );
    let mut evidence = format!(
        "{detect_evidence}; {}; cache={}; signature={}",
        download.evidence().evidence(),
        target.display(),
        source.signature().unwrap_or("missing")
    );
    if !download.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "download_failed_retryable",
            evidence,
            "Network connection failed while downloading Ollama. Check the connection and retry.",
        );
    }

    let verify = runner.run(authenticode_verification(&target));
    evidence = format!("{evidence}; {}", verify.evidence().evidence());
    if !verify.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "installer_verification_failed",
            evidence,
            "The downloaded Ollama installer does not have a valid Ollama Authenticode signature. DesktopLab will not run it.",
        );
    }

    let install = runner.run(
        ProcessCommand::new(target.to_string_lossy())
            .arg("/VERYSILENT")
            .arg("/SUPPRESSMSGBOXES")
            .arg("/NORESTART")
            .arg("/SP-"),
    );
    evidence = format!("{evidence}; {}", install.evidence().evidence());
    if !install.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "install_failed_retryable",
            evidence,
            "Ollama installation failed. Check Windows installer permissions and retry.",
        );
    }

    let binary = runner.run(ProcessCommand::new("ollama").arg("--version"));
    evidence = format!("{evidence}; {}", binary.evidence().evidence());
    if !binary.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "installer_verification_failed",
            evidence,
            "Ollama installed but its command-line binary could not be verified.",
        );
    }

    let start = runner.run(powershell(START_OLLAMA));
    evidence = format!("{evidence}; {}", start.evidence().evidence());
    if !start.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "start_failed_retryable",
            evidence,
            "Ollama installed but could not be started. Retry from DesktopLab.",
        );
    }

    let health = runner.run(powershell(WAIT_FOR_HEALTH));
    evidence = format!("{evidence}; {}", health.evidence().evidence());
    if !health.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "health_failed_retryable",
            evidence,
            "Ollama started but its local API did not become ready. Check Ollama logs and retry.",
        );
    }

    RuntimeInstallExecutionResult::completed_after_desktoplab_start(evidence)
}

fn powershell(command: &str) -> ProcessCommand {
    ProcessCommand::new("powershell.exe")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command)
}

fn authenticode_verification(installer_path: &Path) -> ProcessCommand {
    let escaped_path = installer_path.to_string_lossy().replace('\'', "''");
    powershell(&format!(
        "$installerPath = '{escaped_path}'; {VERIFY_AUTHENTICODE}"
    ))
}
