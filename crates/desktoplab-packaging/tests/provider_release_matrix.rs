use desktoplab_packaging::{
    ArtifactManifestEntry, BuildSource, PackageArtifactSpec, PlatformTarget,
    ProviderAdapterReleaseSpec, ProviderProbeReleaseEvidence, ProviderReleaseMatrix,
    ProviderReleaseStatus, ReleaseChannel, SignatureState,
};

#[test]
fn release_matrix_requires_both_packaged_artifact_and_fresh_physical_probe() {
    let specs = [ProviderAdapterReleaseSpec::new(
        "backend.ollama",
        vec![PlatformTarget::MacosAarch64, PlatformTarget::WindowsX64],
    )];
    let artifacts = [
        artifact(PlatformTarget::MacosAarch64),
        artifact(PlatformTarget::WindowsX64),
    ];
    let probes = [
        ProviderProbeReleaseEvidence::new(
            "backend.ollama",
            PlatformTarget::MacosAarch64,
            true,
            90,
            120,
            "evidence:mac",
            "a".repeat(64),
        ),
        ProviderProbeReleaseEvidence::new(
            "backend.ollama",
            PlatformTarget::WindowsX64,
            true,
            80,
            99,
            "evidence:windows",
            "a".repeat(64),
        ),
    ];
    let matrix = ProviderReleaseMatrix::build(
        &specs,
        &artifacts,
        &probes,
        "0.1.0",
        ReleaseChannel::Dev,
        100,
    );

    assert_eq!(
        status(&matrix, PlatformTarget::MacosAarch64),
        ProviderReleaseStatus::Ready
    );
    assert_eq!(
        status(&matrix, PlatformTarget::WindowsX64),
        ProviderReleaseStatus::ProbeStale
    );
    assert_eq!(
        status(&matrix, PlatformTarget::LinuxX64),
        ProviderReleaseStatus::Unsupported
    );
}

#[test]
fn declarations_without_artifacts_or_probes_never_appear_ready() {
    let specs = [ProviderAdapterReleaseSpec::new(
        "backend.local",
        vec![PlatformTarget::LinuxX64, PlatformTarget::WindowsX64],
    )];
    let artifacts = [artifact(PlatformTarget::LinuxX64)];
    let probes = [ProviderProbeReleaseEvidence::new(
        "backend.local",
        PlatformTarget::WindowsX64,
        false,
        90,
        120,
        "evidence:failed",
        "a".repeat(64),
    )];
    let matrix = ProviderReleaseMatrix::build(
        &specs,
        &artifacts,
        &probes,
        "0.1.0",
        ReleaseChannel::Dev,
        100,
    );

    assert_eq!(
        status(&matrix, PlatformTarget::LinuxX64),
        ProviderReleaseStatus::ProbeMissing
    );
    assert_eq!(
        status(&matrix, PlatformTarget::WindowsX64),
        ProviderReleaseStatus::ArtifactMissing
    );
}

#[test]
fn probe_for_another_artifact_cannot_certify_current_package() {
    let specs = [ProviderAdapterReleaseSpec::new(
        "backend.local",
        vec![PlatformTarget::MacosAarch64],
    )];
    let artifacts = [artifact(PlatformTarget::MacosAarch64)];
    let probes = [ProviderProbeReleaseEvidence::new(
        "backend.local",
        PlatformTarget::MacosAarch64,
        true,
        90,
        120,
        "evidence:other-build",
        "c".repeat(64),
    )];
    let matrix = ProviderReleaseMatrix::build(
        &specs,
        &artifacts,
        &probes,
        "0.1.0",
        ReleaseChannel::Dev,
        100,
    );

    assert_eq!(
        status(&matrix, PlatformTarget::MacosAarch64),
        ProviderReleaseStatus::ProbeArtifactMismatch
    );
}

fn status(
    entries: &[desktoplab_packaging::ProviderReleaseMatrixEntry],
    target: PlatformTarget,
) -> ProviderReleaseStatus {
    entries
        .iter()
        .find(|entry| entry.target() == target)
        .unwrap()
        .status()
}

fn artifact(target: PlatformTarget) -> ArtifactManifestEntry {
    ArtifactManifestEntry::new(
        PackageArtifactSpec::new("desktoplab", "0.1.0", target, ReleaseChannel::Dev, "test")
            .unwrap(),
        "a".repeat(64),
        1,
        SignatureState::unsigned_dev(),
        BuildSource::new("b".repeat(40), "test", "test-runner"),
    )
    .unwrap()
}
