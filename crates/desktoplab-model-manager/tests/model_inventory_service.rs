use desktoplab_compatibility::{
    CompatibilityCatalog, CompatibilityEngine, HardwareProfile, ModelManifest, RuntimeManifest,
};
use desktoplab_model_manager::{
    InMemoryModelInventoryStore, ModelFamilyCatalog, ModelInstallState, ModelInventoryService,
    ModelInventorySource, ModelManager, ModelParameterClass, ModelRuntimeCompatibility,
    ModelVariant,
};
use desktoplab_runtime::RuntimeId;
use xtask::check_logical_line_limit;

#[test]
fn registry_and_local_inventory_stay_separate() {
    let catalog = catalog_with_model("model.qwen3-8b", 8);
    let store = InMemoryModelInventoryStore::default();
    let mut service = ModelInventoryService::new(store);
    service.record_local_model(RuntimeId::new("runtime.ollama"), "qwen3:8b");

    let inventory = service.inventory_for_runtime(
        &catalog,
        &CompatibilityEngine::new(catalog.clone()),
        "runtime.ollama",
    );

    assert_eq!(inventory.registry_models().len(), 1);
    assert_eq!(
        inventory.registry_models()[0].source(),
        ModelInventorySource::Registry
    );
    assert_eq!(inventory.local_models().len(), 1);
    assert_eq!(
        inventory.local_models()[0].source(),
        ModelInventorySource::LocalRuntime
    );
}

#[test]
fn installed_model_is_associated_with_runtime() {
    let catalog = catalog_with_model("model.qwen3-8b", 8);
    let store = InMemoryModelInventoryStore::default();
    let mut service = ModelInventoryService::new(store);

    service.record_local_model(RuntimeId::new("runtime.ollama"), "qwen3:8b");

    let inventory = service.inventory_for_runtime(
        &catalog,
        &CompatibilityEngine::new(catalog.clone()),
        "runtime.ollama",
    );
    let local = &inventory.local_models()[0];

    assert_eq!(local.install_state(), ModelInstallState::Installed);
    assert_eq!(local.runtime_id(), Some("runtime.ollama"));
    assert_eq!(
        local.provenance().catalog_source(),
        "local_runtime_inventory"
    );
    assert_eq!(
        local.provenance().verification_state(),
        "local_runtime_inventory"
    );
}

#[test]
fn compatibility_engine_controls_recommendation_status() {
    let catalog = catalog_with_model("model.large", 64);
    let engine = CompatibilityEngine::new(catalog.clone())
        .with_hardware(HardwareProfile::new("macos", "aarch64", 16));
    let service = ModelInventoryService::new(InMemoryModelInventoryStore::default());

    let inventory = service.inventory_for_runtime(&catalog, &engine, "runtime.ollama");

    assert!(!inventory.registry_models()[0].is_recommended());
    assert_eq!(
        inventory.registry_models()[0].compatibility_reason(),
        "model requires more memory than current hardware"
    );
    assert_eq!(
        inventory.registry_models()[0].provenance().catalog_source(),
        "compatibility_catalog"
    );
    assert_eq!(
        inventory.registry_models()[0]
            .provenance()
            .verification_state(),
        "not_verified_locally"
    );
}

#[test]
fn model_families_and_variants_are_catalog_data() {
    let catalog = ModelFamilyCatalog::new()
        .with_variant(ModelVariant::new(
            "family.qwen",
            "Qwen",
            "model.qwen-coder-7b",
            ModelParameterClass::Small,
            4_700,
            "runtime.ollama",
            "qwen2.5-coder:7b",
            "stable",
        ))
        .with_variant(ModelVariant::new(
            "family.deepseek",
            "DeepSeek",
            "model.deepseek-coder-7b",
            ModelParameterClass::Small,
            3_800,
            "runtime.ollama",
            "deepseek-coder:6.7b",
            "stable",
        ))
        .with_variant(ModelVariant::new(
            "family.glm",
            "GLM",
            "model.glm4-9b",
            ModelParameterClass::Small,
            5_500,
            "runtime.ollama",
            "glm4:9b",
            "beta",
        ))
        .with_variant(ModelVariant::new(
            "family.nvidia-nemotron",
            "NVIDIA Nemotron",
            "model.nemotron-70b-q4",
            ModelParameterClass::Workstation,
            43_000,
            "runtime.ollama",
            "nemotron:70b",
            "experimental",
        ));

    assert_eq!(catalog.families().len(), 4);
    assert_eq!(catalog.variants_for_family("family.qwen").len(), 1);
    assert_eq!(catalog.variants()[0].family_name(), "Qwen");
    assert_eq!(
        catalog.variants()[0].parameter_class(),
        ModelParameterClass::Small
    );
    assert_eq!(catalog.variants()[0].expected_disk_mb(), 4_700);
    assert_eq!(
        catalog.variants()[0].runtime_compatibility(),
        ModelRuntimeCompatibility::new("runtime.ollama", "qwen2.5-coder:7b")
    );
    assert_eq!(catalog.variants()[3].channel(), "experimental");
}

#[test]
fn default_catalog_preserves_model_metadata_from_seed_catalog() {
    let catalog = ModelManager::new().default_family_catalog();
    let gemma = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.gemma4-12b-q4")
        .expect("Gemma seed variant should exist");

    assert_eq!(gemma.parameters_billion(), 12);
    assert_eq!(gemma.quantization(), "Q4");
    assert_eq!(gemma.context_window_tokens(), 256_000);
    assert_eq!(gemma.license_state().as_str(), "known");
    assert_eq!(gemma.license_state().trust_label(), "License verified");

    let qwen3_coder = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.qwen3-coder-30b-q4")
        .expect("Qwen 3 Coder seed variant should exist");
    assert_eq!(qwen3_coder.license_state().as_str(), "known");
    assert_eq!(
        qwen3_coder.runtime_compatibility().pull_ref(),
        "qwen3-coder:30b"
    );
}

#[test]
fn inventory_persists_across_service_restart() {
    let store = InMemoryModelInventoryStore::default();
    let mut first_service = ModelInventoryService::new(store.clone());
    first_service.record_local_model(RuntimeId::new("runtime.ollama"), "qwen3:8b");

    let restarted_service = ModelInventoryService::new(store);
    let inventory = restarted_service.local_inventory();

    assert_eq!(inventory.len(), 1);
    assert_eq!(inventory[0].model_id(), "qwen3:8b");
    assert_eq!(inventory[0].runtime_id(), Some("runtime.ollama"));
}

#[test]
fn model_inventory_service_source_stays_below_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-model-manager/src/catalog.rs",
            include_str!("../src/catalog.rs"),
            240,
        ),
        (
            "crates/desktoplab-model-manager/src/inventory.rs",
            include_str!("../src/inventory.rs"),
            280,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("model manager source files should stay below the line-count guard");
    }
}

fn catalog_with_model(model_id: &str, memory_gb: u32) -> CompatibilityCatalog {
    CompatibilityCatalog::new()
        .with_runtime(RuntimeManifest::new("runtime.ollama", &["gguf"]))
        .with_model(ModelManifest::new(model_id, "gguf", memory_gb))
}
