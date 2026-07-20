use crate::ModelVariant;

pub struct AgentContextWindowPolicy;

impl AgentContextWindowPolicy {
    #[must_use]
    pub fn for_variant(variant: &ModelVariant, available_memory_gb: u32) -> u32 {
        Self::from_capacity(
            variant.context_window_tokens(),
            variant.required_memory_gb(),
            available_memory_gb,
        )
    }

    #[must_use]
    pub fn from_capacity(
        model_max_tokens: u32,
        required_memory_gb: u32,
        available_memory_gb: u32,
    ) -> u32 {
        let memory_headroom_gb = available_memory_gb.saturating_sub(required_memory_gb);
        let hardware_budget = match memory_headroom_gb {
            0..=3 => 8_192,
            4..=7 => 16_384,
            8..=15 => 32_768,
            16..=31 => 65_536,
            32..=63 => 131_072,
            _ => model_max_tokens,
        };
        hardware_budget.min(model_max_tokens)
    }
}
