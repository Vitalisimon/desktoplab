use std::cell::RefCell;

use desktoplab_runtime::{
    OllamaRuntime, ProcessCommand, ProcessOutput, ProcessRunner, RuntimeExecutionState,
    RuntimeInstallExecutor,
};

#[derive(Clone, Debug)]
struct ScriptedResponse {
    exit_code: Option<i32>,
    stdout: &'static str,
    stderr: &'static str,
}

#[derive(Debug)]
struct ScriptedRunner {
    responses: RefCell<Vec<ScriptedResponse>>,
}

impl ScriptedRunner {
    fn new(responses: Vec<ScriptedResponse>) -> Self {
        Self {
            responses: RefCell::new(responses),
        }
    }
}

impl ProcessRunner for ScriptedRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        let response = self.responses.borrow_mut().remove(0);
        ProcessOutput::new(
            response.exit_code,
            response.stdout,
            response.stderr,
            command,
        )
    }
}

#[test]
fn verified_macos_download_installs_starts_and_health_checks_ollama() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let mut responses = install_and_start_responses();
    responses.push(response(0, r#"{"models":[]}"#, ""));
    let runner = ScriptedRunner::new(responses);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(
        result
            .evidence()
            .contains("codesign --verify --deep --strict")
    );
    assert!(result.evidence().contains("spctl --assess --type execute"));
    assert!(result.evidence().contains("unzip"));
    assert!(result.evidence().contains("mv"));
    assert!(
        result.evidence().contains("open /Applications/Ollama.app"),
        "fresh installs must launch the exact copied bundle without waiting for LaunchServices name registration"
    );
    assert!(
        result
            .evidence()
            .contains("http://127.0.0.1:11434/api/tags")
    );
}

#[test]
fn slow_macos_start_retries_health_before_recording_desktoplab_ownership() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let mut responses = install_and_start_responses();
    responses.extend([
        response(7, "", "connection refused"),
        response(0, "", ""),
        response(0, r#"{"models":[]}"#, ""),
    ]);
    let runner = ScriptedRunner::new(responses);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(result.desktoplab_started_runtime());
    assert!(result.evidence().contains("sleep 1"));
}

#[test]
fn persistent_macos_health_failure_stays_bounded_and_does_not_claim_ownership() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let mut responses = install_and_start_responses();
    for attempt in 1..=30 {
        responses.push(ScriptedResponse {
            exit_code: Some(7),
            stdout: "",
            stderr: "connection refused",
        });
        if attempt < 30 {
            responses.push(ScriptedResponse {
                exit_code: Some(0),
                stdout: "",
                stderr: "",
            });
        }
    }

    let result = RuntimeInstallExecutor::new(ScriptedRunner::new(responses))
        .execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "health_failed_retryable");
    assert!(!result.desktoplab_started_runtime());
}

#[test]
fn failed_macos_start_does_not_mark_runtime_verified() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let mut responses = verified_install_responses();
    responses.push(response(1, "", "application launch denied"));
    let runner = ScriptedRunner::new(responses);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "start_failed_retryable");
    assert!(result.remediation().contains("Ollama could not be started"));
}

fn install_and_start_responses() -> Vec<ScriptedResponse> {
    let mut responses = verified_install_responses();
    responses.push(response(0, "started", ""));
    responses
}

fn verified_install_responses() -> Vec<ScriptedResponse> {
    vec![
        response(1, "", "ollama not found"),
        response(0, "downloaded", ""),
        response(0, "extracted", ""),
        response(0, "valid source signature", ""),
        response(0, "accepted source", ""),
        response(0, "target absent", ""),
        response(0, "installed", ""),
        response(0, "source moved", ""),
        response(0, "valid installed signature", ""),
        response(0, "accepted installed app", ""),
    ]
}

fn response(exit_code: i32, stdout: &'static str, stderr: &'static str) -> ScriptedResponse {
    ScriptedResponse {
        exit_code: Some(exit_code),
        stdout,
        stderr,
    }
}
