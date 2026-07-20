#![forbid(unsafe_code)]

mod setup_to_first_prompt;
mod setup_to_first_prompt_outcome;

pub use setup_to_first_prompt::{SetupToFirstPromptHarness, SetupToFirstPromptMode};
pub use setup_to_first_prompt_outcome::SetupToFirstPromptOutcome;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductizationGate {
    LocalSetup,
    CloudProviderDryRun,
    LocalAgentEditTest,
    GitSavePointRollbackCommit,
    WorktreeParallelWrite,
    PluginTrust,
    DegradedOffline,
    SecurityDenial,
}

impl ProductizationGate {
    const ALL: [Self; 8] = [
        Self::LocalSetup,
        Self::CloudProviderDryRun,
        Self::LocalAgentEditTest,
        Self::GitSavePointRollbackCommit,
        Self::WorktreeParallelWrite,
        Self::PluginTrust,
        Self::DegradedOffline,
        Self::SecurityDenial,
    ];
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendProductizationGatePack {
    passed: Vec<ProductizationGate>,
}

impl BackendProductizationGatePack {
    #[must_use]
    pub fn contains(&self, gate: ProductizationGate) -> bool {
        ProductizationGate::ALL.contains(&gate)
    }

    pub fn mark_all_passed(&mut self) {
        self.passed = ProductizationGate::ALL.to_vec();
    }

    #[must_use]
    pub fn is_packaging_ready(&self) -> bool {
        ProductizationGate::ALL
            .iter()
            .all(|gate| self.passed.contains(gate))
    }

    #[must_use]
    pub fn failed_gates(&self) -> Vec<ProductizationGate> {
        ProductizationGate::ALL
            .iter()
            .copied()
            .filter(|gate| !self.passed.contains(gate))
            .collect()
    }
}
