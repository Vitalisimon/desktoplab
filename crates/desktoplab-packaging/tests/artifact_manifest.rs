use desktoplab_packaging::{
    ArtifactManifestEntry, BuildSource, PackageArtifactSpec, PlatformTarget, ReleaseChannel,
    SignatureState,
};
use xtask::check_logical_line_limit;

#[test]
fn manifest_entry_records_publish_evidence() {
    let entry = signed_entry(ReleaseChannel::Beta);

    assert_eq!(entry.target(), PlatformTarget::MacosUniversal);
    assert_eq!(entry.channel(), ReleaseChannel::Beta);
    assert_eq!(entry.version(), "1.4.2");
    assert_eq!(
        entry.file_name(),
        "desktoplab-1.4.2-macos-universal-beta.dmg"
    );
    assert_eq!(entry.sha256(), "a".repeat(64));
    assert_eq!(entry.size_bytes(), 42_000_000);
    assert!(entry.signature_state().is_publish_signed());
    assert_eq!(entry.build_source().commit_sha(), "b".repeat(40));
}

#[test]
fn macos_app_and_dmg_are_distinct_artifact_specs() {
    let app = PackageArtifactSpec::macos_app(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosAarch64,
        ReleaseChannel::Dev,
    )
    .expect("valid macOS app artifact spec");
    let dmg = PackageArtifactSpec::macos_dmg(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosAarch64,
        ReleaseChannel::Dev,
    )
    .expect("valid macOS dmg artifact spec");

    assert!(app.is_macos_app_bundle());
    assert!(!app.is_macos_dmg());
    assert_eq!(app.file_name(), "desktoplab-1.4.2-macos-aarch64-dev.app");

    assert!(dmg.is_macos_dmg());
    assert!(!dmg.is_macos_app_bundle());
    assert_eq!(dmg.file_name(), "desktoplab-1.4.2-macos-aarch64-dev.dmg");
}

#[test]
fn macos_dmg_manifest_records_checksum_and_notarization_state() {
    let spec = PackageArtifactSpec::macos_dmg(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosUniversal,
        ReleaseChannel::Stable,
    )
    .expect("valid macOS dmg artifact spec");

    let entry = ArtifactManifestEntry::new(
        spec,
        "c".repeat(64),
        51_000_000,
        SignatureState::notarized(
            "Developer ID Application: DesktopLab",
            "codesign:sha256:test",
            "notarytool-log-id",
            "spctl accepted",
        ),
        BuildSource::new("d".repeat(40), "release", "macos-runner"),
    )
    .expect("valid notarized manifest entry");

    assert_eq!(
        entry.file_name(),
        "desktoplab-1.4.2-macos-universal-stable.dmg"
    );
    assert_eq!(entry.sha256(), "c".repeat(64));
    assert!(entry.signature_state().is_publish_signed());
    assert!(entry.signature_state().is_notarized());
    assert_eq!(entry.validate_for_publish(), Ok(()));
}

#[test]
fn unsigned_artifacts_are_allowed_only_on_dev_channel() {
    let dev = unsigned_entry(ReleaseChannel::Dev);
    let beta = unsigned_entry(ReleaseChannel::Beta);
    let stable = unsigned_entry(ReleaseChannel::Stable);

    assert_eq!(dev.validate_for_publish(), Ok(()));
    assert_eq!(
        beta.validate_for_publish(),
        Err("beta artifacts require signing fields before publish".to_string())
    );
    assert_eq!(
        stable.validate_for_publish(),
        Err("stable artifacts require signing fields before publish".to_string())
    );
}

#[test]
fn signed_manifest_entries_reject_blank_signature_fields() {
    let spec = PackageArtifactSpec::macos_dmg(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosUniversal,
        ReleaseChannel::Beta,
    )
    .expect("valid macOS dmg artifact spec");

    let error = ArtifactManifestEntry::new(
        spec,
        "a".repeat(64),
        42_000_000,
        SignatureState::signed("", "sigstore:test"),
        BuildSource::new("b".repeat(40), "local", "developer-mac"),
    )
    .expect_err("blank signature identity must be rejected");

    assert_eq!(error, "signatureState.identity must not be blank");
}

#[test]
fn manifest_rejects_missing_integrity_and_build_source() {
    let spec = PackageArtifactSpec::new(
        "desktoplab",
        "1.4.2",
        PlatformTarget::LinuxX64,
        ReleaseChannel::Dev,
        "AppImage",
    )
    .expect("valid package artifact spec");

    let error = ArtifactManifestEntry::new(
        spec,
        "",
        0,
        SignatureState::unsigned_dev(),
        BuildSource::new("", "local", "developer-mac"),
    )
    .expect_err("manifest must reject missing evidence");

    assert_eq!(error, "sha256 must be a 64 character lowercase hex digest");
}

#[test]
fn manifest_rejects_uppercase_sha256() {
    let spec = PackageArtifactSpec::new(
        "desktoplab",
        "1.4.2",
        PlatformTarget::LinuxX64,
        ReleaseChannel::Dev,
        "AppImage",
    )
    .expect("valid package artifact spec");

    let error = ArtifactManifestEntry::new(
        spec,
        "A".repeat(64),
        42_000_000,
        SignatureState::unsigned_dev(),
        BuildSource::new("b".repeat(40), "local", "developer-mac"),
    )
    .expect_err("uppercase sha256 must be rejected");

    assert_eq!(error, "sha256 must be a 64 character lowercase hex digest");
}

#[test]
fn artifact_manifest_paths_stay_under_generated_packaging_directory() {
    let spec = PackageArtifactSpec::new(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosUniversal,
        ReleaseChannel::Dev,
        "dmg",
    )
    .expect("valid package artifact spec");

    assert_eq!(
        spec.generated_artifact_path().as_str(),
        "dist/desktoplab-packaging/desktoplab-1.4.2-macos-universal-dev.dmg"
    );
    assert_eq!(
        PackageArtifactSpec::manifest_path().as_str(),
        "dist/desktoplab-packaging/artifacts.json"
    );
}

#[test]
fn artifact_manifest_source_stays_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-packaging/src/manifest.rs",
        include_str!("../src/manifest.rs"),
        220,
    )
    .expect("manifest contract source should stay below the initial line-count guard");
}

fn unsigned_entry(channel: ReleaseChannel) -> ArtifactManifestEntry {
    let spec = PackageArtifactSpec::new(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosUniversal,
        channel,
        "dmg",
    )
    .expect("valid package artifact spec");

    ArtifactManifestEntry::new(
        spec,
        "a".repeat(64),
        42_000_000,
        SignatureState::unsigned_dev(),
        BuildSource::new("b".repeat(40), "local", "developer-mac"),
    )
    .expect("valid manifest entry")
}

fn signed_entry(channel: ReleaseChannel) -> ArtifactManifestEntry {
    let spec = PackageArtifactSpec::new(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosUniversal,
        channel,
        "dmg",
    )
    .expect("valid package artifact spec");

    ArtifactManifestEntry::new(
        spec,
        "a".repeat(64),
        42_000_000,
        SignatureState::signed("Developer ID Application: DesktopLab", "sigstore:test"),
        BuildSource::new("b".repeat(40), "local", "developer-mac"),
    )
    .expect("valid manifest entry")
}
