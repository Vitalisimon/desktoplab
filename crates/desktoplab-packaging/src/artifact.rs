use crate::{PlatformTarget, ReleaseChannel};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedArtifactPath {
    value: String,
}

impl GeneratedArtifactPath {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageArtifactSpec {
    product: String,
    version: String,
    target: PlatformTarget,
    channel: ReleaseChannel,
    extension: String,
}

impl PackageArtifactSpec {
    pub fn macos_app(
        product: impl Into<String>,
        version: impl Into<String>,
        target: PlatformTarget,
        channel: ReleaseChannel,
    ) -> Result<Self, ArtifactSpecError> {
        Self::new(product, version, target, channel, "app")
    }

    pub fn macos_dmg(
        product: impl Into<String>,
        version: impl Into<String>,
        target: PlatformTarget,
        channel: ReleaseChannel,
    ) -> Result<Self, ArtifactSpecError> {
        Self::new(product, version, target, channel, "dmg")
    }

    pub fn new(
        product: impl Into<String>,
        version: impl Into<String>,
        target: PlatformTarget,
        channel: ReleaseChannel,
        extension: impl Into<String>,
    ) -> Result<Self, ArtifactSpecError> {
        let product = product.into();
        let version = version.into();
        let extension = extension.into();

        reject_blank("product", &product)?;
        reject_blank("version", &version)?;
        reject_blank("extension", &extension)?;

        Ok(Self {
            product,
            version,
            target,
            channel,
            extension,
        })
    }

    #[must_use]
    pub fn product(&self) -> &str {
        &self.product
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    #[must_use]
    pub const fn target(&self) -> PlatformTarget {
        self.target
    }

    #[must_use]
    pub const fn channel(&self) -> ReleaseChannel {
        self.channel
    }

    #[must_use]
    pub fn extension(&self) -> &str {
        &self.extension
    }

    #[must_use]
    pub fn is_macos_app_bundle(&self) -> bool {
        self.extension == "app" && self.target.as_str().starts_with("macos-")
    }

    #[must_use]
    pub fn is_macos_dmg(&self) -> bool {
        self.extension == "dmg" && self.target.as_str().starts_with("macos-")
    }

    #[must_use]
    pub fn file_name(&self) -> String {
        format!(
            "{}-{}-{}-{}.{}",
            self.product,
            self.version,
            self.target.as_str(),
            self.channel.as_str(),
            self.extension
        )
    }

    #[must_use]
    pub fn generated_artifact_path(&self) -> GeneratedArtifactPath {
        GeneratedArtifactPath {
            value: format!("dist/desktoplab-packaging/{}", self.file_name()),
        }
    }

    #[must_use]
    pub fn manifest_path() -> GeneratedArtifactPath {
        GeneratedArtifactPath {
            value: "dist/desktoplab-packaging/artifacts.json".to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactSpecError {
    field: &'static str,
}

impl ArtifactSpecError {
    #[must_use]
    pub const fn field(&self) -> &'static str {
        self.field
    }
}

fn reject_blank(field: &'static str, value: &str) -> Result<(), ArtifactSpecError> {
    if value.trim().is_empty() {
        return Err(ArtifactSpecError { field });
    }

    Ok(())
}
