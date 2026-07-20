use desktoplab_compatibility::{
    CommercialUseState, FrontierCatalogClaimState, FrontierModelClassCatalog,
    FrontierModelClassEntry, FrontierParameterClass, ModelArtifactProvenance,
    ProductModelSeedCatalog,
};
use xtask::check_logical_line_limit;

#[test]
fn planned_catalog_covers_70b_through_1t_without_fake_selectable_models() {
    let catalog = ProductModelSeedCatalog::frontier_class_catalog();
    let classes: Vec<_> = catalog
        .entries()
        .iter()
        .map(FrontierModelClassEntry::parameter_class)
        .collect();

    assert_eq!(catalog, FrontierModelClassCatalog::planned());
    assert_eq!(
        classes,
        vec![
            FrontierParameterClass::B70,
            FrontierParameterClass::B100,
            FrontierParameterClass::B200,
            FrontierParameterClass::B300,
            FrontierParameterClass::B400,
            FrontierParameterClass::B600,
            FrontierParameterClass::T1,
        ]
    );
    assert!(catalog.entries().iter().all(|entry| {
        !entry.is_selectable()
            && entry.claim_state() == FrontierCatalogClaimState::ResearchNeeded
            && entry
                .blocked_reasons()
                .contains(&"source_or_checksum_missing")
            && entry
                .blocked_reasons()
                .contains(&"runtime_adapter_evidence_missing")
    }));
}

#[test]
fn catalog_records_precision_quantization_runtime_license_and_storage_fields() {
    let catalog = FrontierModelClassCatalog::planned();
    let class_600b = catalog
        .entries()
        .iter()
        .find(|entry| entry.parameter_class() == FrontierParameterClass::B600)
        .unwrap();

    assert_eq!(class_600b.parameter_class().parameters_billion(), 600);
    assert!(
        class_600b
            .quantization_formats()
            .contains(&"int4".to_string())
    );
    assert!(class_600b.precision_formats().contains(&"bf16".to_string()));
    assert_eq!(class_600b.estimated_memory_gb(), 800);
    assert_eq!(class_600b.estimated_disk_gb(), 450);
    assert!(
        class_600b
            .runtime_ids()
            .contains(&"runtime.vllm".to_string())
    );
    assert_eq!(class_600b.license_id(), None);
    assert_eq!(class_600b.commercial_use(), CommercialUseState::Unknown);
}

#[test]
fn exact_distribution_and_runtime_evidence_is_required_before_selection() {
    let provenance = ModelArtifactProvenance::verified(
        "https://models.example.invalid/weights.bin",
        "a".repeat(64),
    )
    .unwrap();
    let entry = FrontierModelClassEntry::planned(FrontierParameterClass::B70, 96, 50)
        .with_context_window(131_072)
        .with_distribution_evidence("license.reviewed", CommercialUseState::Allowed, provenance)
        .with_runtime_adapter_evidence()
        .with_claim_state(FrontierCatalogClaimState::Available);

    assert!(entry.is_selectable());
    assert!(entry.blocked_reasons().is_empty());
    assert_eq!(
        entry.provenance().unwrap().checksum_sha256(),
        "a".repeat(64)
    );
}

#[test]
fn frontier_catalog_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-compatibility/src/frontier_catalog.rs",
        include_str!("../src/frontier_catalog.rs"),
        320,
    )
    .expect("frontier model catalog should stay focused");
}
