use desktoplab_desktop_lib::updater::{
    evaluate_macos_update, finalize_macos_update_attempt, MacosUpdateAttempt,
    MacosUpdateCandidate, MacosUpdateChannel, MacosUpdateDecision, MacosUpdateSignature,
};

#[test]
fn stable_macos_updates_refuse_unsigned_or_only_signed_artifacts() {
    let unsigned = MacosUpdateCandidate::new(
        MacosUpdateChannel::Stable,
        MacosUpdateSignature::Unsigned,
        false,
    );
    let signed = MacosUpdateCandidate::new(
        MacosUpdateChannel::Stable,
        MacosUpdateSignature::Signed,
        false,
    );
    let notarized = MacosUpdateCandidate::new(
        MacosUpdateChannel::Stable,
        MacosUpdateSignature::Notarized,
        false,
    );

    assert_eq!(
        evaluate_macos_update(unsigned),
        MacosUpdateDecision::Blocked("stable macOS updates require notarization")
    );
    assert_eq!(
        evaluate_macos_update(signed),
        MacosUpdateDecision::Blocked("stable macOS updates require notarization")
    );
    assert_eq!(evaluate_macos_update(notarized), MacosUpdateDecision::Allowed);
}

#[test]
fn dev_channel_allows_unsigned_local_update_fixtures_only() {
    let local_fixture =
        MacosUpdateCandidate::new(MacosUpdateChannel::Dev, MacosUpdateSignature::Unsigned, true);
    let remote_unsigned =
        MacosUpdateCandidate::new(MacosUpdateChannel::Dev, MacosUpdateSignature::Unsigned, false);

    assert_eq!(evaluate_macos_update(local_fixture), MacosUpdateDecision::Allowed);
    assert_eq!(
        evaluate_macos_update(remote_unsigned),
        MacosUpdateDecision::Blocked("unsigned dev update must be a local fixture")
    );
}

#[test]
fn failed_update_attempt_leaves_existing_app_usable() {
    assert_eq!(
        finalize_macos_update_attempt(false),
        MacosUpdateAttempt::ExistingAppStillUsable
    );
    assert_eq!(finalize_macos_update_attempt(true), MacosUpdateAttempt::Updated);
}

#[test]
fn macos_update_boundary_test_stays_small() {
    xtask::check_logical_line_limit(
        "apps/desktop/src-tauri/tests/macos_update_boundary.rs",
        include_str!("macos_update_boundary.rs"),
        140,
    )
    .expect("macos update boundary test should stay focused");
}
