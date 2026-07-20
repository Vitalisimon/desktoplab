use desktoplab_runtime::{RuntimeIntegrityState, RuntimeProvenance};

#[test]
fn runtime_provenance_marks_missing_hash_as_unavailable() {
    let provenance = RuntimeProvenance::for_runtime("runtime.ollama", Some("0.9.0"));

    assert_eq!(provenance.runtime_id(), "runtime.ollama");
    assert_eq!(provenance.version(), Some("0.9.0"));
    assert_eq!(
        provenance.install_source(),
        "signed_runtime_plan_or_existing_host_install"
    );
    assert_eq!(
        provenance.verification_method(),
        "binary detection plus local API health check"
    );
    assert_eq!(
        provenance.integrity().state(),
        RuntimeIntegrityState::Unavailable
    );
    assert!(provenance.integrity().reason().contains("unavailable"));
}

#[test]
fn runtime_provenance_distinguishes_external_runtimes() {
    let provenance = RuntimeProvenance::for_runtime("runtime.lm-studio", None);

    assert_eq!(provenance.install_source(), "external_app");
    assert_eq!(
        provenance.verification_method(),
        "OpenAI-compatible endpoint health check"
    );
}

#[test]
fn runtime_provenance_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-runtime/tests/runtime_provenance.rs",
        include_str!("runtime_provenance.rs"),
        90,
    )
    .expect("runtime provenance test should stay focused");
}
