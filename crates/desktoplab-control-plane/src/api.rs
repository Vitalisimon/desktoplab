#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionInfo {
    product_version: String,
    api_version: String,
}

impl VersionInfo {
    #[must_use]
    pub fn new(product_version: impl Into<String>, api_version: impl Into<String>) -> Self {
        Self {
            product_version: product_version.into(),
            api_version: api_version.into(),
        }
    }

    #[must_use]
    pub fn product_version(&self) -> &str {
        &self.product_version
    }

    #[must_use]
    pub fn api_version(&self) -> &str {
        &self.api_version
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiSurface {
    base_path: &'static str,
    health_path: &'static str,
    readiness_path: &'static str,
    version_path: &'static str,
}

impl ApiSurface {
    #[must_use]
    pub fn v1() -> Self {
        Self {
            base_path: "/v1",
            health_path: "/health",
            readiness_path: "/v1/readiness",
            version_path: "/v1/version",
        }
    }

    #[must_use]
    pub fn base_path(&self) -> &str {
        self.base_path
    }

    #[must_use]
    pub fn health_path(&self) -> &str {
        self.health_path
    }

    #[must_use]
    pub fn readiness_path(&self) -> &str {
        self.readiness_path
    }

    #[must_use]
    pub fn version_path(&self) -> &str {
        self.version_path
    }
}
