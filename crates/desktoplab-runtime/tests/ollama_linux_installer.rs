use std::cell::RefCell;

use desktoplab_runtime::{
    InstallPlan, InstallerSource, OllamaRuntime, ProcessCommand, ProcessOutput, ProcessRunner,
    RuntimeExecutionState, RuntimeId, RuntimeInstallExecutionStrategy, RuntimeInstallExecutor,
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
fn linux_installer_blocks_automatic_download_without_trusted_digest() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("linux-x64")
        .expect("linux x64 should have an Ollama install plan");
    let runner = ScriptedRunner::new(vec![ScriptedResponse {
        exit_code: Some(1),
        stdout: "",
        stderr: "ollama not found",
    }]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Blocked);
    assert_eq!(
        result.verification_state(),
        "installer_integrity_metadata_missing"
    );
    assert!(result.evidence().contains("https://ollama.com/install.sh"));
    assert!(!result.evidence().contains("curl"));
}

#[test]
fn linux_installer_compares_trusted_digest_before_admin_action() {
    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let plan = trusted_linux_plan(DIGEST);
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
            stdout: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa  ollama-install-linux.sh",
            stderr: "",
        },
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Blocked);
    assert_eq!(result.verification_state(), "requires_admin_action");
    assert!(result.evidence().contains("curl"));
    assert!(result.evidence().contains("sha256sum"));
    assert!(!result.evidence().contains(" sh "));
    assert!(result.remediation().contains("administrator approval"));
}

#[test]
fn linux_installer_rejects_digest_mismatch() {
    const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let plan = trusted_linux_plan(DIGEST);
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
            stdout: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb  ollama-install-linux.sh",
            stderr: "",
        },
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "installer_verification_failed");
    assert!(result.evidence().contains("expected_sha256"));
    assert!(result.evidence().contains("actual_sha256"));
}

fn trusted_linux_plan(digest: &str) -> InstallPlan {
    InstallPlan::new(RuntimeId::new("runtime.ollama"), "Ollama")
        .with_target_platform("linux-x64")
        .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
        .with_disk_requirement_gb(4)
        .with_network_required(true)
        .with_installer_source(InstallerSource::signed_url(
            "https://ollama.com/install.sh",
            format!("sha256:{digest}"),
        ))
}
