#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperatingSystem {
    MacOS,
    Windows,
    Linux,
    Unsupported(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeVaultKind {
    MacOsKeychain,
    WindowsCredentialManager,
    LinuxSecretService,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DegradedVaultReason {
    UnsupportedOperatingSystem(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VaultAdapterSelection {
    Available(NativeVaultKind),
    Degraded(DegradedVaultReason),
}

impl VaultAdapterSelection {
    #[must_use]
    pub fn current() -> Self {
        Self::for_os(OperatingSystem::current())
    }

    #[must_use]
    pub fn for_os(operating_system: OperatingSystem) -> Self {
        match operating_system {
            OperatingSystem::MacOS => Self::Available(NativeVaultKind::MacOsKeychain),
            OperatingSystem::Windows => Self::Available(NativeVaultKind::WindowsCredentialManager),
            OperatingSystem::Linux => Self::Available(NativeVaultKind::LinuxSecretService),
            OperatingSystem::Unsupported(name) => {
                Self::Degraded(DegradedVaultReason::UnsupportedOperatingSystem(name))
            }
        }
    }

    #[must_use]
    pub fn can_save_credentials(&self) -> bool {
        matches!(self, Self::Available(_))
    }

    #[must_use]
    pub fn allows_plaintext_fallback(&self) -> bool {
        false
    }
}

impl OperatingSystem {
    #[must_use]
    pub fn current() -> Self {
        Self::from_label(std::env::consts::OS)
    }

    #[must_use]
    pub fn from_label(label: &str) -> Self {
        match label {
            "macos" => Self::MacOS,
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            other => Self::Unsupported(other.to_string()),
        }
    }
}
