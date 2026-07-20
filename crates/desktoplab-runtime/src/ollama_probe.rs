#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeProbe {
    pub(crate) binary_path: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) models: Vec<String>,
}

impl RuntimeProbe {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_binary_path(mut self, binary_path: impl Into<String>) -> Self {
        self.binary_path = Some(binary_path.into());
        self
    }

    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.models.push(model.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDetection {
    pub(crate) installed: bool,
    pub(crate) version: Option<String>,
    pub(crate) models: Vec<String>,
}

impl RuntimeDetection {
    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.installed
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn models(&self) -> &[String] {
        &self.models
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeHealth {
    healthy: bool,
    reason: Option<String>,
}

impl RuntimeHealth {
    #[must_use]
    pub fn healthy() -> Self {
        Self {
            healthy: true,
            reason: None,
        }
    }

    #[must_use]
    pub fn unhealthy(reason: impl Into<String>) -> Self {
        Self {
            healthy: false,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn is_healthy(&self) -> bool {
        self.healthy
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaBinaryVerification {
    pub(crate) passed: bool,
    pub(crate) reason: Option<String>,
}

impl OllamaBinaryVerification {
    #[must_use]
    pub fn passed() -> Self {
        Self {
            passed: true,
            reason: None,
        }
    }

    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            reason: Some(reason.into()),
        }
    }
}
