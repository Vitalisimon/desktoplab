use desktoplab_runtime::{
    HighEndRuntimeContract, HighEndRuntimeFamily, RuntimeCapabilityState,
    RuntimeInferenceCapabilities, RuntimeLaunchSupport, RuntimeSessionOwnership,
    high_end_runtime_contracts,
};
use xtask::check_logical_line_limit;

#[test]
fn high_end_runtime_families_exist_without_assuming_capabilities() {
    let contracts = high_end_runtime_contracts();
    let families: Vec<_> = contracts
        .iter()
        .map(HighEndRuntimeContract::family)
        .collect();

    assert_eq!(contracts.len(), 6);
    for family in [
        HighEndRuntimeFamily::Nim,
        HighEndRuntimeFamily::TensorRtLlm,
        HighEndRuntimeFamily::Vllm,
        HighEndRuntimeFamily::LlamaCppServer,
        HighEndRuntimeFamily::OpenAiCompatibleLocal,
        HighEndRuntimeFamily::CustomLan,
    ] {
        assert!(families.contains(&family), "missing {}", family.as_str());
    }
    assert!(contracts.iter().all(|contract| {
        contract.capabilities().tool_calling() == RuntimeCapabilityState::ProbeRequired
            && contract.capabilities().has_unverified_fields()
            && contract.session_ownership() == RuntimeSessionOwnership::DesktopLab
    }));
}

#[test]
fn endpoint_evidence_can_fill_the_full_inference_contract() {
    let capabilities = RuntimeInferenceCapabilities::probe_required()
        .with_protocol_support(
            RuntimeCapabilityState::Confirmed,
            RuntimeCapabilityState::Confirmed,
            RuntimeCapabilityState::Confirmed,
        )
        .with_context_limit(262_144)
        .with_batching(RuntimeCapabilityState::Confirmed, 32)
        .with_tensor_parallelism(RuntimeCapabilityState::Confirmed, 8)
        .with_quantization_formats(&["fp8", "bf16", "int4"]);

    assert!(!capabilities.has_unverified_fields());
    assert_eq!(capabilities.max_context_tokens(), Some(262_144));
    assert_eq!(capabilities.max_batch_size(), Some(32));
    assert_eq!(capabilities.max_tensor_parallel_size(), Some(8));
    assert_eq!(
        capabilities.quantization_formats(),
        &["fp8", "bf16", "int4"]
    );
}

#[test]
fn custom_endpoints_are_attach_only_while_known_runtimes_can_be_launchable() {
    let contracts = high_end_runtime_contracts();
    let custom = contracts
        .iter()
        .find(|contract| contract.family() == HighEndRuntimeFamily::CustomLan)
        .unwrap();
    let vllm = contracts
        .iter()
        .find(|contract| contract.family() == HighEndRuntimeFamily::Vllm)
        .unwrap();

    assert_eq!(custom.launch_support(), RuntimeLaunchSupport::AttachOnly);
    assert_eq!(vllm.launch_support(), RuntimeLaunchSupport::LaunchOrAttach);
}

#[test]
fn high_end_runtime_contract_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/high_end.rs",
        include_str!("../src/high_end.rs"),
        320,
    )
    .expect("high-end runtime contract should stay focused");
}
