use desktoplab_compatibility::{
    AcceleratorConfidence, AcceleratorProfile, BackendCapabilitySet, Channel, ChannelPolicy,
    CompatibilityCatalog, CompatibilityEngine, CompatibilityEvidence, CompatibilityStatus,
    HardwareProfile, LocalOverride, MatchRequest, ModelManifest, RecommendationDecision,
    RuntimeManifest,
};
use xtask::check_logical_line_limit;

#[test]
fn known_blocked_combinations_are_never_recommended() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.ollama", &["gguf"]))
        .with_model(model("model.qwen3", "gguf", 8))
        .with_blocked_combination(
            "runtime.ollama",
            "model.qwen3",
            "known runtime/model regression",
        );
    let engine = CompatibilityEngine::new(catalog);
    let decision = engine.evaluate(request("runtime.ollama", "model.qwen3"));

    assert_eq!(decision.status(), CompatibilityStatus::Blocked);
    assert_eq!(decision.reason(), "known runtime/model regression");
    assert!(!decision.is_recommended());
}

#[test]
fn model_compatibility_is_data_driven_by_format_and_hardware_requirements() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.llama-cpp", &["gguf"]))
        .with_model(model("model.small", "gguf", 8))
        .with_model(model("model.large", "gguf", 64));
    let engine = CompatibilityEngine::new(catalog)
        .with_hardware(HardwareProfile::new("macos", "aarch64", 32));

    assert_eq!(
        engine
            .evaluate(request("runtime.llama-cpp", "model.small"))
            .status(),
        CompatibilityStatus::Recommended
    );
    assert_eq!(
        engine
            .evaluate(request("runtime.llama-cpp", "model.large"))
            .status(),
        CompatibilityStatus::Unsupported
    );
}

#[test]
fn runtime_compatibility_is_data_driven_by_supported_model_formats() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.mlx", &["mlx"]))
        .with_model(model("model.gguf", "gguf", 8));
    let engine = CompatibilityEngine::new(catalog);
    let decision = engine.evaluate(request("runtime.mlx", "model.gguf"));

    assert_eq!(decision.status(), CompatibilityStatus::Unsupported);
    assert_eq!(
        decision.reason(),
        "runtime does not support model format gguf"
    );
}

#[test]
fn backend_capability_sets_filter_route_compatibility() {
    let capabilities = BackendCapabilitySet::new("backend.local")
        .with_capability("llm.chat")
        .with_capability("agent.events.stream");

    assert!(capabilities.satisfies(&["llm.chat"]));
    assert!(!capabilities.satisfies(&["llm.chat", "tools.filesystem.write"]));
}

#[test]
fn channel_policy_blocks_beta_when_stable_only_is_required() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.ollama", &["gguf"]))
        .with_model(model("model.beta", "gguf", 8).with_channel(Channel::Beta))
        .with_evidence(CompatibilityEvidence::new("evidence.beta", 80));
    let engine = CompatibilityEngine::new(catalog).with_channel_policy(ChannelPolicy::StableOnly);
    let decision = engine.evaluate(request("runtime.ollama", "model.beta"));

    assert_eq!(decision.status(), CompatibilityStatus::Blocked);
    assert_eq!(decision.reason(), "channel beta is blocked by policy");
}

#[test]
fn local_override_can_enable_experimental_but_not_security_blocks() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(
            runtime("runtime.experimental", &["gguf"]).with_channel(Channel::Experimental),
        )
        .with_model(model("model.experimental", "gguf", 8).with_channel(Channel::Experimental))
        .with_blocked_combination(
            "runtime.experimental",
            "model.experimental",
            "security advisory",
        );
    let engine = CompatibilityEngine::new(catalog)
        .with_channel_policy(ChannelPolicy::StableOnly)
        .with_local_override(LocalOverride::allow_experimental());
    let decision = engine.evaluate(request("runtime.experimental", "model.experimental"));

    assert_eq!(decision.status(), CompatibilityStatus::Blocked);
    assert_eq!(decision.reason(), "security advisory");
    assert!(!decision.is_recommended());
}

#[test]
fn recommendation_requires_evidence() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.ollama", &["gguf"]))
        .with_model(model("model.qwen3", "gguf", 8));
    let engine = CompatibilityEngine::new(catalog).with_minimum_evidence_score(60);
    let decision = engine.evaluate(request("runtime.ollama", "model.qwen3"));

    assert_eq!(decision.status(), CompatibilityStatus::Compatible);
    assert_eq!(
        decision.recommendation(),
        RecommendationDecision::CompatibleNotRecommended
    );
}

#[test]
fn unknown_accelerator_evidence_cannot_promote_high_confidence_recommendations() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.ollama", &["gguf"]))
        .with_model(model("model.nemotron-70b", "gguf", 96))
        .with_evidence(CompatibilityEvidence::new("evidence.bench", 95));
    let hardware = HardwareProfile::new("linux", "x86_64", 192)
        .with_accelerator(AcceleratorProfile::unknown());
    let engine = CompatibilityEngine::new(catalog).with_hardware(hardware);
    let decision = engine.evaluate(request("runtime.ollama", "model.nemotron-70b"));

    assert_eq!(decision.status(), CompatibilityStatus::Compatible);
    assert_eq!(
        decision.recommendation(),
        RecommendationDecision::CompatibleNotRecommended
    );
    assert!(
        decision
            .reason()
            .contains("accelerator confidence is unknown")
    );
}

#[test]
fn confirmed_accelerator_evidence_can_support_recommendations() {
    let catalog = CompatibilityCatalog::new()
        .with_runtime(runtime("runtime.ollama", &["gguf"]))
        .with_model(model("model.nemotron-70b", "gguf", 96))
        .with_evidence(CompatibilityEvidence::new("evidence.bench", 95));
    let hardware = HardwareProfile::new("linux", "x86_64", 192).with_accelerator(
        AcceleratorProfile::new("nvidia", "discrete", 96, 192, "verified")
            .with_confidence(AcceleratorConfidence::Confirmed),
    );
    let engine = CompatibilityEngine::new(catalog).with_hardware(hardware);

    assert_eq!(
        engine
            .evaluate(request("runtime.ollama", "model.nemotron-70b"))
            .status(),
        CompatibilityStatus::Recommended
    );
}

#[test]
fn compatibility_source_files_stay_below_initial_line_count_guard() {
    let (path, source) = (
        "crates/desktoplab-compatibility/src/lib.rs",
        include_str!("../src/lib.rs"),
    );

    check_logical_line_limit(path, source, 250)
        .expect("compatibility source should stay below the initial line-count guard");
}

fn runtime(id: &str, supported_formats: &[&str]) -> RuntimeManifest {
    RuntimeManifest::new(id, supported_formats)
}

fn model(id: &str, format: &str, required_memory_gb: u32) -> ModelManifest {
    ModelManifest::new(id, format, required_memory_gb)
}

fn request(runtime_id: &str, model_id: &str) -> MatchRequest {
    MatchRequest::new(runtime_id, model_id)
}
