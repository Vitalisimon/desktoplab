#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientKind {
    DesktopShell,
    DeveloperCli,
    Diagnostics,
    FutureCompanion,
}

impl ClientKind {
    #[must_use]
    pub fn all_supported() -> &'static [Self] {
        &[
            Self::DesktopShell,
            Self::DeveloperCli,
            Self::Diagnostics,
            Self::FutureCompanion,
        ]
    }
}
