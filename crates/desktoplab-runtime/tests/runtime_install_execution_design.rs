use desktoplab_runtime::{OllamaRuntime, RuntimeInstallExecutionDesign, RuntimeInstallPhase};

#[test]
fn runtime_install_execution_design_names_all_required_phases() {
    let plan = OllamaRuntime::new()
        .try_platform_install_plan("darwin-arm64")
        .expect("ollama macOS install plan should exist");
    let design = RuntimeInstallExecutionDesign::from_install_plan(&plan);

    assert_eq!(
        design.phases(),
        &[
            RuntimeInstallPhase::Detect,
            RuntimeInstallPhase::Download,
            RuntimeInstallPhase::VerifyInstaller,
            RuntimeInstallPhase::Install,
            RuntimeInstallPhase::Start,
            RuntimeInstallPhase::Health,
        ]
    );
    assert_eq!(design.runtime_id(), "runtime.ollama");
}
