#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformTarget {
    MacosUniversal,
    MacosAarch64,
    MacosX64,
    WindowsX64,
    LinuxX64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxPackageKind {
    AppImage,
    Deb,
    Rpm,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallerUserDataPolicy {
    Install,
    Upgrade,
    Uninstall,
    FullRemoval,
}

impl PlatformTarget {
    pub const fn supported() -> &'static [Self] {
        &[
            Self::MacosUniversal,
            Self::MacosAarch64,
            Self::MacosX64,
            Self::WindowsX64,
            Self::LinuxX64,
        ]
    }

    pub const fn macos_targets() -> &'static [Self] {
        &[Self::MacosAarch64, Self::MacosX64, Self::MacosUniversal]
    }

    pub const fn windows_targets() -> &'static [Self] {
        &[Self::WindowsX64]
    }

    pub const fn linux_targets() -> &'static [Self] {
        &[Self::LinuxX64]
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MacosUniversal => "macos-universal",
            Self::MacosAarch64 => "macos-aarch64",
            Self::MacosX64 => "macos-x64",
            Self::WindowsX64 => "windows-x64",
            Self::LinuxX64 => "linux-x64",
        }
    }
}

impl InstallerUserDataPolicy {
    #[must_use]
    pub const fn preserves_user_repositories(self) -> bool {
        !matches!(self, Self::FullRemoval)
    }

    #[must_use]
    pub const fn preserves_vault_references(self) -> bool {
        matches!(self, Self::Upgrade | Self::Uninstall)
    }

    #[must_use]
    pub const fn requires_explicit_user_action(self) -> bool {
        matches!(self, Self::FullRemoval)
    }
}

impl LinuxPackageKind {
    pub const fn supported() -> &'static [Self] {
        &[Self::AppImage, Self::Deb, Self::Rpm]
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AppImage => "AppImage",
            Self::Deb => "deb",
            Self::Rpm => "rpm",
        }
    }
}
