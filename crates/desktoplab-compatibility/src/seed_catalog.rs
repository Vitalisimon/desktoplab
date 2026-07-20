use crate::Channel;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelLicense {
    Known,
    Unknown,
    Restricted,
}

impl ModelLicense {
    #[must_use]
    pub fn is_unknown(self) -> bool {
        self == Self::Unknown
    }

    #[must_use]
    pub fn is_restricted(self) -> bool {
        self == Self::Restricted
    }

    #[must_use]
    pub fn is_recommendable(self) -> bool {
        self == Self::Known
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::Unknown => "unknown",
            Self::Restricted => "restricted",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductModelSeedEntry {
    family_id: String,
    family_name: String,
    model_id: String,
    parameter_class: String,
    parameters_billion: u32,
    quantization: String,
    context_window_tokens: u32,
    required_memory_gb: u32,
    expected_disk_mb: u64,
    runtime_id: String,
    pull_ref: String,
    channel: Channel,
    license: ModelLicense,
    capabilities: Vec<String>,
}

impl ProductModelSeedEntry {
    #[allow(clippy::too_many_arguments)]
    fn new(
        family_id: impl Into<String>,
        family_name: impl Into<String>,
        model_id: impl Into<String>,
        parameter_class: impl Into<String>,
        parameters_billion: u32,
        quantization: impl Into<String>,
        context_window_tokens: u32,
        required_memory_gb: u32,
        expected_disk_mb: u64,
        runtime_id: impl Into<String>,
        pull_ref: impl Into<String>,
        channel: Channel,
        license: ModelLicense,
        capabilities: &[&str],
    ) -> Self {
        Self {
            family_id: family_id.into(),
            family_name: family_name.into(),
            model_id: model_id.into(),
            parameter_class: parameter_class.into(),
            parameters_billion,
            quantization: quantization.into(),
            context_window_tokens,
            required_memory_gb,
            expected_disk_mb,
            runtime_id: runtime_id.into(),
            pull_ref: pull_ref.into(),
            channel,
            license,
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
        }
    }

    #[must_use]
    pub fn family_id(&self) -> &str {
        &self.family_id
    }

    #[must_use]
    pub fn family_name(&self) -> &str {
        &self.family_name
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn parameter_class(&self) -> &str {
        &self.parameter_class
    }

    #[must_use]
    pub fn parameters_billion(&self) -> u32 {
        self.parameters_billion
    }

    #[must_use]
    pub fn quantization(&self) -> &str {
        &self.quantization
    }

    #[must_use]
    pub fn context_window_tokens(&self) -> u32 {
        self.context_window_tokens
    }

    #[must_use]
    pub fn required_memory_gb(&self) -> u32 {
        self.required_memory_gb
    }

    #[must_use]
    pub fn expected_disk_mb(&self) -> u64 {
        self.expected_disk_mb
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn pull_ref(&self) -> &str {
        &self.pull_ref
    }

    #[must_use]
    pub fn channel(&self) -> Channel {
        self.channel
    }

    #[must_use]
    pub fn license(&self) -> ModelLicense {
        self.license
    }

    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }

    #[must_use]
    pub fn is_recommendable(&self) -> bool {
        self.license.is_recommendable()
    }

    #[must_use]
    pub fn is_downloadable_now(&self) -> bool {
        matches!(
            self.runtime_id.as_str(),
            "runtime.ollama" | "runtime.mlx-lm"
        ) && self.is_recommendable()
    }

    #[must_use]
    pub fn is_usable_now(&self) -> bool {
        self.runtime_id != "runtime.future" && self.is_recommendable()
    }

    #[must_use]
    pub fn requires_fresh_accelerator_evidence(&self) -> bool {
        self.required_memory_gb >= 96
    }

    #[must_use]
    pub fn eligible_high_end_hardware_classes(&self) -> &'static [&'static str] {
        match self.required_memory_gb {
            512.. => &["dgx_station_class", "custom_frontier_rig"],
            128.. => &[
                "dgx_spark_class",
                "dgx_station_class",
                "custom_frontier_rig",
            ],
            96.. => &[
                "workstation_local",
                "dgx_spark_class",
                "dgx_station_class",
                "custom_frontier_rig",
            ],
            _ => &[],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductModelSeedCatalog {
    entries: Vec<ProductModelSeedEntry>,
}

impl ProductModelSeedCatalog {
    #[must_use]
    pub fn initial_coding() -> Self {
        Self {
            entries: INITIAL_CODING_MODELS
                .iter()
                .map(ProductModelSeedEntry::from)
                .collect(),
        }
    }

    #[must_use]
    pub fn frontier_class_catalog() -> crate::FrontierModelClassCatalog {
        crate::FrontierModelClassCatalog::planned()
    }

    #[must_use]
    pub fn entries(&self) -> &[ProductModelSeedEntry] {
        &self.entries
    }

    #[must_use]
    pub fn model_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| entry.model_id.clone())
            .collect()
    }

    #[must_use]
    pub fn entry(&self, model_id: &str) -> Option<&ProductModelSeedEntry> {
        self.entries.iter().find(|entry| entry.model_id == model_id)
    }
}

struct SeedModelRow {
    family_id: &'static str,
    family_name: &'static str,
    model_id: &'static str,
    parameter_class: &'static str,
    parameters_billion: u32,
    quantization: &'static str,
    context_window_tokens: u32,
    required_memory_gb: u32,
    expected_disk_mb: u64,
    runtime_id: &'static str,
    pull_ref: &'static str,
    channel: Channel,
    capabilities: &'static [&'static str],
}

impl From<&SeedModelRow> for ProductModelSeedEntry {
    fn from(row: &SeedModelRow) -> Self {
        ProductModelSeedEntry::new(
            row.family_id,
            row.family_name,
            row.model_id,
            row.parameter_class,
            row.parameters_billion,
            row.quantization,
            row.context_window_tokens,
            row.required_memory_gb,
            row.expected_disk_mb,
            row.runtime_id,
            row.pull_ref,
            row.channel,
            ModelLicense::Known,
            row.capabilities,
        )
    }
}

#[rustfmt::skip]
const INITIAL_CODING_MODELS: &[SeedModelRow] = &[
    seed("family.nemotron-3-nano", "Nemotron 3 Nano", "model.nemotron-3-nano-4b-q4", "small", 4, "Q4", 256_000, 12, 2_800, "runtime.ollama", "nemotron-3-nano:4b", Channel::Experimental, AGENT_CANDIDATE),
    seed("family.qwen3.5", "Qwen 3.5", "model.qwen3.5-9b-q4", "small", 9, "Q4", 256_000, 16, 6_600, "runtime.ollama", "qwen3.5:9b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.gemma4", "Gemma 4", "model.gemma4-12b-q4", "small", 12, "Q4", 256_000, 16, 7_600, "runtime.ollama", "gemma4:12b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.gpt-oss", "GPT OSS", "model.gpt-oss-20b-mxfp4", "medium", 20, "MXFP4", 128_000, 24, 14_000, "runtime.ollama", "gpt-oss:20b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.qwen3-coder", "Qwen 3 Coder", "model.qwen3-coder-30b-q4", "medium", 30, "Q4", 256_000, 32, 19_000, "runtime.ollama", "qwen3-coder:30b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.qwen3.6", "Qwen 3.6", "model.qwen3.6-27b-q4", "medium", 27, "Q4", 256_000, 32, 17_000, "runtime.ollama", "qwen3.6:27b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.devstral-small-2", "Devstral Small 2", "model.devstral-small-2-24b-q4", "medium", 24, "Q4", 384_000, 32, 15_000, "runtime.ollama", "devstral-small-2:24b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.north-mini-code", "North Mini Code", "model.north-mini-code-30b-q4", "medium", 30, "Q4", 488_000, 32, 19_000, "runtime.ollama", "north-mini-code-1.0:q4_K_M", Channel::Experimental, AGENT_CANDIDATE),
    seed("family.gemma4", "Gemma 4", "model.gemma4-26b-q4", "medium", 26, "Q4", 256_000, 32, 18_000, "runtime.ollama", "gemma4:26b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.qwen3.6", "Qwen 3.6", "model.qwen3.6-35b-q4", "large", 35, "Q4", 256_000, 48, 24_000, "runtime.ollama", "qwen3.6:35b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.nemotron-3-nano", "Nemotron 3 Nano", "model.nemotron-3-nano-30b-q4", "large", 30, "Q4", 1_000_000, 48, 24_000, "runtime.ollama", "nemotron-3-nano:30b", Channel::Experimental, AGENT_CANDIDATE),
    seed("family.nemotron-cascade-2", "Nemotron Cascade 2", "model.nemotron-cascade-2-30b-q4", "large", 30, "Q4", 256_000, 48, 24_000, "runtime.ollama", "nemotron-cascade-2:30b", Channel::Experimental, AGENT_CANDIDATE),
    seed("family.gpt-oss", "GPT OSS", "model.gpt-oss-120b-mxfp4", "large", 120, "MXFP4", 128_000, 96, 65_000, "runtime.ollama", "gpt-oss:120b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.qwen3-coder-next", "Qwen 3 Coder Next", "model.qwen3-coder-next-80b-q4", "large", 80, "Q4", 256_000, 96, 52_000, "runtime.ollama", "qwen3-coder-next:q4_K_M", Channel::Beta, AGENT_CANDIDATE),
    seed("family.qwen3.5", "Qwen 3.5", "model.qwen3.5-122b-q4", "workstation", 122, "Q4", 256_000, 128, 81_000, "runtime.ollama", "qwen3.5:122b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.mistral-medium-3.5", "Mistral Medium 3.5", "model.mistral-medium-3.5-128b-q4", "workstation", 128, "Q4", 256_000, 128, 80_000, "runtime.ollama", "mistral-medium-3.5:128b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.devstral-2", "Devstral 2", "model.devstral-2-123b-q4", "workstation", 123, "Q4", 256_000, 128, 75_000, "runtime.ollama", "devstral-2:123b", Channel::Beta, AGENT_CANDIDATE),
    seed("family.nemotron-3-super", "Nemotron 3 Super", "model.nemotron-3-super-120b-q4", "workstation", 120, "Q4", 256_000, 128, 87_000, "runtime.ollama", "nemotron-3-super:120b", Channel::Experimental, AGENT_CANDIDATE),
    seed("family.qwen3-coder", "Qwen 3 Coder", "model.qwen3-coder-480b-q4", "workstation", 480, "Q4", 256_000, 512, 290_000, "runtime.ollama", "qwen3-coder:480b", Channel::Beta, AGENT_CANDIDATE),
];

const AGENT_CANDIDATE: &[&str] = &["coding", "tool_use", "agent_candidate", "official_ollama"];

#[allow(clippy::too_many_arguments)]
const fn seed(
    family_id: &'static str,
    family_name: &'static str,
    model_id: &'static str,
    parameter_class: &'static str,
    parameters_billion: u32,
    quantization: &'static str,
    context_window_tokens: u32,
    required_memory_gb: u32,
    expected_disk_mb: u64,
    runtime_id: &'static str,
    pull_ref: &'static str,
    channel: Channel,
    capabilities: &'static [&'static str],
) -> SeedModelRow {
    SeedModelRow {
        family_id,
        family_name,
        model_id,
        parameter_class,
        parameters_billion,
        quantization,
        context_window_tokens,
        required_memory_gb,
        expected_disk_mb,
        runtime_id,
        pull_ref,
        channel,
        capabilities,
    }
}
