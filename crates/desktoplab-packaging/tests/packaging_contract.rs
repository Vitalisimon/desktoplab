use desktoplab_packaging::{LinuxPackageKind, PackageArtifactSpec, PlatformTarget, ReleaseChannel};
use xtask::check_logical_line_limit;

#[test]
fn supported_targets_are_explicit_and_stable() {
    let targets: Vec<_> = PlatformTarget::supported()
        .iter()
        .map(|target| target.as_str())
        .collect();

    assert_eq!(
        targets,
        vec![
            "macos-universal",
            "macos-aarch64",
            "macos-x64",
            "windows-x64",
            "linux-x64",
        ]
    );
}

#[test]
fn macos_targets_are_explicit_and_do_not_imply_universal_certification() {
    let targets: Vec<_> = PlatformTarget::macos_targets()
        .iter()
        .map(|target| target.as_str())
        .collect();

    assert_eq!(
        targets,
        vec!["macos-aarch64", "macos-x64", "macos-universal"]
    );
}

#[test]
fn windows_targets_start_with_x64_nsis_only() {
    let targets: Vec<_> = PlatformTarget::windows_targets()
        .iter()
        .map(|target| target.as_str())
        .collect();

    assert_eq!(targets, vec!["windows-x64"]);
}

#[test]
fn linux_targets_are_x64_with_separate_package_formats() {
    let targets: Vec<_> = PlatformTarget::linux_targets()
        .iter()
        .map(|target| target.as_str())
        .collect();
    let packages: Vec<_> = LinuxPackageKind::supported()
        .iter()
        .map(|package| package.as_str())
        .collect();

    assert_eq!(targets, vec!["linux-x64"]);
    assert_eq!(packages, vec!["AppImage", "deb", "rpm"]);
}

#[test]
fn release_channels_are_explicit_and_ordered_by_risk() {
    let channels: Vec<_> = ReleaseChannel::supported()
        .iter()
        .map(|channel| channel.as_str())
        .collect();

    assert_eq!(channels, vec!["dev", "beta", "stable"]);
    assert!(ReleaseChannel::Dev.allows_unsigned_artifacts());
    assert!(!ReleaseChannel::Beta.allows_unsigned_artifacts());
    assert!(!ReleaseChannel::Stable.allows_unsigned_artifacts());
}

#[test]
fn artifact_names_include_product_version_target_channel_and_extension() {
    let artifact = PackageArtifactSpec::new(
        "desktoplab",
        "1.4.2",
        PlatformTarget::MacosUniversal,
        ReleaseChannel::Beta,
        "dmg",
    )
    .expect("valid package artifact spec");

    assert_eq!(
        artifact.file_name(),
        "desktoplab-1.4.2-macos-universal-beta.dmg"
    );
}

#[test]
fn artifact_names_reject_ambiguous_inputs() {
    assert!(
        PackageArtifactSpec::new(
            "",
            "1.4.2",
            PlatformTarget::LinuxX64,
            ReleaseChannel::Dev,
            "AppImage"
        )
        .is_err()
    );
    assert!(
        PackageArtifactSpec::new(
            "desktoplab",
            "",
            PlatformTarget::LinuxX64,
            ReleaseChannel::Dev,
            "AppImage"
        )
        .is_err()
    );
    assert!(
        PackageArtifactSpec::new(
            "desktoplab",
            "1.4.2",
            PlatformTarget::LinuxX64,
            ReleaseChannel::Dev,
            ""
        )
        .is_err()
    );
}

#[test]
fn packaging_contract_sources_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-packaging/src/artifact.rs",
        include_str!("../src/artifact.rs"),
        180,
    )
    .expect("artifact contract source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-packaging/src/channel.rs",
        include_str!("../src/channel.rs"),
        80,
    )
    .expect("channel contract source should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-packaging/src/platform.rs",
        include_str!("../src/platform.rs"),
        120,
    )
    .expect("platform contract source should stay below the initial line-count guard");
}
