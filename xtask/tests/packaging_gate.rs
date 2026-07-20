use xtask::packaging::{
    PackagingArtifactRecord, PackagingGate, PackagingGateViolation, PackagingSignatureState,
    PackagingSourceBudget,
};

#[test]
fn packaging_gate_accepts_dev_unsigned_and_signed_release_artifacts() {
    let records = vec![
        artifact(
            "desktoplab-1.4.2-linux-x64-dev.AppImage",
            "linux-x64",
            "dev",
        )
        .with_signature_state(PackagingSignatureState::Unsigned),
        artifact(
            "desktoplab-1.4.2-macos-universal-beta.dmg",
            "macos-universal",
            "beta",
        )
        .with_signature_state(PackagingSignatureState::Signed),
    ];

    assert_eq!(PackagingGate::verify_artifacts(&records), Ok(()));
}

#[test]
fn packaging_gate_rejects_invalid_target_and_channel_with_exact_artifact() {
    let records = vec![artifact(
        "desktoplab-1.4.2-solaris-nightly.tar",
        "solaris",
        "nightly",
    )];

    assert_eq!(
        PackagingGate::verify_artifacts(&records),
        Err(PackagingGateViolation::InvalidTarget {
            file_name: "desktoplab-1.4.2-solaris-nightly.tar".to_string(),
            target: "solaris".to_string(),
        })
    );
}

#[test]
fn packaging_gate_rejects_unsigned_beta_and_stable_artifacts() {
    let records = vec![
        artifact(
            "desktoplab-1.4.2-windows-x64-stable.exe",
            "windows-x64",
            "stable",
        )
        .with_signature_state(PackagingSignatureState::Unsigned),
    ];

    assert_eq!(
        PackagingGate::verify_artifacts(&records),
        Err(PackagingGateViolation::UnsignedReleaseArtifact {
            file_name: "desktoplab-1.4.2-windows-x64-stable.exe".to_string(),
            channel: "stable".to_string(),
        })
    );
}

#[test]
fn packaging_gate_rejects_uppercase_sha256() {
    let records = vec![
        artifact(
            "desktoplab-1.4.2-linux-x64-dev.AppImage",
            "linux-x64",
            "dev",
        )
        .with_sha256("A".repeat(64)),
    ];

    assert_eq!(
        PackagingGate::verify_artifacts(&records),
        Err(PackagingGateViolation::InvalidSha256 {
            file_name: "desktoplab-1.4.2-linux-x64-dev.AppImage".to_string(),
        })
    );
}

#[test]
fn packaging_gate_rejects_source_files_over_line_budget() {
    let budgets = vec![PackagingSourceBudget::new(
        "crates/desktoplab-packaging/src/artifact.rs",
        "pub fn one() {}\npub fn two() {}\n",
        1,
    )];

    assert_eq!(
        PackagingGate::verify_source_budgets(&budgets),
        Err(PackagingGateViolation::LineBudgetExceeded {
            path: "crates/desktoplab-packaging/src/artifact.rs".to_string(),
            logical_lines: 2,
            max_lines: 1,
        })
    );
}

fn artifact(
    file_name: &'static str,
    target: &'static str,
    channel: &'static str,
) -> PackagingArtifactRecord {
    PackagingArtifactRecord::new(
        file_name,
        target,
        channel,
        "a".repeat(64),
        42_000_000,
        PackagingSignatureState::Signed,
    )
}
