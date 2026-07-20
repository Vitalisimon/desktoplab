use desktoplab_compatibility::{
    CompatibilityCatalog, CompatibilityEngine, HardwareProfile, MatchRequest, ModelManifest,
    ProductModelSeedCatalog, RuntimeManifest,
};
use desktoplab_model_manager::{
    DownloadPolicy, ModelDownloadPlan, ModelFamilyCatalog, ModelManager, ModelParameterClass,
    ModelReadiness, ModelVariant, ModelVerification, SetupSelection,
};
use desktoplab_runtime::RuntimeId;
use xtask::check_logical_line_limit;

#[test]
fn model_recommendations_come_from_registry_and_compatibility_engine() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(RuntimeManifest::new("runtime.ollama", &["gguf"]))
        .with_model(ModelManifest::new("model.qwen3-8b", "gguf", 8));
    let engine = CompatibilityEngine::new(catalog)
        .with_hardware(HardwareProfile::new("linux", "x86_64", 32));
    let manager = ModelManager::new();

    let recommendation = manager.recommend(
        &engine,
        MatchRequest::new("runtime.ollama", "model.qwen3-8b"),
    );

    assert!(recommendation.is_recommended());
    assert_eq!(recommendation.model_id(), "model.qwen3-8b");
}

#[test]
fn model_downloads_are_automatic_after_accepted_setup_plan() {
    let selection =
        SetupSelection::accepted(RuntimeId::new("runtime.ollama"), "model.qwen3-8b", 5_000);
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);

    assert!(plan.starts_automatically());
    assert_eq!(plan.runtime_id(), &RuntimeId::new("runtime.ollama"));
    assert_eq!(plan.model_id(), "model.qwen3-8b");
    assert_eq!(plan.expected_disk_mb(), 5_000);
}

#[test]
fn unaccepted_setup_selection_does_not_start_download() {
    let selection = SetupSelection::preview(RuntimeId::new("runtime.lm-studio"), "model.devstral");
    let plan = ModelDownloadPlan::from_selection(&selection, DownloadPolicy::AutomaticAfterAccept);

    assert!(!plan.starts_automatically());
}

#[test]
fn model_variant_ranking_follows_hardware_fit_and_storage() {
    let manager = ModelManager::new();
    let catalog = ModelFamilyCatalog::new()
        .with_variant(
            ModelVariant::new(
                "family.qwen",
                "Qwen",
                "model.qwen-coder-7b",
                ModelParameterClass::Small,
                5_500,
                "runtime.ollama",
                "qwen2.5-coder:7b",
                "stable",
            )
            .with_required_memory_gb(12),
        )
        .with_variant(
            ModelVariant::new(
                "family.deepseek",
                "DeepSeek",
                "model.deepseek-coder-7b",
                ModelParameterClass::Medium,
                12_000,
                "runtime.ollama",
                "deepseek-coder:6.7b",
                "stable",
            )
            .with_required_memory_gb(24),
        )
        .with_variant(
            ModelVariant::new(
                "family.synthetic-workstation",
                "Synthetic Workstation",
                "model.synthetic-workstation-70b",
                ModelParameterClass::Workstation,
                42_000,
                "runtime.ollama",
                "synthetic-workstation:70b",
                "experimental",
            )
            .with_required_memory_gb(96),
        );

    let laptop = manager.rank_variants(&catalog, 16, 100_000);
    assert_eq!(laptop[0].model_id(), "model.qwen-coder-7b");
    assert!(laptop[0].is_recommended());
    assert!(!laptop[2].is_recommended());
    assert_eq!(laptop[2].reason(), "not recommended on this computer");

    let workstation = manager.rank_variants(&catalog, 128, 100_000);
    assert!(workstation[2].is_recommended());
    assert_eq!(workstation[2].reason(), "fits this machine");

    let storage_limited = manager.rank_variants(&catalog, 128, 8_000);
    assert_eq!(storage_limited[1].reason(), "not enough free storage");
    assert!(!storage_limited[1].is_recommended());
}

#[test]
fn default_catalog_seeds_multiple_model_families_without_ui_branching() {
    let catalog = ModelManager::new().default_family_catalog();

    assert!(catalog.variants_for_family("family.qwen").is_empty());
    assert_eq!(catalog.variants_for_family("family.qwen3.5").len(), 2);
    assert_eq!(catalog.variants_for_family("family.qwen3-coder").len(), 2);
    assert_eq!(catalog.variants_for_family("family.gemma4").len(), 2);
    assert_eq!(catalog.variants_for_family("family.gpt-oss").len(), 2);
    assert_eq!(
        catalog.variants_for_family("family.nemotron-3-nano").len(),
        2
    );
    assert!(catalog.variants_for_family("family.deepseek").is_empty());
    assert!(catalog.variants_for_family("family.glm").is_empty());

    let gemma_tiers: Vec<ModelParameterClass> = catalog
        .variants_for_family("family.gemma4")
        .iter()
        .map(|variant| variant.parameter_class())
        .collect();
    assert!(gemma_tiers.contains(&ModelParameterClass::Small));
    assert!(gemma_tiers.contains(&ModelParameterClass::Medium));

    let workstation_runtime = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.qwen3-coder-480b-q4")
        .expect("frontier coding model should be present as downloadable catalog data");
    assert_eq!(
        workstation_runtime.runtime_compatibility().runtime_id(),
        "runtime.ollama"
    );
    assert_eq!(
        workstation_runtime.runtime_compatibility().pull_ref(),
        "qwen3-coder:480b"
    );
}

#[test]
fn default_catalog_excludes_uncertified_community_conversions() {
    let catalog = ModelManager::new().default_family_catalog();
    let mlx_variants: Vec<_> = catalog
        .variants()
        .iter()
        .filter(|variant| variant.runtime_compatibility().runtime_id() == "runtime.mlx-lm")
        .collect();

    assert!(mlx_variants.is_empty());
}

#[test]
fn default_catalog_is_derived_from_the_authoritative_seed_catalog() {
    let seed_ids = ProductModelSeedCatalog::initial_coding().model_ids();
    let manager_ids: Vec<String> = ModelManager::new()
        .default_family_catalog()
        .variants()
        .iter()
        .map(|variant| variant.model_id().to_string())
        .collect();

    assert_eq!(manager_ids, seed_ids);
}

#[test]
fn verification_failure_blocks_model_readiness() {
    let readiness =
        ModelReadiness::from_verification(ModelVerification::failed("checksum mismatch"));

    assert!(!readiness.is_ready());
    assert_eq!(readiness.reason(), Some("checksum mismatch"));
}

#[test]
fn model_manager_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-model-manager/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-model-manager/src/download.rs",
            include_str!("../src/download.rs"),
            250,
        ),
        (
            "crates/desktoplab-model-manager/src/context_window.rs",
            include_str!("../src/context_window.rs"),
            120,
        ),
        (
            "crates/desktoplab-model-manager/src/request_timeout.rs",
            include_str!("../src/request_timeout.rs"),
            120,
        ),
        (
            "crates/desktoplab-model-manager/src/manager.rs",
            include_str!("../src/manager.rs"),
            250,
        ),
        (
            "crates/desktoplab-model-manager/src/readiness.rs",
            include_str!("../src/readiness.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("model manager source should stay below the initial line-count guard");
    }
}
