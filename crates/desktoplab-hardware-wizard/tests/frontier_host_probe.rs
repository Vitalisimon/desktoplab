use desktoplab_hardware_wizard::{
    FrontierFeatureState, FrontierHardwareClass, FrontierHardwareClassifier, FrontierHostFacts,
    FrontierHostProbeAdapter, FrontierProbeSource, HardwareFactSource,
};
use xtask::check_logical_line_limit;

const NOW: u64 = 10_000;

#[test]
fn detected_capability_envelopes_classify_station_spark_and_custom_rigs() {
    let classifier = FrontierHardwareClassifier::new(300);

    let station = complete_facts(&["accelerator-0"], 288)
        .with_memory(748, Some(748))
        .with_storage_available_gb(4_000);
    assert_eq!(
        classifier.classify(&station, NOW).class(),
        FrontierHardwareClass::DgxStationClass
    );

    let spark = complete_facts(&["accelerator-0"], 128)
        .with_memory(128, Some(128))
        .with_storage_available_gb(1_000);
    assert_eq!(
        classifier.classify(&spark, NOW).class(),
        FrontierHardwareClass::DgxSparkClass
    );

    let custom = complete_facts(&["accelerator-0", "accelerator-1"], 384)
        .with_memory(512, None)
        .with_storage_available_gb(2_000);
    let assessment = classifier.classify(&custom, NOW);
    assert_eq!(assessment.class(), FrontierHardwareClass::CustomFrontierRig);
    assert!(assessment.is_high_end_candidate());
}

#[test]
fn declared_stale_or_incomplete_facts_cannot_promote_high_end_routes() {
    let classifier = FrontierHardwareClassifier::new(300);
    let declared = FrontierHostFacts::declared(NOW)
        .with_accelerators(&["declared-gpu"], 800)
        .with_memory(800, Some(800))
        .with_storage_available_gb(8_000)
        .with_cuda("600", "13");
    let declared_assessment = classifier.classify(&declared, NOW);
    assert_eq!(declared.source(), HardwareFactSource::Declared);
    assert_eq!(
        declared_assessment.class(),
        FrontierHardwareClass::Unclassified
    );
    assert!(
        declared_assessment
            .blockers()
            .iter()
            .any(|reason| reason.contains("detected"))
    );

    let stale = complete_facts(&["accelerator-0"], 800)
        .with_memory(800, Some(800))
        .with_storage_available_gb(8_000);
    assert!(
        classifier
            .classify(&stale, NOW + 301)
            .blockers()
            .iter()
            .any(|reason| reason.contains("stale"))
    );

    let incomplete = FrontierHostFacts::detected(NOW)
        .with_memory(800, Some(800))
        .with_storage_available_gb(8_000);
    assert!(
        !classifier
            .classify(&incomplete, NOW)
            .is_high_end_candidate()
    );
}

#[test]
fn probe_adapter_preserves_measured_topology_and_versions() {
    let facts = FrontierHostProbeAdapter::new(FixtureSource).probe();

    assert_eq!(facts.gpu_models(), &["GPU 0", "GPU 1"]);
    assert_eq!(facts.accelerator_count(), 2);
    assert_eq!(facts.accelerator_memory_gb(), Some(384));
    assert_eq!(facts.coherent_memory_gb(), None);
    assert_eq!(facts.cpu_ram_gb(), Some(512));
    assert_eq!(facts.storage_available_gb(), Some(2_000));
    assert_eq!(facts.cuda_driver_version(), Some("600.1"));
    assert_eq!(facts.cuda_runtime_version(), Some("13.0"));
    assert_eq!(facts.nvlink(), FrontierFeatureState::Detected);
    assert_eq!(facts.nvswitch(), FrontierFeatureState::Detected);
    assert_eq!(facts.mig(), FrontierFeatureState::NotDetected);
}

#[test]
fn frontier_probe_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-hardware-wizard/src/frontier.rs",
        include_str!("../src/frontier.rs"),
        380,
    )
    .expect("frontier hardware probe source should stay focused");
}

fn complete_facts(gpus: &[&str], accelerator_memory_gb: u32) -> FrontierHostFacts {
    FrontierHostFacts::detected(NOW)
        .with_accelerators(gpus, accelerator_memory_gb)
        .with_memory(512, None)
        .with_storage_available_gb(2_000)
        .with_cuda("600.1", "13.0")
        .with_topology(
            FrontierFeatureState::Detected,
            FrontierFeatureState::Detected,
            FrontierFeatureState::NotDetected,
        )
}

struct FixtureSource;

impl FrontierProbeSource for FixtureSource {
    fn observe(&self) -> FrontierHostFacts {
        complete_facts(&["GPU 0", "GPU 1"], 384)
    }
}
