use desktoplab_runtime::{
    RuntimeLifecycleCheckpoint, RuntimeLifecycleFailureClass, RuntimeLifecycleOwnership,
    RuntimeLifecyclePhase, RuntimeLifecycleRecovery, read_runtime_lifecycle,
    write_runtime_lifecycle,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn lifecycle_advances_exactly_and_is_idempotent_at_current_phase() {
    let mut checkpoint = RuntimeLifecycleCheckpoint::new(
        "runtime.mlx-lm",
        RuntimeLifecycleOwnership::DesktopLabManaged,
    );
    assert_eq!(checkpoint.phase(), RuntimeLifecyclePhase::Detect);
    checkpoint
        .advance(RuntimeLifecyclePhase::Detect)
        .expect("same phase is idempotent");
    checkpoint
        .advance(RuntimeLifecyclePhase::Plan)
        .expect("next phase");
    assert_eq!(
        checkpoint.advance(RuntimeLifecyclePhase::Acquire),
        Err("runtime lifecycle phase must advance exactly once")
    );
}

#[test]
fn recovery_is_selected_runtime_scoped_and_failure_aware() {
    let mut checkpoint = RuntimeLifecycleCheckpoint::new(
        "runtime.lm-studio",
        RuntimeLifecycleOwnership::DesktopLabManaged,
    );
    checkpoint
        .advance(RuntimeLifecyclePhase::Plan)
        .expect("next phase");
    assert_eq!(
        checkpoint.recovery("runtime.ollama"),
        RuntimeLifecycleRecovery::Blocked("selected_runtime_mismatch".to_string())
    );
    checkpoint.fail(
        RuntimeLifecycleFailureClass::Infrastructure,
        "driver token=private failed",
    );
    assert_eq!(
        checkpoint.recovery("runtime.lm-studio"),
        RuntimeLifecycleRecovery::AwaitOperator(RuntimeLifecyclePhase::Plan)
    );
    checkpoint.fail(
        RuntimeLifecycleFailureClass::Retryable,
        "network unavailable",
    );
    assert_eq!(
        checkpoint.recovery("runtime.lm-studio"),
        RuntimeLifecycleRecovery::Resume(RuntimeLifecyclePhase::Plan)
    );
    checkpoint.fail(RuntimeLifecycleFailureClass::Terminal, "license rejected");
    assert_eq!(
        checkpoint.recovery("runtime.lm-studio"),
        RuntimeLifecycleRecovery::Blocked("license rejected".to_string())
    );
}

#[test]
fn lifecycle_checkpoint_round_trips_atomically_without_secrets() {
    let fixture = TempDir::new().expect("fixture");
    let path = fixture.path().join("runtime").join("lifecycle.json");
    let mut checkpoint =
        RuntimeLifecycleCheckpoint::new("runtime.ollama", RuntimeLifecycleOwnership::UserOwned);
    checkpoint.fail(
        RuntimeLifecycleFailureClass::Retryable,
        "password=private endpoint unavailable",
    );
    write_runtime_lifecycle(&path, &checkpoint).expect("write checkpoint");

    let stored = std::fs::read_to_string(&path).expect("stored checkpoint");
    assert!(!stored.contains("private"));
    assert!(!fixture.path().join("runtime/lifecycle.json.tmp").exists());
    let recovered = read_runtime_lifecycle(&path)
        .expect("read checkpoint")
        .expect("checkpoint");
    assert_eq!(recovered, checkpoint);
    assert_eq!(recovered.ownership(), RuntimeLifecycleOwnership::UserOwned);
}

#[test]
fn lifecycle_source_stays_focused() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/runtime_lifecycle.rs",
        include_str!("../src/runtime_lifecycle.rs"),
        300,
    )
    .expect("unified lifecycle should stay focused");
}
