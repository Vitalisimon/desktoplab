use crate::{HardwareProfile, PerformanceClass, WarningCode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareRecommendationInputs {
    performance_class: PerformanceClass,
    warnings: Vec<WarningCode>,
    expected_limitations: Vec<String>,
    setup_inputs: Vec<String>,
}

impl HardwareRecommendationInputs {
    #[must_use]
    pub(crate) fn from_profile(profile: &HardwareProfile) -> Self {
        let mut expected_limitations = Vec::new();
        if profile.performance_class() == PerformanceClass::Light {
            expected_limitations
                .push("small local models or cloud/external backends are more realistic".into());
        }
        Self {
            performance_class: profile.performance_class(),
            warnings: profile.warnings().to_vec(),
            expected_limitations,
            setup_inputs: vec![
                "hardware.operating_system".into(),
                "hardware.architecture".into(),
                "memory.ram_gb".into(),
                "memory.vram_gb".into(),
                "memory.unified_memory_gb".into(),
                "storage.available_gb".into(),
            ],
        }
    }

    #[must_use]
    pub fn performance_class(&self) -> PerformanceClass {
        self.performance_class
    }

    #[must_use]
    pub fn performance_label(&self) -> &'static str {
        match self.performance_class {
            PerformanceClass::Workstation => "Local workstation",
            PerformanceClass::Strong => "Strong local machine",
            PerformanceClass::Standard => "Standard local machine",
            PerformanceClass::Light => "Light local machine",
            PerformanceClass::NotRecommended => "Cloud or external backend recommended",
            PerformanceClass::Unknown => "Hardware class needs confirmation",
        }
    }

    #[must_use]
    pub fn has_warning(&self, warning: WarningCode) -> bool {
        self.warnings.contains(&warning)
    }

    #[must_use]
    pub fn expected_limitations(&self) -> &[String] {
        &self.expected_limitations
    }

    #[must_use]
    pub fn setup_inputs(&self) -> &[String] {
        &self.setup_inputs
    }
}
