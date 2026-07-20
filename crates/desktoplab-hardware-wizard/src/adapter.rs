use crate::std_host_probe::{
    host_cpu_name, host_gpu_name, host_ram_gb, host_storage_available_gb, host_vram_gb,
};
use crate::{HardwareObservation, HardwareProfile, HardwareWizard, ProbeSnapshot};

#[derive(Clone, Debug)]
pub struct HardwareProbeAdapter<S> {
    source: S,
}

impl<S> HardwareProbeAdapter<S>
where
    S: HostProbeSource,
{
    #[must_use]
    pub fn new(source: S) -> Self {
        Self { source }
    }

    #[must_use]
    pub fn snapshot(&self) -> ProbeSnapshot {
        let mut snapshot = ProbeSnapshot::new()
            .with_ram(self.source.ram_gb())
            .with_storage_available(self.source.storage_available_gb())
            .with_vram(self.source.vram_gb())
            .with_unified_memory(self.source.unified_memory_gb());

        if let Some(operating_system) = self.source.operating_system() {
            snapshot = snapshot.with_operating_system(operating_system);
        }
        if let Some(architecture) = self.source.architecture() {
            snapshot = snapshot.with_architecture(architecture);
        }
        if let Some(cpu) = self.source.cpu() {
            snapshot = snapshot.with_cpu(cpu);
        }
        if let Some(gpu) = self.source.gpu() {
            snapshot = snapshot.with_gpu(gpu);
        }

        snapshot
    }

    #[must_use]
    pub fn profile(&self) -> HardwareProfile {
        HardwareWizard::v1().profile(self.snapshot())
    }
}

impl HardwareProbeAdapter<StdHostProbeSource> {
    #[must_use]
    pub fn for_current_host() -> Self {
        Self::new(StdHostProbeSource)
    }
}

pub trait HostProbeSource {
    fn operating_system(&self) -> Option<String>;
    fn architecture(&self) -> Option<String>;
    fn cpu(&self) -> Option<String>;
    fn ram_gb(&self) -> HardwareObservation<u32>;
    fn storage_available_gb(&self) -> HardwareObservation<u32>;
    fn gpu(&self) -> Option<String>;
    fn vram_gb(&self) -> HardwareObservation<u32>;
    fn unified_memory_gb(&self) -> HardwareObservation<u32>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StdHostProbeSource;

impl HostProbeSource for StdHostProbeSource {
    fn operating_system(&self) -> Option<String> {
        Some(std::env::consts::OS.to_string())
    }

    fn architecture(&self) -> Option<String> {
        Some(std::env::consts::ARCH.to_string())
    }

    fn cpu(&self) -> Option<String> {
        host_cpu_name()
    }

    fn ram_gb(&self) -> HardwareObservation<u32> {
        host_ram_gb().map_or_else(
            || HardwareObservation::unknown(0),
            HardwareObservation::confirmed,
        )
    }

    fn storage_available_gb(&self) -> HardwareObservation<u32> {
        host_storage_available_gb().map_or_else(
            || HardwareObservation::unknown(0),
            HardwareObservation::confirmed,
        )
    }

    fn gpu(&self) -> Option<String> {
        host_gpu_name()
    }

    fn vram_gb(&self) -> HardwareObservation<u32> {
        host_vram_gb().map_or_else(
            || HardwareObservation::unknown(0),
            HardwareObservation::confirmed,
        )
    }

    fn unified_memory_gb(&self) -> HardwareObservation<u32> {
        if std::env::consts::OS == "macos" && std::env::consts::ARCH == "aarch64" {
            return self.ram_gb();
        }

        HardwareObservation::unknown(0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostProbePlan {
    steps: Vec<HostProbeStep>,
}

impl HostProbePlan {
    #[must_use]
    pub fn v1() -> Self {
        Self {
            steps: vec![
                HostProbeStep::new("operating_system"),
                HostProbeStep::new("architecture"),
                HostProbeStep::new("cpu"),
                HostProbeStep::new("ram"),
                HostProbeStep::new("storage_available"),
                HostProbeStep::new("gpu_best_effort"),
                HostProbeStep::new("vram_best_effort"),
                HostProbeStep::new("unified_memory_best_effort"),
            ],
        }
    }

    #[must_use]
    pub fn steps(&self) -> &[HostProbeStep] {
        &self.steps
    }

    #[must_use]
    pub fn requires_elevated_permissions(&self) -> bool {
        self.steps.iter().any(HostProbeStep::requires_elevation)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostProbeStep {
    name: String,
    requires_elevation: bool,
}

impl HostProbeStep {
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            requires_elevation: false,
        }
    }

    #[must_use]
    pub fn requires_elevation(&self) -> bool {
        self.requires_elevation
    }
}
