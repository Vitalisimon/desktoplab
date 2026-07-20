use desktoplab_desktop_lib::updater::{
    evaluate_update_manifest, update_ui_state, UpdateChannel, UpdateDeliveryState,
    UpdateManifestTrust, UpdateUiCheckState, UpdateUiInteraction,
};

#[test]
fn tauri_bundle_contains_no_placeholder_update_channel_or_key() {
    let config = include_str!("../tauri.conf.json");

    assert!(!config.contains("\"updater\""));
    assert!(!config.contains("releases.desktoplab.ai"));
    assert!(!config.contains("PACKAGING_GATE_REQUIRES_REAL_UPDATER_PUBKEY"));
}

#[test]
fn current_build_rejects_every_manifest_before_network_or_install() {
    for trust in [
        UpdateManifestTrust::LocalFixture,
        UpdateManifestTrust::UnsignedRemote,
        UpdateManifestTrust::Signed,
        UpdateManifestTrust::Notarized,
    ] {
        assert_eq!(
            evaluate_update_manifest(UpdateDeliveryState::Disabled, UpdateChannel::Beta, trust),
            Err("update delivery is disabled for this build")
        );
    }
}

#[test]
fn dev_channel_can_use_local_fixture_manifest_only() {
    assert_eq!(
        evaluate_update_manifest(UpdateDeliveryState::EnabledWithVerifiedKey, UpdateChannel::Dev, UpdateManifestTrust::LocalFixture),
        Ok(())
    );
    assert_eq!(
        evaluate_update_manifest(UpdateDeliveryState::EnabledWithVerifiedKey, UpdateChannel::Dev, UpdateManifestTrust::UnsignedRemote),
        Err("unsigned remote update manifest is not trusted")
    );
}

#[test]
fn beta_and_stable_channels_require_signed_manifest_state() {
    for channel in [UpdateChannel::Beta, UpdateChannel::Stable] {
        assert_eq!(
            evaluate_update_manifest(UpdateDeliveryState::EnabledWithVerifiedKey, channel, UpdateManifestTrust::UnsignedRemote),
            Err("beta and stable update manifests require signing")
        );
        assert_eq!(
            evaluate_update_manifest(UpdateDeliveryState::EnabledWithVerifiedKey, channel, UpdateManifestTrust::Signed),
            Ok(())
        );
        assert_eq!(
            evaluate_update_manifest(UpdateDeliveryState::EnabledWithVerifiedKey, channel, UpdateManifestTrust::Notarized),
            Ok(())
        );
    }
}

#[test]
fn update_ui_remains_read_only_until_check_succeeds() {
    assert_eq!(
        update_ui_state(UpdateUiCheckState::NotChecked),
        UpdateUiInteraction::ReadOnly
    );
    assert_eq!(
        update_ui_state(UpdateUiCheckState::CheckFailed),
        UpdateUiInteraction::ReadOnly
    );
    assert_eq!(
        update_ui_state(UpdateUiCheckState::VerifiedNoUpdate),
        UpdateUiInteraction::ReadOnly
    );
    assert_eq!(
        update_ui_state(UpdateUiCheckState::VerifiedUpdateAvailable),
        UpdateUiInteraction::Actionable
    );
}

#[test]
fn updater_config_test_stays_small() {
    xtask::check_logical_line_limit(
        "apps/desktop/src-tauri/tests/updater_config.rs",
        include_str!("updater_config.rs"),
        150,
    )
    .expect("updater config test should stay focused");
}
