use std::collections::HashSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendCapabilitySet {
    backend_id: String,
    capabilities: HashSet<String>,
}

impl BackendCapabilitySet {
    #[must_use]
    pub fn new(backend_id: impl Into<String>) -> Self {
        Self {
            backend_id: backend_id.into(),
            capabilities: HashSet::new(),
        }
    }

    #[must_use]
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.insert(capability.into());
        self
    }

    #[must_use]
    pub fn satisfies(&self, required: &[&str]) -> bool {
        required
            .iter()
            .all(|capability| self.capabilities.contains(*capability))
    }
}
