use crate::{ModelManifest, RuntimeManifest};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockedCombination {
    runtime_id: String,
    model_id: String,
    reason: String,
}

impl BlockedCombination {
    #[must_use]
    pub fn matches(&self, runtime_id: &str, model_id: &str) -> bool {
        self.runtime_id == runtime_id && self.model_id == model_id
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityEvidence {
    id: String,
    score: u8,
}

impl CompatibilityEvidence {
    #[must_use]
    pub fn new(id: impl Into<String>, score: u8) -> Self {
        Self {
            id: id.into(),
            score,
        }
    }

    #[must_use]
    pub fn score(&self) -> u8 {
        self.score
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CompatibilityCatalog {
    runtimes: Vec<RuntimeManifest>,
    models: Vec<ModelManifest>,
    blocked: Vec<BlockedCombination>,
    evidence: Vec<CompatibilityEvidence>,
}

impl CompatibilityCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_runtime(mut self, runtime: RuntimeManifest) -> Self {
        self.runtimes.push(runtime);
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: ModelManifest) -> Self {
        self.models.push(model);
        self
    }

    #[must_use]
    pub fn with_blocked_combination(
        mut self,
        runtime_id: impl Into<String>,
        model_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.blocked.push(BlockedCombination {
            runtime_id: runtime_id.into(),
            model_id: model_id.into(),
            reason: reason.into(),
        });
        self
    }

    #[must_use]
    pub fn with_evidence(mut self, evidence: CompatibilityEvidence) -> Self {
        self.evidence.push(evidence);
        self
    }

    #[must_use]
    pub fn runtime(&self, runtime_id: &str) -> Option<&RuntimeManifest> {
        self.runtimes
            .iter()
            .find(|runtime| runtime.id() == runtime_id)
    }

    #[must_use]
    pub fn model(&self, model_id: &str) -> Option<&ModelManifest> {
        self.models.iter().find(|model| model.id() == model_id)
    }

    #[must_use]
    pub fn models(&self) -> &[ModelManifest] {
        &self.models
    }

    #[must_use]
    pub fn blocked_combination(
        &self,
        runtime_id: &str,
        model_id: &str,
    ) -> Option<&BlockedCombination> {
        self.blocked
            .iter()
            .find(|blocked| blocked.matches(runtime_id, model_id))
    }

    #[must_use]
    pub fn strongest_evidence_score(&self) -> Option<u8> {
        self.evidence.iter().map(CompatibilityEvidence::score).max()
    }
}
