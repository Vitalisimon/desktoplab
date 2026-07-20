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
    let runner = ScriptedRunner::new(vec![
        ScriptedResponse {
            exit_code: Some(1),
            stdout: "",
            stderr: "ollama not found",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "downloaded",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "accepted",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "/dev/disk4s1 Apple_HFS /Volumes/Ollama",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "copied",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "detached",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "started",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: r#"{"models":[]}"#,
            stderr: "",
        },
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(result.evidence().contains("spctl --assess"));
    assert!(result.evidence().contains("hdiutil attach"));
    assert!(result.evidence().contains("ditto"));
    assert!(result.evidence().contains("hdiutil detach"));
    assert!(result.evidence().contains("open -a Ollama"));
    assert!(
        result
            .evidence()
            .contains("http://127.0.0.1:11434/api/tags")
    );
}

#[test]
fn failed_macos_start_does_not_mark_runtime_verified() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let runner = ScriptedRunner::new(vec![
        ScriptedResponse {
            exit_code: Some(1),
            stdout: "",
            stderr: "ollama not found",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "downloaded",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "accepted",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "/dev/disk4s1 Apple_HFS /Volumes/Ollama",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "copied",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(0),
            stdout: "detached",
            stderr: "",
        },
        ScriptedResponse {
            exit_code: Some(1),
            stdout: "",
            stderr: "application launch denied",
        },
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "start_failed_retryable");
    assert!(result.remediation().contains("Ollama could not be started"));
}
