use desktoplab_model_manager::{AgentContextWindowPolicy, AgentRequestTimeoutPolicy, ModelManager};

#[test]
fn context_window_scales_with_model_and_wizard_memory_headroom() {
    let catalog = ModelManager::new().default_family_catalog();
    let gemma = catalog
        .variants()
        .iter()
        .find(|variant| variant.model_id() == "model.gemma4-12b-q4")
        .unwrap();

    assert_eq!(AgentContextWindowPolicy::for_variant(gemma, 16), 8_192);
    assert_eq!(AgentContextWindowPolicy::for_variant(gemma, 24), 32_768);
    assert_eq!(AgentContextWindowPolicy::for_variant(gemma, 36), 65_536);
    assert_eq!(AgentContextWindowPolicy::for_variant(gemma, 64), 131_072);
    assert_eq!(AgentContextWindowPolicy::for_variant(gemma, 128), 256_000);
}

#[test]
fn context_window_never_exceeds_the_model_limit() {
    assert_eq!(
        AgentContextWindowPolicy::from_capacity(16_384, 8, 128),
        16_384
    );
}

#[test]
fn request_timeout_scales_with_model_memory_headroom() {
    assert_eq!(AgentRequestTimeoutPolicy::from_capacity(16, 16), 600);
    assert_eq!(AgentRequestTimeoutPolicy::from_capacity(16, 24), 300);
    assert_eq!(AgentRequestTimeoutPolicy::from_capacity(16, 36), 240);
    assert_eq!(AgentRequestTimeoutPolicy::from_capacity(16, 64), 180);
    assert_eq!(AgentRequestTimeoutPolicy::from_capacity(16, 128), 120);
}
