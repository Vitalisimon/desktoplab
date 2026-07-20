#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReleaseChannel {
    Dev,
    Beta,
    Stable,
}

impl ReleaseChannel {
    pub const fn supported() -> &'static [Self] {
        &[Self::Dev, Self::Beta, Self::Stable]
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Beta => "beta",
            Self::Stable => "stable",
        }
    }

    #[must_use]
    pub const fn allows_unsigned_artifacts(self) -> bool {
        matches!(self, Self::Dev)
    }

    #[must_use]
    pub const fn requires_signed_artifact(self) -> bool {
        !self.allows_unsigned_artifacts()
    }

    #[must_use]
    pub const fn promotion_rank(self) -> u8 {
        match self {
            Self::Dev => 0,
            Self::Beta => 1,
            Self::Stable => 2,
        }
    }

    #[must_use]
    pub const fn can_promote_to(self, target: Self) -> bool {
        self.promotion_rank() < target.promotion_rank()
    }

    #[must_use]
    pub fn accepts_signature_state(self, signature_state: &str) -> bool {
        if self.allows_unsigned_artifacts() {
            return matches!(signature_state, "unsigned_dev" | "signed" | "notarized");
        }

        matches!(signature_state, "signed" | "notarized")
    }
}
