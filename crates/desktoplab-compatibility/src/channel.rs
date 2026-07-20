#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Channel {
    Stable,
    Beta,
    Experimental,
}

impl Channel {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stable => "stable",
            Self::Beta => "beta",
            Self::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelPolicy {
    StableOnly,
    StableAndBeta,
    AllowExperimental,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocalOverride {
    allow_experimental: bool,
}

impl LocalOverride {
    #[must_use]
    pub fn none() -> Self {
        Self {
            allow_experimental: false,
        }
    }

    #[must_use]
    pub fn allow_experimental() -> Self {
        Self {
            allow_experimental: true,
        }
    }

    #[must_use]
    pub fn allows_experimental(&self) -> bool {
        self.allow_experimental
    }
}
