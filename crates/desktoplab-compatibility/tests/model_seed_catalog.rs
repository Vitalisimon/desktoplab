use desktoplab_compatibility::{Channel, ProductModelSeedCatalog};
use xtask::check_logical_line_limit;

#[test]
fn seed_catalog_contains_curated_downloadable_agent_models() {
    let catalog = ProductModelSeedCatalog::initial_coding();
    let ids = catalog.model_ids();

    assert_eq!(
        ids,
        vec![
            "model.nemotron-3-nano-4b-q4",
            "model.qwen3.5-9b-q4",
            "model.gemma4-12b-q4",
            "model.gpt-oss-20b-mxfp4",
            "model.qwen3-coder-30b-q4",
            "model.qwen3.6-27b-q4",
            "model.devstral-small-2-24b-q4",
            "model.north-mini-code-30b-q4",
            "model.gemma4-26b-q4",
            "model.qwen3.6-35b-q4",
            "model.nemotron-3-nano-30b-q4",
            "model.nemotron-cascade-2-30b-q4",
            "model.gpt-oss-120b-mxfp4",
            "model.qwen3-coder-next-80b-q4",
            "model.qwen3.5-122b-q4",
            "model.mistral-medium-3.5-128b-q4",
            "model.devstral-2-123b-q4",
            "model.nemotron-3-super-120b-q4",
            "model.qwen3-coder-480b-q4",
        ]
    );
    assert!(catalog.entries().iter().all(|entry| {
        entry.capabilities().contains(&"tool_use".to_string())
            && entry
                .capabilities()
                .contains(&"agent_candidate".to_string())
            && entry.is_downloadable_now()
    }));
    let validated = catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry
                .capabilities()
                .contains(&"desktoplab_live_validated".to_string())
        })
        .map(|entry| entry.model_id())
        .collect::<Vec<_>>();
    assert!(validated.is_empty());
}

#[test]
fn unsuitable_and_uncertified_models_are_absent_from_the_app_catalog() {
    let catalog = ProductModelSeedCatalog::initial_coding();
    let excluded = [
        "model.qwen-coder-7b-q4",
        "model.qwen-coder-14b-q4",
        "model.qwen-coder-32b-q4",
        "model.mlx-qwen-3.5-4b-8bit",
        "model.llama-3.1-8b-q4",
        "model.deepseek-coder-7b-q4",
        "model.deepseek-r1-8b-q4",
        "model.glm4-9b",
        "model.glm-5.2-cloud",
        "model.nemotron-70b-q4",
        "model.devstral-24b-q4",
        "model.codestral-22b-q4",
        "model.gemma3-12b-q4",
        "model.glm-4.7-flash-30b-q4",
    ];

    for model_id in excluded {
        assert!(
            catalog.entry(model_id).is_none(),
            "{model_id} leaked into app catalog"
        );
    }
}

#[test]
fn seed_catalog_never_recommends_unknown_license_entries() {
    let catalog = ProductModelSeedCatalog::initial_coding();

    assert!(
        catalog
            .entries()
            .iter()
            .filter(|entry| entry.license().is_unknown())
            .all(|entry| !entry.is_recommendable())
    );
}

#[test]
fn seed_catalog_marks_displayed_entries_as_known_license_downloadable_models() {
    let catalog = ProductModelSeedCatalog::initial_coding();

    assert!(
        catalog
            .entries()
            .iter()
            .all(|entry| entry.license().is_recommendable()
                && (entry.is_downloadable_now()
                    || entry.runtime_id() == "runtime.ollama-cloud"
                    || entry.runtime_id() == "runtime.mlx-lm"))
    );
}

#[test]
fn seed_catalog_uses_hardware_requirements_and_channels() {
    let catalog = ProductModelSeedCatalog::initial_coding();
    let candidate = catalog.entry("model.gemma4-12b-q4").unwrap();
    assert_eq!(candidate.required_memory_gb(), 16);
    assert_eq!(candidate.pull_ref(), "gemma4:12b");
    assert_eq!(candidate.channel(), Channel::Beta);
    assert_eq!(candidate.family_id(), "family.gemma4");
    assert_eq!(candidate.runtime_id(), "runtime.ollama");
    assert_eq!(candidate.expected_disk_mb(), 7_600);
    assert_eq!(candidate.parameters_billion(), 12);
    assert_eq!(candidate.quantization(), "Q4");
    assert_eq!(candidate.context_window_tokens(), 256_000);
    assert!(
        candidate
            .capabilities()
            .contains(&"agent_candidate".to_string())
    );
    assert!(
        !candidate
            .capabilities()
            .contains(&"desktoplab_live_validated".to_string())
    );
}

#[test]
fn downloadable_catalog_entries_all_use_implemented_runtime_pull_refs() {
    let catalog = ProductModelSeedCatalog::initial_coding();

    for entry in catalog
        .entries()
        .iter()
        .filter(|entry| entry.is_downloadable_now())
    {
        assert!(
            matches!(entry.runtime_id(), "runtime.ollama" | "runtime.mlx-lm"),
            "{}",
            entry.model_id()
        );
        assert!(!entry.pull_ref().trim().is_empty(), "{}", entry.model_id());
        assert!(
            entry
                .pull_ref()
                .chars()
                .all(|character| character.is_ascii_alphanumeric()
                    || matches!(character, '.' | '_' | '-' | ':' | '/'))
                && !entry.pull_ref().contains("..")
                && !entry.pull_ref().starts_with('/')
                && !entry.pull_ref().ends_with('/'),
            "downloadable model must use a safe runtime pull ref: {}",
            entry.model_id()
        );
    }
}

#[test]
fn model_seed_catalog_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-compatibility/src/seed_catalog.rs",
        include_str!("../src/seed_catalog.rs"),
        380,
    )
    .expect("model seed catalog source should stay focused");
}
