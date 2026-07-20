use crate::HardwareObservation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeSnapshot {
    operating_system: Option<String>,
    architecture: Option<String>,
    cpu: Option<String>,
    ram_gb: HardwareObservation<u32>,
    gpu: Option<String>,
    vram_gb: HardwareObservation<u32>,
    unified_memory_gb: HardwareObservation<u32>,
    storage_available_gb: HardwareObservation<u32>,
}

impl ProbeSnapshot {
    #[must_use]
    pub fn new() -> Self {
        Self {
            operating_system: None,
            architecture: None,
            cpu: None,
            ram_gb: HardwareObservation::unknown(0),
            gpu: None,
            vram_gb: HardwareObservation::unknown(0),
            unified_memory_gb: HardwareObservation::unknown(0),
            storage_available_gb: HardwareObservation::unknown(0),
        }
    }

    #[must_use]
    pub fn with_operating_system(mut self, operating_system: impl Into<String>) -> Self {
        self.operating_system = Some(operating_system.into());
        self
    }

    #[must_use]
    pub fn with_architecture(mut self, architecture: impl Into<String>) -> Self {
        self.architecture = Some(architecture.into());
        self
    }

    #[must_use]
    pub fn with_cpu(mut self, cpu: impl Into<String>) -> Self {
        self.cpu = Some(cpu.into());
        self
    }

    #[must_use]
    pub fn with_ram_gb(mut self, ram_gb: u32) -> Self {
        self.ram_gb = HardwareObservation::confirmed(ram_gb);
        self
    }

    #[must_use]
    pub fn with_ram(mut self, ram_gb: HardwareObservation<u32>) -> Self {
        self.ram_gb = ram_gb;
        self
    }

    #[must_use]
    pub fn with_gpu(mut self, gpu: impl Into<String>) -> Self {
        self.gpu = Some(gpu.into());
        self
    }

    #[must_use]
    pub fn with_vram_gb(mut self, vram_gb: u32) -> Self {
        self.vram_gb = HardwareObservation::confirmed(vram_gb);
        self
    }

    #[must_use]
    pub fn with_vram(mut self, vram_gb: HardwareObservation<u32>) -> Self {
        self.vram_gb = vram_gb;
        self
    }

    #[must_use]
    pub fn with_unified_memory_gb(mut self, unified_memory_gb: u32) -> Self {
        self.unified_memory_gb = HardwareObservation::confirmed(unified_memory_gb);
        self
    }

    #[must_use]
    pub fn with_unified_memory(mut self, unified_memory_gb: HardwareObservation<u32>) -> Self {
        self.unified_memory_gb = unified_memory_gb;
        self
    }

    #[must_use]
    pub fn with_storage_available_gb(mut self, storage_available_gb: u32) -> Self {
        self.storage_available_gb = HardwareObservation::confirmed(storage_available_gb);
        self
    }

    #[must_use]
    pub fn with_storage_available(
        mut self,
        storage_available_gb: HardwareObservation<u32>,
    ) -> Self {
        self.storage_available_gb = storage_available_gb;
        self
    }

    pub(crate) fn operating_system(&self) -> Option<&str> {
        self.operating_system.as_deref()
    }

    pub(crate) fn architecture(&self) -> Option<&str> {
        self.architecture.as_deref()
    }

    pub(crate) fn cpu(&self) -> Option<&str> {
        self.cpu.as_deref()
    }

    pub(crate) fn ram_gb(&self) -> &HardwareObservation<u32> {
        &self.ram_gb
    }

    pub(crate) fn gpu(&self) -> Option<&str> {
        self.gpu.as_deref()
    }

    pub(crate) fn vram_gb(&self) -> &HardwareObservation<u32> {
        &self.vram_gb
    }

    pub(crate) fn unified_memory_gb(&self) -> &HardwareObservation<u32> {
        &self.unified_memory_gb
    }

    pub(crate) fn storage_available_gb(&self) -> &HardwareObservation<u32> {
        &self.storage_available_gb
    }
}

impl Default for ProbeSnapshot {
    fn default() -> Self {
        Self::new()
    }
}
