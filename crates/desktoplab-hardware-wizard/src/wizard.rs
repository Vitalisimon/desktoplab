use crate::{
    AcceleratorKind, AcceleratorVendor, Architecture, Confidence, DriverState, HardwareObservation,
    HardwareProfile, HardwareRecommendationInputs, OperatingSystem, PerformanceClass,
    ProbeSnapshot, WarningCode,
};

#[derive(Clone, Debug, Default)]
pub struct HardwareWizard;

impl HardwareWizard {
    #[must_use]
    pub fn v1() -> Self {
        Self
    }

    #[must_use]
    pub fn profile(&self, snapshot: ProbeSnapshot) -> HardwareProfile {
        let operating_system = normalize_operating_system(snapshot.operating_system());
        let architecture = normalize_architecture(snapshot.architecture());
        let cpu = snapshot.cpu().map_or_else(
            || HardwareObservation::unknown(String::new()),
            confirmed_string,
        );
        let gpu = snapshot.gpu().map_or_else(
            || HardwareObservation::unknown(String::new()),
            confirmed_string,
        );
        let ram_gb = snapshot.ram_gb().clone();
        let vram_gb = snapshot.vram_gb().clone();
        let unified_memory_gb = snapshot.unified_memory_gb().clone();
        let accelerator_vendor = accelerator_vendor(&gpu);
        let accelerator_kind =
            accelerator_kind(&gpu, &accelerator_vendor, &vram_gb, &unified_memory_gb);
        let driver_state = HardwareObservation::unknown(DriverState::DeferredToV2);
        let storage_available_gb = snapshot.storage_available_gb().clone();
        let mut warnings = warnings_for_profile(
            &operating_system,
            &architecture,
            &ram_gb,
            &gpu,
            &accelerator_kind,
            &vram_gb,
            &unified_memory_gb,
            &storage_available_gb,
        );
        warnings.sort_by_key(|warning| *warning as u8);
        warnings.dedup();
        let performance_class = performance_class(
            &operating_system,
            &architecture,
            &ram_gb,
            &vram_gb,
            &unified_memory_gb,
        );

        HardwareProfile::new(
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
        )
    }

    #[must_use]
    pub fn recommendation_inputs(&self, profile: &HardwareProfile) -> HardwareRecommendationInputs {
        HardwareRecommendationInputs::from_profile(profile)
    }
}

fn confirmed_string(value: &str) -> HardwareObservation<String> {
    HardwareObservation::confirmed(value.to_string())
}

fn normalize_operating_system(raw: Option<&str>) -> HardwareObservation<OperatingSystem> {
    let Some(raw) = raw else {
        return HardwareObservation::unknown(OperatingSystem::Unknown);
    };
    let normalized = raw.to_ascii_lowercase();
    if normalized.contains("macos") || normalized.contains("darwin") {
        return HardwareObservation::confirmed(OperatingSystem::Macos);
    }
    if normalized.contains("windows") {
        return HardwareObservation::confirmed(OperatingSystem::Windows);
    }
    if normalized.contains("linux") || normalized.contains("ubuntu") {
        return HardwareObservation::confirmed(OperatingSystem::Linux);
    }
    HardwareObservation::unsupported(OperatingSystem::Unsupported(raw.to_string()))
}

fn normalize_architecture(raw: Option<&str>) -> HardwareObservation<Architecture> {
    let Some(raw) = raw else {
        return HardwareObservation::unknown(Architecture::Unknown);
    };
    match raw.to_ascii_lowercase().as_str() {
        "aarch64" | "arm64" => HardwareObservation::confirmed(Architecture::Aarch64),
        "amd64" | "x64" | "x86_64" => HardwareObservation::confirmed(Architecture::X86_64),
        _ => HardwareObservation::unsupported(Architecture::Unsupported(raw.to_string())),
    }
}

fn accelerator_vendor(gpu: &HardwareObservation<String>) -> HardwareObservation<AcceleratorVendor> {
    if gpu.confidence() == Confidence::Unknown {
        return HardwareObservation::unknown(AcceleratorVendor::Unknown);
    }
    let normalized = gpu.value().to_ascii_lowercase();
    if normalized.contains("nvidia") {
        return HardwareObservation::confirmed(AcceleratorVendor::Nvidia);
    }
    if normalized.contains("amd") || normalized.contains("radeon") {
        return HardwareObservation::confirmed(AcceleratorVendor::Amd);
    }
    if normalized.contains("apple") || normalized.contains("m1") || normalized.contains("m2") {
        return HardwareObservation::confirmed(AcceleratorVendor::Apple);
    }
    if normalized.contains("intel") {
        return HardwareObservation::confirmed(AcceleratorVendor::Intel);
    }
    HardwareObservation::unknown(AcceleratorVendor::Unknown)
}

fn accelerator_kind(
    gpu: &HardwareObservation<String>,
    vendor: &HardwareObservation<AcceleratorVendor>,
    vram_gb: &HardwareObservation<u32>,
    unified_memory_gb: &HardwareObservation<u32>,
) -> HardwareObservation<AcceleratorKind> {
    if vendor.confidence() == Confidence::Unknown {
        return HardwareObservation::unknown(AcceleratorKind::Unknown);
    }
    if vendor.value() == AcceleratorVendor::Apple
        || unified_memory_gb.confidence() == Confidence::Confirmed
    {
        return HardwareObservation::confirmed(AcceleratorKind::UnifiedMemory);
    }
    if vram_gb.confidence() == Confidence::Confirmed && vram_gb.value() > 0 {
        return HardwareObservation::confirmed(AcceleratorKind::Discrete);
    }
    if is_integrated_gpu(gpu, vendor) {
        return HardwareObservation::confirmed(AcceleratorKind::Integrated);
    }
    HardwareObservation::unknown(AcceleratorKind::Unknown)
}

fn is_integrated_gpu(
    gpu: &HardwareObservation<String>,
    vendor: &HardwareObservation<AcceleratorVendor>,
) -> bool {
    if gpu.confidence() == Confidence::Unknown || vendor.value() != AcceleratorVendor::Intel {
        return false;
    }
    let identity = gpu.value().to_ascii_lowercase();
    ["uhd graphics", "hd graphics", "iris"]
        .iter()
        .any(|family| identity.contains(family))
}

fn warnings_for_profile(
    operating_system: &HardwareObservation<OperatingSystem>,
    architecture: &HardwareObservation<Architecture>,
    ram_gb: &HardwareObservation<u32>,
    gpu: &HardwareObservation<String>,
    accelerator_kind: &HardwareObservation<AcceleratorKind>,
    vram_gb: &HardwareObservation<u32>,
    unified_memory_gb: &HardwareObservation<u32>,
    storage_available_gb: &HardwareObservation<u32>,
) -> Vec<WarningCode> {
    let mut warnings = vec![WarningCode::DriverProbeDeferredToV2];
    if operating_system.confidence() == Confidence::Unsupported {
        warnings.push(WarningCode::UnsupportedOperatingSystem);
    }
    if architecture.confidence() == Confidence::Unsupported {
        warnings.push(WarningCode::UnsupportedArchitecture);
    }
    if ram_gb.value() <= 8 {
        warnings.push(WarningCode::LimitedMemory);
    }
    if storage_available_gb.confidence() == Confidence::Confirmed
        && storage_available_gb.value() < 64
    {
        warnings.push(WarningCode::LowStorage);
    }
    if gpu.confidence() == Confidence::Unknown {
        warnings.push(WarningCode::GpuProbeUnavailable);
    }
    let has_accelerator_memory = vram_gb.confidence() == Confidence::Confirmed
        || unified_memory_gb.confidence() == Confidence::Confirmed;
    if vram_gb.confidence() == Confidence::Unknown
        && !has_accelerator_memory
        && accelerator_kind.value() != AcceleratorKind::Integrated
    {
        warnings.push(WarningCode::VramProbeUnavailable);
    }
    warnings
}

fn performance_class(
    operating_system: &HardwareObservation<OperatingSystem>,
    architecture: &HardwareObservation<Architecture>,
    ram_gb: &HardwareObservation<u32>,
    vram_gb: &HardwareObservation<u32>,
    unified_memory_gb: &HardwareObservation<u32>,
) -> PerformanceClass {
    if operating_system.confidence() == Confidence::Unsupported
        || architecture.confidence() == Confidence::Unsupported
        || ram_gb.confidence() == Confidence::Unsupported
    {
        return PerformanceClass::Unknown;
    }

    let effective_memory = ram_gb
        .value()
        .max(vram_gb.value())
        .max(unified_memory_gb.value());

    match effective_memory {
        96.. => PerformanceClass::Workstation,
        32..=95 => PerformanceClass::Strong,
        16..=31 => PerformanceClass::Standard,
        8..=15 => PerformanceClass::Light,
        1..=7 => PerformanceClass::NotRecommended,
        _ => PerformanceClass::Unknown,
    }
}
