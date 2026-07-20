use desktoplab_e2e_harness::{BackendProductizationGatePack, ProductizationGate};

#[test]
fn backend_productization_gate_pack_names_all_packaging_readiness_gates() {
    let pack = BackendProductizationGatePack::default();

    for gate in [
        ProductizationGate::LocalSetup,
        ProductizationGate::CloudProviderDryRun,
        ProductizationGate::LocalAgentEditTest,
        ProductizationGate::GitSavePointRollbackCommit,
        ProductizationGate::WorktreeParallelWrite,
        ProductizationGate::PluginTrust,
        ProductizationGate::DegradedOffline,
        ProductizationGate::SecurityDenial,
    ] {
        assert!(pack.contains(gate));
    }
}

#[test]
fn backend_productization_gate_pack_passes_only_when_every_gate_passed() {
    let mut pack = BackendProductizationGatePack::default();
    assert!(!pack.is_packaging_ready());

    pack.mark_all_passed();

    assert!(pack.is_packaging_ready());
    assert_eq!(pack.failed_gates(), Vec::<ProductizationGate>::new());
}
