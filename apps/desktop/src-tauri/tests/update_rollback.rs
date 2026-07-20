use desktoplab_desktop_lib::updater::{
    evaluate_update_manifest, finalize_update_install, verify_update_artifact, UpdateChannel,
    UpdateDeliveryState, UpdateInstallResult, UpdateManifestTrust,
};

const EXPECTED_SHA256: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const ACTUAL_SHA256: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn hash_mismatch_blocks_update_before_install() {
    assert_eq!(
        verify_update_artifact(EXPECTED_SHA256, ACTUAL_SHA256),
        Err("update artifact hash mismatch")
    );
    assert_eq!(verify_update_artifact(EXPECTED_SHA256, EXPECTED_SHA256), Ok(()));
}

#[test]
fn unsigned_stable_update_manifest_is_blocked() {
    assert_eq!(
        evaluate_update_manifest(UpdateDeliveryState::EnabledWithVerifiedKey, UpdateChannel::Stable, UpdateManifestTrust::UnsignedRemote),
        Err("beta and stable update manifests require signing")
    );
}

#[test]
fn failed_install_reports_recoverable_failure() {
    assert_eq!(
        finalize_update_install(false),
        UpdateInstallResult::RecoverableFailure("existing app remains usable")
    );
    assert_eq!(finalize_update_install(true), UpdateInstallResult::Installed);
}

#[test]
fn update_rollback_test_stays_small() {
    xtask::check_logical_line_limit(
        "apps/desktop/src-tauri/tests/update_rollback.rs",
        include_str!("update_rollback.rs"),
        120,
    )
    .expect("update rollback test should stay focused");
}
