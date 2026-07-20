#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcceleratorConfidence {
    Confirmed,
    Probable,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceleratorProfile {
    vendor: String,
    kind: String,
    vram_gb: u32,
    unified_memory_gb: u32,
    driver_state: String,
    confidence: AcceleratorConfidence,
}

impl AcceleratorProfile {
    #[must_use]
    pub fn new(
        vendor: impl Into<String>,
        kind: impl Into<String>,
        vram_gb: u32,
        unified_memory_gb: u32,
        driver_state: impl Into<String>,
    ) -> Self {
        Self {
            vendor: vendor.into(),
            kind: kind.into(),
            vram_gb,
            unified_memory_gb,
            driver_state: driver_state.into(),
            confidence: AcceleratorConfidence::Probable,
        }
    }

    #[must_use]
    pub fn unknown() -> Self {
        Self {
            vendor: "unknown".into(),
            kind: "unknown".into(),
            vram_gb: 0,
            unified_memory_gb: 0,
            driver_state: "unknown".into(),
            confidence: AcceleratorConfidence::Unknown,
        }
    }

    #[must_use]
    pub fn with_confidence(mut self, confidence: AcceleratorConfidence) -> Self {
        self.confidence = confidence;
        self
    }

    #[must_use]
    pub fn confidence(&self) -> AcceleratorConfidence {
        self.confidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareProfile {
    operating_system: String,
    architecture: String,
    memory_gb: u32,
    accelerator: AcceleratorProfile,
}

impl HardwareProfile {
    #[must_use]
    pub fn new(
        operating_system: impl Into<String>,
        architecture: impl Into<String>,
        memory_gb: u32,
    ) -> Self {
        Self {
            operating_system: operating_system.into(),
            architecture: architecture.into(),
            memory_gb,
            accelerator: AcceleratorProfile::unknown(),
        }
    }

    #[must_use]
    pub fn with_accelerator(mut self, accelerator: AcceleratorProfile) -> Self {
        self.accelerator = accelerator;
        self
    }

    #[must_use]
    pub fn memory_gb(&self) -> u32 {
        self.memory_gb
    }

    #[must_use]
    pub fn accelerator(&self) -> &AcceleratorProfile {
        &self.accelerator
    }
}
