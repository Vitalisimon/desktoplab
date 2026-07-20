use desktoplab_runtime::{
    DeterministicProcessRunner, LmStudioRuntime, MlxLmRuntime, OllamaRuntime, ProcessCommand,
    RuntimeExecutionState, RuntimeInstallExecutor,
};
use xtask::check_logical_line_limit;

#[test]
fn runtime_process_adapter_executes_command_and_records_redacted_evidence() {
    let runner = DeterministicProcessRunner::succeeds("ollama 0.5.0", "token=secret");
    let output = runner.run(ProcessCommand::new("ollama").arg("--version"));

    assert_eq!(output.exit_code(), Some(0));
    assert_eq!(output.stdout(), "ollama 0.5.0");
    assert_eq!(output.stderr(), "[REDACTED]");
    assert_eq!(output.evidence().program(), "ollama");
    assert_eq!(output.evidence().args(), &["--version"]);
}

#[test]
fn ollama_command_candidates_include_gui_safe_macos_locations() {
    let command = ProcessCommand::new("ollama").arg("--version");

    let candidates = command.program_candidates();

    assert_eq!(candidates[0], "ollama");
    assert!(candidates.contains(&"/usr/local/bin/ollama".to_string()));
    assert!(candidates.contains(&"/opt/homebrew/bin/ollama".to_string()));
    assert!(candidates.contains(&"/Applications/Ollama.app/Contents/Resources/ollama".to_string()));
}

#[test]
#[cfg(target_os = "windows")]
fn ollama_command_candidates_include_windows_per_user_install() {
    let candidates = ProcessCommand::new("ollama").program_candidates();

    assert!(candidates.iter().any(|candidate| {
        candidate
            .replace('\\', "/")
            .ends_with("/Programs/Ollama/ollama.exe")
    }));
}

#[test]
fn ordinary_command_candidates_do_not_add_runtime_paths() {
    let command = ProcessCommand::new("curl");

    assert_eq!(command.program_candidates(), vec!["curl".to_string()]);
}

#[test]
fn ollama_install_executor_completes_or_blocks_truthfully() {
    let runtime = OllamaRuntime::new();
    let plan = runtime
        .try_platform_install_plan("darwin-arm64")
        .expect("ollama supports mac arm64");
    let runner = DeterministicProcessRunner::succeeds("ollama 0.5.0", "");

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(result.evidence().contains("ollama --version"));
    assert!(result.evidence().contains("existing runtime detected"));
    assert!(!result.desktoplab_started_runtime());
}

#[test]
fn ollama_verify_existing_never_installs_from_missing_runtime() {
    let runner = DeterministicProcessRunner::missing();

    let result = RuntimeInstallExecutor::new(runner).verify_existing("runtime.ollama");

    assert_eq!(result.state(), RuntimeExecutionState::Blocked);
    assert_eq!(result.verification_state(), "blocked");
    assert!(result.evidence().contains("ollama --version"));
    assert!(!result.evidence().contains("curl"));
    assert!(!result.desktoplab_started_runtime());
}

#[test]
fn ollama_verify_existing_requires_health_endpoint() {
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(0), "ollama 0.5.0", ""),
        (Some(0), r#"{"models":[]}"#, ""),
    ]);

    let result = RuntimeInstallExecutor::new(runner).verify_existing("runtime.ollama");

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(
        result
            .evidence()
            .contains("curl --fail http://127.0.0.1:11434/api/tags")
    );
}

#[test]
fn ollama_install_executor_marks_runtime_started_by_desktoplab() {
    let runtime = OllamaRuntime::new();
    let plan = runtime
        .try_platform_install_plan("darwin-arm64")
        .expect("ollama supports mac arm64");
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(1), "", "command not found"),
        (Some(0), "downloaded", ""),
        (Some(0), "accepted", ""),
        (Some(0), "/Volumes/Ollama", ""),
        (Some(0), "copied", ""),
        (Some(0), "detached", ""),
        (Some(0), "started", ""),
        (Some(0), r#"{"models":[]}"#, ""),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(result.evidence().contains("open -a Ollama"));
    assert!(result.desktoplab_started_runtime());
}

#[test]
fn windows_ollama_install_verifies_publisher_and_reaches_health() {
    let runtime = OllamaRuntime::new();
    let plan = runtime
        .try_platform_install_plan("windows-x64")
        .expect("ollama supports Windows x64");
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(1), "", "command not found"),
        (Some(0), "downloaded", ""),
        (Some(0), "Status=Valid; Subject=Ollama, Inc.", ""),
        (Some(0), "installed", ""),
        (Some(0), "ollama version 0.32.0", ""),
        (Some(0), "started", ""),
        (Some(0), r#"{"models":[]}"#, ""),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(result.evidence().contains("OllamaSetup.exe"));
    assert!(result.evidence().contains("Get-AuthenticodeSignature"));
    assert!(!result.evidence().contains("$args[0]"));
    assert!(result.evidence().contains("/VERYSILENT"));
    assert!(result.evidence().contains("Start-Process"));
    assert!(result.desktoplab_started_runtime());
}

#[test]
fn windows_ollama_install_stops_when_authenticode_fails() {
    let runtime = OllamaRuntime::new();
    let plan = runtime
        .try_platform_install_plan("windows-x64")
        .expect("ollama supports Windows x64");
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(1), "", "command not found"),
        (Some(0), "downloaded", ""),
        (Some(1), "", "untrusted signer"),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "installer_verification_failed");
    assert!(!result.evidence().contains("/VERYSILENT"));
}

#[test]
fn existing_ollama_requires_local_api_health_before_runtime_is_verified() {
    let runtime = OllamaRuntime::new();
    let plan = runtime
        .try_platform_install_plan("darwin-arm64")
        .expect("ollama supports mac arm64");
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(0), "ollama 0.5.0", ""),
        (Some(1), "", "connection refused"),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Failed);
    assert_eq!(result.verification_state(), "health_failed_retryable");
    assert!(result.evidence().contains("ollama --version"));
    assert!(
        result
            .evidence()
            .contains("curl --fail http://127.0.0.1:11434/api/tags")
    );
}

#[test]
fn mlx_lm_install_executor_manages_python_environment_and_verifies_import() {
    let runtime = MlxLmRuntime::new();
    let plan = runtime
        .try_install_plan("darwin-arm64")
        .expect("Apple Silicon should have an MLX-LM plan");
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(1), "", "ModuleNotFoundError: mlx_lm"),
        (Some(0), "installed mlx-lm", ""),
        (Some(0), "mlx-lm import ok", ""),
    ]);

    let result = RuntimeInstallExecutor::new(runner).execute_existing_or_install(&plan);

    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert_eq!(result.verification_state(), "verified");
    assert!(result.evidence().contains("python3 -m pip install"));
    assert!(result.evidence().contains("import mlx_lm"));
}

#[test]
fn lm_studio_reports_external_guided_setup() {
    let runtime = LmStudioRuntime::new();
    let result = RuntimeInstallExecutor::external_guided(runtime.guided_setup_plan("darwin-arm64"));

    assert_eq!(result.state(), RuntimeExecutionState::ExternalGuided);
    assert_eq!(result.verification_state(), "requires_external_app");
    assert!(result.remediation().contains("start local server"));
}

#[test]
fn runtime_execution_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/process.rs",
        include_str!("../src/process.rs"),
        220,
    )
    .expect("runtime process adapter should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/execution.rs",
        include_str!("../src/execution.rs"),
        220,
    )
    .expect("runtime execution adapter should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/execution_result.rs",
        include_str!("../src/execution_result.rs"),
        120,
    )
    .expect("runtime execution result should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/mlx_lm_execution.rs",
        include_str!("../src/mlx_lm_execution.rs"),
        120,
    )
    .expect("MLX-LM runtime execution adapter should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/windows_ollama_install.rs",
        include_str!("../src/windows_ollama_install.rs"),
        220,
    )
    .expect("Windows Ollama installer should stay focused");
}
