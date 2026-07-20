#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallerSource {
    url: String,
    checksum: String,
    signature: Option<String>,
}

impl InstallerSource {
    #[must_use]
    pub fn signed_url(url: impl Into<String>, checksum: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            checksum: checksum.into(),
            signature: None,
        }
    }

    #[must_use]
    pub fn signed_url_with_signature(
        url: impl Into<String>,
        checksum: impl Into<String>,
        signature: impl Into<String>,
    ) -> Self {
        Self {
            url: url.into(),
            checksum: checksum.into(),
            signature: Some(signature.into()),
        }
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn checksum(&self) -> &str {
        &self.checksum
    }

    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }

    pub(crate) fn has_verification_metadata(&self) -> bool {
        !self.checksum.trim().is_empty() || self.signature.is_some()
    }

    pub(crate) fn is_trusted_remote(&self) -> bool {
        let url = self.url.trim();
        url.starts_with("https://") && !url.contains(char::is_whitespace)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInstallExecutionStrategy {
    NativeInstaller,
    ArchiveExtraction,
    ManagedAppBridge,
    PythonEnvironment,
}

impl RuntimeInstallExecutionStrategy {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NativeInstaller => "native_installer",
            Self::ArchiveExtraction => "archive_extraction",
            Self::ManagedAppBridge => "managed_app_bridge",
            Self::PythonEnvironment => "python_environment",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInstallManagement {
    DesktopLabManaged,
    ExternallyManaged,
}
