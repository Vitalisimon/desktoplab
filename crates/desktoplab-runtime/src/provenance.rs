#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProvenance {
    runtime_id: String,
    version: Option<String>,
    install_source: String,
    verification_method: String,
    integrity: RuntimeIntegrityEvidence,
}

impl RuntimeProvenance {
    #[must_use]
    pub fn for_runtime(runtime_id: &str, version: Option<&str>) -> Self {
        match runtime_id {
            "runtime.lm-studio" => Self::external(
                runtime_id,
                version,
                "external_app",
                "OpenAI-compatible endpoint health check",
            ),
            "runtime.mlx-lm" => Self::desktoplab_managed(
                runtime_id,
                version,
                "python_environment",
                "MLX-LM process and local endpoint verification",
            ),
            _ => Self::desktoplab_managed(
                runtime_id,
                version,
                "signed_runtime_plan_or_existing_host_install",
                "binary detection plus local API health check",
            ),
        }
    }

    fn desktoplab_managed(
        runtime_id: &str,
        version: Option<&str>,
        install_source: &str,
        verification_method: &str,
    ) -> Self {
        Self::new(runtime_id, version, install_source, verification_method)
    }

    fn external(
        runtime_id: &str,
        version: Option<&str>,
        install_source: &str,
        verification_method: &str,
    ) -> Self {
        Self::new(runtime_id, version, install_source, verification_method)
    }

    fn new(
        runtime_id: &str,
        version: Option<&str>,
        install_source: &str,
        verification_method: &str,
    ) -> Self {
        Self {
            runtime_id: runtime_id.to_string(),
            version: version.map(ToString::to_string),
            install_source: install_source.to_string(),
            verification_method: verification_method.to_string(),
            integrity: RuntimeIntegrityEvidence::unavailable(
                "Runtime hash is unavailable for existing host installs.",
            ),
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn install_source(&self) -> &str {
        &self.install_source
    }

    #[must_use]
    pub fn verification_method(&self) -> &str {
        &self.verification_method
    }

    #[must_use]
    pub fn integrity(&self) -> &RuntimeIntegrityEvidence {
        &self.integrity
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeIntegrityEvidence {
    state: RuntimeIntegrityState,
    reason: String,
}

impl RuntimeIntegrityEvidence {
    #[must_use]
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            state: RuntimeIntegrityState::Unavailable,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn state(&self) -> RuntimeIntegrityState {
        self.state
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeIntegrityState {
    Unavailable,
}
