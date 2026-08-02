use std::{
    cell::RefCell,
    path::Path,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

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

static ENV_LOCK: Mutex<()> = Mutex::new(());

impl ScriptedRunner {
    fn new(responses: Vec<ScriptedResponse>) -> Self {
        Self {
            responses: RefCell::new(responses),
        }
    }
}

impl ProcessRunner for ScriptedRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        let mut responses = self.responses.borrow_mut();
        let response = responses.remove(0);
        ProcessOutput::new(
            response.exit_code,
            response.stdout,
            response.stderr,
            command,
        )
    }
}

#[derive(Debug)]
struct CachePreparingRunner {
    cache_root: String,
    calls: RefCell<usize>,
}

impl CachePreparingRunner {
    fn new(cache_root: String) -> Self {
        Self {
            cache_root,
            calls: RefCell::new(0),
        }
    }
}

impl ProcessRunner for CachePreparingRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        let mut calls = self.calls.borrow_mut();
        *calls += 1;
        if *calls == 1 {
            return ProcessOutput::new(Some(1), "", "ollama not found", command);
        }
        if command.program() == "curl" && command.args().iter().any(|arg| arg == "--output") {
            let target = command
                .args()
                .windows(2)
                .find(|window| window[0] == "--output")
                .map(|window| window[1].clone())
                .expect("curl output target should be present");
            assert!(
                target.starts_with(&self.cache_root),
                "installer cache should use the isolated cache root: {target}"
            );
            assert!(
                Path::new(&target).parent().is_some_and(Path::exists),
                "installer cache parent should exist before curl runs: {target}"
            );
        }
        ProcessOutput::new(Some(0), "ok", "", command)
    }
}

#[test]
fn macos_installer_prepares_cache_directory_before_download() {
    let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
    let cache_root = std::env::temp_dir().join(format!(
        "desktoplab-runtime-cache-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after epoch")
            .as_nanos()
    ));
    unsafe {
        std::env::set_var("DESKTOPLAB_RUNTIME_INSTALLER_CACHE_DIR", &cache_root);
    }
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let runner = CachePreparingRunner::new(cache_root.to_string_lossy().to_string());

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    unsafe {
        std::env::remove_var("DESKTOPLAB_RUNTIME_INSTALLER_CACHE_DIR");
    }
    let _ = std::fs::remove_dir_all(&cache_root);
    assert_eq!(result.state(), RuntimeExecutionState::Completed);
}

#[test]
fn macos_installer_download_uses_plan_source_cache_and_retryable_network_failure() {
    let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
    unsafe {
        std::env::remove_var("DESKTOPLAB_RUNTIME_INSTALLER_CACHE_DIR");
    }
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
            exit_code: Some(6),
            stdout: "",
            stderr: "Could not resolve host",
        },
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "download_failed_retryable");
    assert!(result.evidence().contains("curl"));
    assert!(
        result
            .evidence()
            .contains("https://ollama.com/download/Ollama-darwin.zip")
    );
    assert!(
        result
            .evidence()
            .contains("runtime-installers/ollama/Ollama-darwin-arm64.zip")
    );
    assert!(result.evidence().contains("checksum=vendor-signed"));
    assert!(
        result
            .evidence()
            .contains("signature=ollama-vendor-signature")
    );
    assert!(result.remediation().contains("Network connection"));
}

#[test]
fn macos_installer_extracts_and_verifies_the_vendor_app_before_and_after_install() {
    let _guard = ENV_LOCK.lock().expect("env lock should not be poisoned");
    unsafe {
        std::env::remove_var("DESKTOPLAB_RUNTIME_INSTALLER_CACHE_DIR");
    }
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let runner = ScriptedRunner::new(vec![
        response(1, "", "ollama not found"),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "{}", ""),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert!(result.evidence().contains("unzip"));
    assert!(
        result
            .evidence()
            .contains("codesign --verify --deep --strict")
    );
    assert!(result.evidence().contains("spctl --assess --type execute"));
    assert!(
        result
            .evidence()
            .contains("test ! -e /Applications/Ollama.app")
    );
    assert!(result.evidence().contains("mv"));
    assert!(!result.evidence().contains("hdiutil"));
    assert!(!result.evidence().contains("ditto"));
}

#[test]
fn macos_installer_preserves_an_existing_app_instead_of_overwriting_it() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let runner = ScriptedRunner::new(vec![
        response(1, "", "ollama not found"),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(1, "", "existing app"),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Blocked);
    assert_eq!(result.verification_state(), "existing_runtime_preserved");
    assert!(result.remediation().contains("already exists"));
}

#[test]
fn macos_installer_quarantines_a_new_copy_that_fails_post_install_verification() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let runner = ScriptedRunner::new(vec![
        response(1, "", "ollama not found"),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(1, "", "invalid installed signature"),
        response(0, "quarantined", ""),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(
        result.verification_state(),
        "installed_runtime_verification_failed"
    );
    assert!(result.evidence().contains("rejected-Ollama.app"));
}

#[test]
fn macos_installer_does_not_claim_a_racing_existing_app_when_no_clobber_move_skips() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("macOS arm64 should have an Ollama install plan");
    let runner = ScriptedRunner::new(vec![
        response(1, "", "ollama not found"),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "", ""),
        response(0, "move skipped without error", ""),
        response(1, "", "source still exists"),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Blocked);
    assert_eq!(result.verification_state(), "existing_runtime_preserved");
    assert!(result.remediation().contains("appeared in Applications"));
}

fn response(exit_code: i32, stdout: &'static str, stderr: &'static str) -> ScriptedResponse {
    ScriptedResponse {
        exit_code: Some(exit_code),
        stdout,
        stderr,
    }
}
