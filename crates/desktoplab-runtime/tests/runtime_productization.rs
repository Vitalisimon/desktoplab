use desktoplab_runtime::{
    LmStudioProductionAdapter, OllamaInstallerAdapter, RuntimeRepairInventory, RuntimeRepairKind,
    RuntimeState,
};
use xtask::check_logical_line_limit;

#[test]
fn ollama_installer_adapter_downloads_on_demand_and_blocks_failed_verification() {
    let adapter = OllamaInstallerAdapter::for_target("macos-aarch64");
    let plan = adapter.install_plan();
    let failed = adapter.verify_installer("sha256:expected", "sha256:actual");

    assert!(!plan.is_bundled());
    assert!(
        plan.explanation()
            .contains("download Ollama installer on demand")
    );
    assert_eq!(failed.state(), RuntimeState::VerificationFailed);
}

#[test]
fn unsupported_ollama_platform_returns_guided_setup() {
    let adapter = OllamaInstallerAdapter::for_target("solaris-sparc");
    let plan = adapter.guided_setup_plan();

    assert!(!plan.can_install_automatically());
    assert!(plan.explanation().contains("install Ollama manually"));
}

#[test]
fn lm_studio_production_adapter_is_externally_managed() {
    let adapter = LmStudioProductionAdapter::new("http://127.0.0.1:1234");

    assert!(!adapter.can_be_stopped_by_desktoplab());
    assert!(adapter.endpoint_metadata().is_openai_compatible());
}

#[test]
fn runtime_repair_inventory_explains_next_action_from_state() {
    let inventory = RuntimeRepairInventory::new()
        .with_blocked_runtime("runtime.ollama", "Ollama", "checksum mismatch")
        .with_missing_runtime("runtime.lm-studio", "LM Studio");

    let ollama = inventory.repair_plan("runtime.ollama").unwrap();
    let lm_studio = inventory.repair_plan("runtime.lm-studio").unwrap();

    assert_eq!(ollama.kind(), RuntimeRepairKind::VerifyDownload);
    assert_eq!(lm_studio.kind(), RuntimeRepairKind::GuidedInstall);
    assert!(!ollama.log_excerpt().contains("sk-live-secret"));
}

#[test]
fn runtime_productization_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/productization.rs",
        include_str!("../src/productization.rs"),
        260,
    )
    .expect("runtime productization source should stay focused");
}
