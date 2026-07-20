use crate::PlatformTarget;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmokeState {
    Passed,
    Failed,
    NotRun,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagingSmokeResult {
    platform: PlatformTarget,
    artifact: String,
    install_state: SmokeState,
    launch_state: SmokeState,
    local_api_state: SmokeState,
    cleanup_state: SmokeState,
}

impl PackagingSmokeResult {
    #[must_use]
    pub fn new(
        platform: PlatformTarget,
        artifact: impl Into<String>,
        install_state: SmokeState,
        launch_state: SmokeState,
        local_api_state: SmokeState,
        cleanup_state: SmokeState,
    ) -> Self {
        Self {
            platform,
            artifact: artifact.into(),
            install_state,
            launch_state,
            local_api_state,
            cleanup_state,
        }
    }

    #[must_use]
    pub fn unsupported(platform: PlatformTarget, artifact: impl Into<String>) -> Self {
        Self::new(
            platform,
            artifact,
            SmokeState::NotRun,
            SmokeState::NotRun,
            SmokeState::NotRun,
            SmokeState::NotRun,
        )
    }

    #[must_use]
    pub const fn platform(&self) -> PlatformTarget {
        self.platform
    }

    #[must_use]
    pub fn artifact(&self) -> &str {
        &self.artifact
    }

    #[must_use]
    pub const fn install_state(&self) -> SmokeState {
        self.install_state
    }

    #[must_use]
    pub const fn launch_state(&self) -> SmokeState {
        self.launch_state
    }

    #[must_use]
    pub const fn local_api_state(&self) -> SmokeState {
        self.local_api_state
    }

    #[must_use]
    pub const fn cleanup_state(&self) -> SmokeState {
        self.cleanup_state
    }

    #[must_use]
    pub fn to_json_line(&self) -> String {
        format!(
            "{{\"platform\":\"{}\",\"artifact\":\"{}\",\"installState\":\"{}\",\"launchState\":\"{}\",\"localApiState\":\"{}\",\"cleanupState\":\"{}\"}}",
            self.platform.as_str(),
            escape_json(&self.artifact),
            self.install_state.as_str(),
            self.launch_state.as_str(),
            self.local_api_state.as_str(),
            self.cleanup_state.as_str()
        )
    }
}

impl SmokeState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::NotRun => "not_run",
        }
    }
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
