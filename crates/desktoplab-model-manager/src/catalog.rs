#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelParameterClass {
    Cloud,
    Small,
    Medium,
    Large,
    Workstation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelLicenseState {
    Known,
    Unknown,
    Restricted,
}

impl ModelLicenseState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::Unknown => "unknown",
            Self::Restricted => "restricted",
        }
    }

    #[must_use]
    pub fn trust_label(self) -> &'static str {
        match self {
            Self::Known => "License verified",
            Self::Unknown => "License needs review",
            Self::Restricted => "Restricted terms",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRuntimeCompatibility {
    runtime_id: String,
    pull_ref: String,
}

impl ModelRuntimeCompatibility {
    #[must_use]
    pub fn new(runtime_id: impl Into<String>, pull_ref: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            pull_ref: pull_ref.into(),
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn pull_ref(&self) -> &str {
        &self.pull_ref
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelVariant {
    family_id: String,
    family_name: String,
    model_id: String,
    parameter_class: ModelParameterClass,
    parameters_billion: u32,
    quantization: String,
    context_window_tokens: u32,
    required_memory_gb: u32,
    expected_disk_mb: u64,
    runtime_compatibility: ModelRuntimeCompatibility,
    channel: String,
    license_state: ModelLicenseState,
    capabilities: Vec<String>,
}

impl ModelVariant {
    #[must_use]
    pub fn new(
        family_id: impl Into<String>,
        family_name: impl Into<String>,
        model_id: impl Into<String>,
        parameter_class: ModelParameterClass,
        expected_disk_mb: u64,
        runtime_id: impl Into<String>,
        pull_ref: impl Into<String>,
        channel: impl Into<String>,
    ) -> Self {
        Self {
            family_id: family_id.into(),
            family_name: family_name.into(),
            model_id: model_id.into(),
            parameter_class,
            parameters_billion: 0,
            quantization: "unknown".to_string(),
            context_window_tokens: 0,
            required_memory_gb: 0,
            expected_disk_mb,
            runtime_compatibility: ModelRuntimeCompatibility::new(runtime_id, pull_ref),
            channel: channel.into(),
            license_state: ModelLicenseState::Unknown,
            capabilities: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_model_metadata(
        mut self,
        parameters_billion: u32,
        quantization: impl Into<String>,
        context_window_tokens: u32,
    ) -> Self {
        self.parameters_billion = parameters_billion;
        self.quantization = quantization.into();
        self.context_window_tokens = context_window_tokens;
        self
    }

    #[must_use]
    pub fn with_required_memory_gb(mut self, required_memory_gb: u32) -> Self {
        self.required_memory_gb = required_memory_gb;
        self
    }

    #[must_use]
    pub fn with_license_state(mut self, license_state: ModelLicenseState) -> Self {
        self.license_state = license_state;
        self
    }

    #[must_use]
    pub fn with_capabilities(mut self, capabilities: &[String]) -> Self {
        self.capabilities = capabilities.to_vec();
        self
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
    pub fn parameter_class(&self) -> ModelParameterClass {
        self.parameter_class
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
    pub fn runtime_compatibility(&self) -> ModelRuntimeCompatibility {
        self.runtime_compatibility.clone()
    }

    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }

    #[must_use]
    pub fn license_state(&self) -> ModelLicenseState {
        self.license_state
    }

    #[must_use]
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelFamilyCatalog {
    variants: Vec<ModelVariant>,
}

impl ModelFamilyCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_variant(mut self, variant: ModelVariant) -> Self {
        self.variants.push(variant);
        self
    }

    #[must_use]
    pub fn variants(&self) -> &[ModelVariant] {
        &self.variants
    }

    #[must_use]
    pub fn variants_for_family(&self, family_id: &str) -> Vec<&ModelVariant> {
        self.variants()
            .iter()
            .filter(|variant| variant.family_id() == family_id)
            .collect()
    }

    #[must_use]
    pub fn families(&self) -> Vec<(&str, &str)> {
        let mut families = Vec::new();
        for variant in &self.variants {
            if !families
                .iter()
                .any(|(family_id, _)| *family_id == variant.family_id())
            {
                families.push((variant.family_id(), variant.family_name()));
            }
        }
        families
    }
}
