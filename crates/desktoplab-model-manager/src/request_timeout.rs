use crate::ModelVariant;

pub struct AgentRequestTimeoutPolicy;

impl AgentRequestTimeoutPolicy {
    #[must_use]
    pub fn for_variant(variant: &ModelVariant, available_memory_gb: u32) -> u64 {
        Self::from_capacity(variant.required_memory_gb(), available_memory_gb)
    }

    #[must_use]
    pub fn from_capacity(required_memory_gb: u32, available_memory_gb: u32) -> u64 {
        match available_memory_gb.saturating_sub(required_memory_gb) {
            0..=3 => 600,
            4..=15 => 300,
            16..=31 => 240,
            32..=63 => 180,
            _ => 120,
        }
    }
}
