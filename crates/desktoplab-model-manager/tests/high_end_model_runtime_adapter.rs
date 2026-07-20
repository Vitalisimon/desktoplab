use desktoplab_model_manager::HighEndModelRuntimeAdapter;
use desktoplab_runtime::{HighEndRuntimeFamily, high_end_runtime_contracts};

#[test]
fn high_end_model_adapter_exposes_contract_without_faking_weight_management() {
    let contract = high_end_runtime_contracts()
        .into_iter()
        .find(|contract| contract.family() == HighEndRuntimeFamily::Vllm)
        .unwrap();
    let adapter = HighEndModelRuntimeAdapter::new(contract);

    assert_eq!(adapter.contract().family(), HighEndRuntimeFamily::Vllm);
    assert!(adapter.contract().capabilities().has_unverified_fields());
    assert!(!adapter.supports_direct_pull());
}
