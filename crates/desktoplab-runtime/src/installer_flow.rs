use std::{fs, path::PathBuf};

use crate::{InstallPlan, ProcessCommand, ProcessRunner, RuntimeInstallExecutionResult};

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
    let Ok(target) = prepare_installer_cache_path("ollama", "Ollama-darwin-arm64.dmg") else {
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

    let verify = runner.run(
        ProcessCommand::new("spctl")
            .arg("--assess")
            .arg("--type")
            .arg("open")
            .arg("--context")
            .arg("context:primary-signature")
            .arg(target.to_string_lossy()),
    );
    evidence = format!("{evidence}; {}", verify.evidence().evidence());
    if !verify.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "installer_verification_failed",
            evidence,
            "The downloaded Ollama installer could not be verified. DesktopLab will not install it.",
        );
    }

    let attach = runner.run(
        ProcessCommand::new("hdiutil")
            .arg("attach")
            .arg(target.to_string_lossy())
            .arg("-nobrowse")
            .arg("-quiet"),
    );
    evidence = format!("{evidence}; {}", attach.evidence().evidence());
    if !attach.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "install_failed_retryable",
            evidence,
            "Ollama installer could not be mounted. Check macOS permissions and retry.",
        );
    }

    let mount_point = macos_ollama_mount_point(attach.stdout());
    let source_app = PathBuf::from(&mount_point).join("Ollama.app");
    let install = runner.run(
        ProcessCommand::new("ditto")
            .arg(source_app.to_string_lossy())
            .arg("/Applications/Ollama.app"),
    );
    evidence = format!("{evidence}; {}", install.evidence().evidence());
    if !install.succeeded() {
        let detach = runner.run(
            ProcessCommand::new("hdiutil")
                .arg("detach")
                .arg(mount_point.clone())
                .arg("-quiet"),
        );
        evidence = format!("{evidence}; {}", detach.evidence().evidence());
        return RuntimeInstallExecutionResult::failed(
            "install_failed_retryable",
            evidence,
            "Ollama could not be copied into Applications. Check macOS permissions and retry.",
        );
    }

    let detach = runner.run(
        ProcessCommand::new("hdiutil")
            .arg("detach")
            .arg(mount_point)
            .arg("-quiet"),
    );
    evidence = format!("{evidence}; {}", detach.evidence().evidence());
    if !detach.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "install_failed_retryable",
            evidence,
            "Ollama installer could not be closed cleanly. Retry setup before starting Ollama.",
        );
    }

    let start = runner.run(ProcessCommand::new("open").arg("-a").arg("Ollama"));
    evidence = format!("{evidence}; {}", start.evidence().evidence());
    if !start.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "start_failed_retryable",
            evidence,
            "Ollama could not be started. Open Ollama manually or retry from DesktopLab.",
        );
    }

    let health = runner.run(
        ProcessCommand::new("curl")
            .arg("--fail")
            .arg("http://127.0.0.1:11434/api/tags"),
    );
    evidence = format!("{evidence}; {}", health.evidence().evidence());
    if !health.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "health_failed_retryable",
            evidence,
            "Ollama started but its local API is not ready yet. Retry after it finishes launching.",
        );
    }

    RuntimeInstallExecutionResult::completed_after_desktoplab_start(evidence)
}

pub(crate) fn run_linux_ollama_install<R>(
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
    let Some(expected_digest) = trusted_sha256_digest(source.checksum()) else {
        return RuntimeInstallExecutionResult::blocked_with_state(
            "installer_integrity_metadata_missing",
            format!("{detect_evidence}; installer_source={}", source.url()),
            "DesktopLab does not have a trusted Linux installer digest for this Ollama release. Install Ollama manually or wait for a signed catalog update.",
        );
    };
    let Ok(target) = prepare_installer_cache_path("ollama", "ollama-install-linux.sh") else {
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
    let mut evidence = format!(
        "{detect_evidence}; {}; cache={}; checksum={}",
        download.evidence().evidence(),
        target.display(),
        source.checksum()
    );
    if !download.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "download_failed_retryable",
            evidence,
            "Network connection failed while downloading Ollama. Check the connection and retry.",
        );
    }

    let verify = runner.run(ProcessCommand::new("sha256sum").arg(target.to_string_lossy()));
    evidence = format!("{evidence}; {}", verify.evidence().evidence());
    if !verify.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "installer_verification_failed",
            evidence,
            "The downloaded Ollama installer could not be verified. DesktopLab will not run it.",
        );
    }
    let actual_digest = verify
        .stdout()
        .split_whitespace()
        .next()
        .unwrap_or_default();
    if actual_digest != expected_digest {
        return RuntimeInstallExecutionResult::failed(
            "installer_verification_failed",
            format!("{evidence}; expected_sha256={expected_digest}; actual_sha256={actual_digest}"),
            "The downloaded Ollama installer digest did not match trusted metadata. DesktopLab will not run it.",
        );
    }

    RuntimeInstallExecutionResult::blocked_with_state(
        "requires_admin_action",
        evidence,
        "Ollama Linux installation requires administrator approval. Review the verified installer before continuing.",
    )
}

fn trusted_sha256_digest(checksum: &str) -> Option<&str> {
    checksum
        .strip_prefix("sha256:")
        .filter(|digest| digest.len() >= 32 && digest.chars().all(|c| c.is_ascii_hexdigit()))
}

pub(crate) fn prepare_installer_cache_path(
    runtime: &str,
    file_name: &str,
) -> std::io::Result<PathBuf> {
    let target = installer_cache_root()
        .join("runtime-installers")
        .join(runtime)
        .join(file_name);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(target)
}

fn installer_cache_root() -> PathBuf {
    std::env::var_os("DESKTOPLAB_RUNTIME_INSTALLER_CACHE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("desktoplab"))
}

fn macos_ollama_mount_point(output: &str) -> String {
    output
        .lines()
        .rev()
        .find_map(|line| line.split_whitespace().last())
        .filter(|value| value.starts_with("/Volumes/"))
        .unwrap_or("/Volumes/Ollama")
        .to_string()
}
