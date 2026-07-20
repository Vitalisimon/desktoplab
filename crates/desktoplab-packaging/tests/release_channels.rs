use desktoplab_packaging::ReleaseChannel;

#[test]
fn channel_signing_requirements_are_explicit() {
    assert!(ReleaseChannel::Dev.allows_unsigned_artifacts());
    assert!(!ReleaseChannel::Beta.allows_unsigned_artifacts());
    assert!(!ReleaseChannel::Stable.allows_unsigned_artifacts());

    assert!(!ReleaseChannel::Dev.requires_signed_artifact());
    assert!(ReleaseChannel::Beta.requires_signed_artifact());
    assert!(ReleaseChannel::Stable.requires_signed_artifact());
}

#[test]
fn channel_promotion_is_one_way_dev_to_beta_to_stable() {
    assert!(ReleaseChannel::Dev.can_promote_to(ReleaseChannel::Beta));
    assert!(ReleaseChannel::Beta.can_promote_to(ReleaseChannel::Stable));
    assert!(ReleaseChannel::Dev.can_promote_to(ReleaseChannel::Stable));

    assert!(!ReleaseChannel::Stable.can_promote_to(ReleaseChannel::Beta));
    assert!(!ReleaseChannel::Beta.can_promote_to(ReleaseChannel::Dev));
    assert!(!ReleaseChannel::Stable.can_promote_to(ReleaseChannel::Dev));
}

#[test]
fn stable_cannot_consume_unsigned_artifacts() {
    assert!(!ReleaseChannel::Stable.accepts_signature_state("unsigned_dev"));
    assert!(ReleaseChannel::Stable.accepts_signature_state("signed"));
    assert!(ReleaseChannel::Stable.accepts_signature_state("notarized"));
}
