use std::fs;
use std::path::PathBuf;

use xtask::packaging::{
    PACKAGING_MANIFEST_SCHEMA_VERSION, PackagingSignatureState, UpdateChannelManifest,
    UpdateManifestPolicy, UpdateManifestPolicyViolation, channel_manifest_path,
};
use xtask::packaging_manifest::{ArtifactManifestGenerator, ArtifactManifestInput};

#[test]
fn generator_computes_sha256_size_and_deterministic_json() {
    let fixture = fixture_path("desktoplab-artifact.bin");
    fs::write(&fixture, b"desktoplab").expect("fixture should write");

    let manifest = ArtifactManifestGenerator::generate(&[ArtifactManifestInput::new(
        &fixture,
        "macos-aarch64",
        "dev",
        PackagingSignatureState::Unsigned,
        "commit:abcdef",
    )])
    .expect("manifest should generate");

    assert_eq!(manifest.schema_version(), PACKAGING_MANIFEST_SCHEMA_VERSION);
    assert_eq!(manifest.entries()[0].size_bytes(), 10);
    assert_eq!(
        manifest.entries()[0].sha256(),
        "c4b3f8df721b8c11292a4baee1043250a66a39b66ac2d4ec6b77abf195e16dec"
    );
    assert_eq!(manifest.to_json(), manifest.to_json());
}

#[test]
fn generator_refuses_missing_artifacts() {
    let missing = fixture_path("missing-artifact.bin");
    let error = ArtifactManifestGenerator::generate(&[ArtifactManifestInput::new(
        &missing,
        "linux-x64",
        "dev",
        PackagingSignatureState::Unsigned,
        "local",
    )])
    .expect_err("missing artifact must fail");

    assert!(error.to_string().contains("missing artifact"));
}

#[test]
fn update_channel_manifest_paths_are_explicit() {
    assert_eq!(
        channel_manifest_path("beta", "linux-x64").expect("path should be valid"),
        "channels/beta/linux-x64/manifest.json"
    );
}

#[test]
fn update_channel_policy_rejects_unsigned_remote_and_rollback() {
    let remote_dev = UpdateChannelManifest::new(
        "dev",
        "linux-x64",
        "https://updates.desktoplab.ai/dev/linux-x64/DesktopLab.AppImage",
        PackagingSignatureState::Unsigned,
        false,
    );
    assert_eq!(
        UpdateManifestPolicy::verify(&remote_dev),
        Err(UpdateManifestPolicyViolation::UnsignedRemoteDev)
    );

    let rollback = UpdateChannelManifest::new(
        "dev",
        "linux-x64",
        "file:///tmp/DesktopLab.AppImage",
        PackagingSignatureState::Unsigned,
        true,
    );
    assert_eq!(
        UpdateManifestPolicy::verify(&rollback),
        Err(UpdateManifestPolicyViolation::UnsignedRollback)
    );
}

#[test]
fn packaging_manifest_test_stays_small() {
    xtask::check_logical_line_limit(
        "xtask/tests/packaging_manifest.rs",
        include_str!("packaging_manifest.rs"),
        140,
    )
    .expect("packaging manifest test should stay focused");
}

fn fixture_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("desktoplab-{name}-{}", std::process::id()));
    path
}
