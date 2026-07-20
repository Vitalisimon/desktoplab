use crate::{AgentProfileId, ModelProfileId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelProfile {
    id: ModelProfileId,
    name: String,
}

impl ModelProfile {
    #[must_use]
    pub fn new(id: ModelProfileId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &ModelProfileId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareProfile {
    operating_system: String,
    architecture: String,
}

impl HardwareProfile {
    #[must_use]
    pub fn new(operating_system: impl Into<String>, architecture: impl Into<String>) -> Self {
        Self {
            operating_system: operating_system.into(),
            architecture: architecture.into(),
        }
    }

    #[must_use]
    pub fn operating_system(&self) -> &str {
        &self.operating_system
    }

    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentProfile {
    id: AgentProfileId,
    name: String,
}

impl AgentProfile {
    #[must_use]
    pub fn new(id: AgentProfileId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }

    #[must_use]
    pub fn id(&self) -> &AgentProfileId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
