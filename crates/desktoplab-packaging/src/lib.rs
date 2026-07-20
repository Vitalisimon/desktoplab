#![forbid(unsafe_code)]

mod artifact;
mod channel;
mod manifest;
mod platform;
mod provider_release;
mod smoke;
mod update_manifest;

pub use artifact::{ArtifactSpecError, GeneratedArtifactPath, PackageArtifactSpec};
pub use channel::ReleaseChannel;
pub use manifest::{ArtifactManifestEntry, BuildSource, ManifestEntryError, SignatureState};
pub use platform::{InstallerUserDataPolicy, LinuxPackageKind, PlatformTarget};
pub use provider_release::{
    ProviderAdapterReleaseSpec, ProviderProbeReleaseEvidence, ProviderReleaseMatrix,
    ProviderReleaseMatrixEntry, ProviderReleaseStatus,
};
pub use smoke::{PackagingSmokeResult, SmokeState};
pub use update_manifest::{UpdateManifestEntry, UpdateManifestError};
