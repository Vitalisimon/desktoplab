#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierParameterClass {
    B70,
    B100,
    B200,
    B300,
    B400,
    B600,
    T1,
}

impl FrontierParameterClass {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::B70 => "70B",
            Self::B100 => "100B",
            Self::B200 => "200B",
            Self::B300 => "300B",
            Self::B400 => "400B",
            Self::B600 => "600B",
            Self::T1 => "1T",
        }
    }

    #[must_use]
    pub fn parameters_billion(self) -> u16 {
        match self {
            Self::B70 => 70,
            Self::B100 => 100,
            Self::B200 => 200,
            Self::B300 => 300,
            Self::B400 => 400,
            Self::B600 => 600,
            Self::T1 => 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierCatalogClaimState {
    Available,
    Blocked,
    ResearchNeeded,
    NotPubliclyClaimed,
}

impl FrontierCatalogClaimState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Blocked => "blocked",
            Self::ResearchNeeded => "research-needed",
            Self::NotPubliclyClaimed => "not-publicly-claimed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommercialUseState {
    Allowed,
    Restricted,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelArtifactProvenance {
    source_url: String,
    checksum_sha256: String,
}

impl ModelArtifactProvenance {
    pub fn verified(
        source_url: impl Into<String>,
        checksum_sha256: impl Into<String>,
    ) -> Option<Self> {
        let source_url = source_url.into();
        let checksum_sha256 = checksum_sha256.into();
        let source_is_supported = source_url.starts_with("https://");
        let checksum_is_sha256 = checksum_sha256.len() == 64
            && checksum_sha256
                .chars()
                .all(|character| character.is_ascii_hexdigit());
        (source_is_supported && checksum_is_sha256).then_some(Self {
            source_url,
            checksum_sha256,
        })
    }

    #[must_use]
    pub fn source_url(&self) -> &str {
        &self.source_url
    }

    #[must_use]
    pub fn checksum_sha256(&self) -> &str {
        &self.checksum_sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierModelClassEntry {
    catalog_key: String,
    parameter_class: FrontierParameterClass,
    quantization_formats: Vec<String>,
    precision_formats: Vec<String>,
    context_window_tokens: u32,
    estimated_memory_gb: u32,
    estimated_disk_gb: u32,
    runtime_ids: Vec<String>,
    runtime_adapter_evidence: bool,
    license_id: Option<String>,
    commercial_use: CommercialUseState,
    provenance: Option<ModelArtifactProvenance>,
    claim_state: FrontierCatalogClaimState,
}

impl FrontierModelClassEntry {
    #[must_use]
    pub fn planned(
        parameter_class: FrontierParameterClass,
        estimated_memory_gb: u32,
        estimated_disk_gb: u32,
    ) -> Self {
        Self {
            catalog_key: format!(
                "frontier.class.{}",
                parameter_class.as_str().to_ascii_lowercase()
            ),
            parameter_class,
            quantization_formats: vec!["int4".into(), "fp8".into()],
            precision_formats: vec!["bf16".into(), "fp16".into()],
            context_window_tokens: 0,
            estimated_memory_gb,
            estimated_disk_gb,
            runtime_ids: vec![
                "runtime.nim".into(),
                "runtime.tensorrt-llm".into(),
                "runtime.vllm".into(),
                "runtime.llama-cpp-server".into(),
            ],
            runtime_adapter_evidence: false,
            license_id: None,
            commercial_use: CommercialUseState::Unknown,
            provenance: None,
            claim_state: FrontierCatalogClaimState::ResearchNeeded,
        }
    }

    #[must_use]
    pub fn with_context_window(mut self, context_window_tokens: u32) -> Self {
        self.context_window_tokens = context_window_tokens;
        self
    }

    #[must_use]
    pub fn with_distribution_evidence(
        mut self,
        license_id: impl Into<String>,
        commercial_use: CommercialUseState,
        provenance: ModelArtifactProvenance,
    ) -> Self {
        self.license_id = Some(license_id.into());
        self.commercial_use = commercial_use;
        self.provenance = Some(provenance);
        self
    }

    #[must_use]
    pub fn with_runtime_adapter_evidence(mut self) -> Self {
        self.runtime_adapter_evidence = true;
        self
    }

    #[must_use]
    pub fn with_claim_state(mut self, claim_state: FrontierCatalogClaimState) -> Self {
        self.claim_state = claim_state;
        self
    }

    #[must_use]
    pub fn catalog_key(&self) -> &str {
        &self.catalog_key
    }

    #[must_use]
    pub fn parameter_class(&self) -> FrontierParameterClass {
        self.parameter_class
    }

    #[must_use]
    pub fn quantization_formats(&self) -> &[String] {
        &self.quantization_formats
    }

    #[must_use]
    pub fn precision_formats(&self) -> &[String] {
        &self.precision_formats
    }

    #[must_use]
    pub fn context_window_tokens(&self) -> u32 {
        self.context_window_tokens
    }

    #[must_use]
    pub fn estimated_memory_gb(&self) -> u32 {
        self.estimated_memory_gb
    }

    #[must_use]
    pub fn estimated_disk_gb(&self) -> u32 {
        self.estimated_disk_gb
    }

    #[must_use]
    pub fn runtime_ids(&self) -> &[String] {
        &self.runtime_ids
    }

    #[must_use]
    pub fn license_id(&self) -> Option<&str> {
        self.license_id.as_deref()
    }

    #[must_use]
    pub fn commercial_use(&self) -> CommercialUseState {
        self.commercial_use
    }

    #[must_use]
    pub fn provenance(&self) -> Option<&ModelArtifactProvenance> {
        self.provenance.as_ref()
    }

    #[must_use]
    pub fn claim_state(&self) -> FrontierCatalogClaimState {
        self.claim_state
    }

    #[must_use]
    pub fn is_selectable(&self) -> bool {
        self.claim_state == FrontierCatalogClaimState::Available
            && self.runtime_adapter_evidence
            && self.license_id.is_some()
            && self.commercial_use == CommercialUseState::Allowed
            && self.provenance.is_some()
            && self.context_window_tokens > 0
    }

    #[must_use]
    pub fn blocked_reasons(&self) -> Vec<&'static str> {
        let mut reasons = Vec::new();
        if !self.runtime_adapter_evidence {
            reasons.push("runtime_adapter_evidence_missing");
        }
        if self.license_id.is_none() || self.commercial_use == CommercialUseState::Unknown {
            reasons.push("license_or_commercial_use_unknown");
        }
        if self.commercial_use == CommercialUseState::Restricted {
            reasons.push("commercial_use_restricted");
        }
        if self.provenance.is_none() {
            reasons.push("source_or_checksum_missing");
        }
        if self.context_window_tokens == 0 {
            reasons.push("context_window_unknown");
        }
        if self.claim_state != FrontierCatalogClaimState::Available {
            reasons.push("catalog_claim_not_available");
        }
        reasons
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrontierModelClassCatalog {
    entries: Vec<FrontierModelClassEntry>,
}

impl FrontierModelClassCatalog {
    #[must_use]
    pub fn planned() -> Self {
        let rows = [
            (FrontierParameterClass::B70, 96, 50),
            (FrontierParameterClass::B100, 144, 75),
            (FrontierParameterClass::B200, 288, 150),
            (FrontierParameterClass::B300, 416, 225),
            (FrontierParameterClass::B400, 544, 300),
            (FrontierParameterClass::B600, 800, 450),
            (FrontierParameterClass::T1, 1_280, 750),
        ];
        Self {
            entries: rows
                .into_iter()
                .map(|(class, memory, disk)| FrontierModelClassEntry::planned(class, memory, disk))
                .collect(),
        }
    }

    #[must_use]
    pub fn entries(&self) -> &[FrontierModelClassEntry] {
        &self.entries
    }
}
