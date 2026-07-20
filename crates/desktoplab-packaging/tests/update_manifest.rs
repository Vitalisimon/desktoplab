use desktoplab_packaging::{PlatformTarget, ReleaseChannel, SignatureState, UpdateManifestEntry};

#[test]
fn update_manifest_records_required_metadata() {
    let entry = signed_entry("1.4.2", ReleaseChannel::Beta);

    assert_eq!(entry.version(), "1.4.2");
    assert_eq!(entry.channel(), ReleaseChannel::Beta);
    assert_eq!(entry.platform(), PlatformTarget::MacosUniversal);
    assert_eq!(entry.url(), "https://releases.desktoplab.ai/desktoplab.dmg");
    assert_eq!(entry.sha256(), "a".repeat(64));
    assert_eq!(
        entry.release_notes_url(),
        "https://desktoplab.ai/releases/1.4.2"
    );
}

#[test]
fn update_manifest_blocks_downgrade_by_default() {
    let entry = signed_entry("1.4.1", ReleaseChannel::Beta);

    assert_eq!(
        entry.validate_version_transition("1.4.2"),
        Err("downgrade blocked by default".to_string())
    );
}

#[test]
fn emergency_rollback_must_be_explicit_and_signed() {
    let unsigned = UpdateManifestEntry::new(
        "1.4.1",
        ReleaseChannel::Dev,
        PlatformTarget::MacosUniversal,
        "https://releases.desktoplab.ai/desktoplab.dmg",
        "a".repeat(64),
        SignatureState::unsigned_dev(),
        "https://desktoplab.ai/releases/1.4.1",
    )
    .expect("valid unsigned dev manifest");

    assert_eq!(
        unsigned.emergency_rollback_from("1.4.2"),
        Err("emergency rollback requires signed update metadata".to_string())
    );

    let signed = signed_entry("1.4.1", ReleaseChannel::Beta);
    assert_eq!(signed.emergency_rollback_from("1.4.2"), Ok(()));
}

fn signed_entry(version: &str, channel: ReleaseChannel) -> UpdateManifestEntry {
    UpdateManifestEntry::new(
        version,
        channel,
        PlatformTarget::MacosUniversal,
        "https://releases.desktoplab.ai/desktoplab.dmg",
        "a".repeat(64),
        SignatureState::signed("Developer ID Application: DesktopLab", "sigstore:test"),
        format!("https://desktoplab.ai/releases/{version}"),
    )
    .expect("valid signed update manifest")
}
