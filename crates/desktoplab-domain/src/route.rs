use crate::ExecutionBackendId;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BackendCapability(String);

impl BackendCapability {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionRoute {
    backend_id: ExecutionBackendId,
    required_capabilities: Vec<BackendCapability>,
}

impl ExecutionRoute {
    #[must_use]
    pub fn new(
        backend_id: ExecutionBackendId,
        required_capabilities: Vec<BackendCapability>,
    ) -> Self {
        Self {
            backend_id,
            required_capabilities,
        }
    }

    #[must_use]
    pub fn backend_id(&self) -> &ExecutionBackendId {
        &self.backend_id
    }

    #[must_use]
    pub fn required_capabilities(&self) -> &[BackendCapability] {
        &self.required_capabilities
    }
}
