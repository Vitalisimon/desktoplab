use crate::HardwareObservation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperatingSystem {
    Macos,
    Windows,
    Linux,
    Unsupported(String),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Architecture {
    Aarch64,
    X86_64,
    Unsupported(String),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcceleratorVendor {
    Apple,
    Amd,
    Nvidia,
    Intel,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AcceleratorKind {
    Integrated,
    Discrete,
    UnifiedMemory,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DriverState {
    DeferredToV2,
    Verified(String),
    Missing,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PerformanceClass {
    NotRecommended,
    Light,
    Standard,
    Strong,
    Workstation,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WarningCode {
    DriverProbeDeferredToV2,
    GpuProbeUnavailable,
    LimitedMemory,
    LowStorage,
    UnsupportedArchitecture,
    UnsupportedOperatingSystem,
    VramProbeUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareProfile {
    operating_system: HardwareObservation<OperatingSystem>,
    architecture: HardwareObservation<Architecture>,
    cpu: HardwareObservation<String>,
    ram_gb: HardwareObservation<u32>,
    gpu: HardwareObservation<String>,
    accelerator_vendor: HardwareObservation<AcceleratorVendor>,
    accelerator_kind: HardwareObservation<AcceleratorKind>,
    vram_gb: HardwareObservation<u32>,
    unified_memory_gb: HardwareObservation<u32>,
    driver_state: HardwareObservation<DriverState>,
    storage_available_gb: HardwareObservation<u32>,
    performance_class: PerformanceClass,
    warnings: Vec<WarningCode>,
}

impl HardwareProfile {
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) fn new(
        operating_system: HardwareObservation<OperatingSystem>,
        architecture: HardwareObservation<Architecture>,
        cpu: HardwareObservation<String>,
        ram_gb: HardwareObservation<u32>,
        gpu: HardwareObservation<String>,
        accelerator_vendor: HardwareObservation<AcceleratorVendor>,
        accelerator_kind: HardwareObservation<AcceleratorKind>,
        vram_gb: HardwareObservation<u32>,
        unified_memory_gb: HardwareObservation<u32>,
        driver_state: HardwareObservation<DriverState>,
        storage_available_gb: HardwareObservation<u32>,
        performance_class: PerformanceClass,
        warnings: Vec<WarningCode>,
    ) -> Self {
        Self {
            operating_system,
            architecture,
            cpu,
            ram_gb,
            gpu,
            accelerator_vendor,
            accelerator_kind,
            vram_gb,
            unified_memory_gb,
            driver_state,
            storage_available_gb,
            performance_class,
            warnings,
        }
    }

    #[must_use]
    pub fn operating_system(&self) -> &HardwareObservation<OperatingSystem> {
        &self.operating_system
    }

    #[must_use]
    pub fn architecture(&self) -> &HardwareObservation<Architecture> {
        &self.architecture
    }

    #[must_use]
    pub fn cpu(&self) -> &HardwareObservation<String> {
        &self.cpu
    }

    #[must_use]
    pub fn ram_gb(&self) -> &HardwareObservation<u32> {
        &self.ram_gb
    }

    #[must_use]
    pub fn gpu(&self) -> &HardwareObservation<String> {
        &self.gpu
    }

    #[must_use]
    pub fn accelerator_vendor(&self) -> &HardwareObservation<AcceleratorVendor> {
        &self.accelerator_vendor
    }

    #[must_use]
    pub fn accelerator_kind(&self) -> &HardwareObservation<AcceleratorKind> {
        &self.accelerator_kind
    }

    #[must_use]
    pub fn vram_gb(&self) -> &HardwareObservation<u32> {
        &self.vram_gb
    }

    #[must_use]
    pub fn unified_memory_gb(&self) -> &HardwareObservation<u32> {
        &self.unified_memory_gb
    }

    #[must_use]
    pub fn driver_state(&self) -> &HardwareObservation<DriverState> {
        &self.driver_state
    }

    #[must_use]
    pub fn storage_available_gb(&self) -> &HardwareObservation<u32> {
        &self.storage_available_gb
    }

    #[must_use]
    pub fn performance_class(&self) -> PerformanceClass {
        self.performance_class
    }

    #[must_use]
    pub fn warnings(&self) -> &[WarningCode] {
        &self.warnings
    }

    #[must_use]
    pub fn has_warning(&self, code: WarningCode) -> bool {
        self.warnings.contains(&code)
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.warnings.iter().any(|warning| {
            matches!(
                warning,
                WarningCode::GpuProbeUnavailable
                    | WarningCode::VramProbeUnavailable
                    | WarningCode::DriverProbeDeferredToV2
            )
        })
    }
}
