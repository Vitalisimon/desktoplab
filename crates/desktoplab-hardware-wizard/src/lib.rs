mod adapter;
mod driver_probe;
mod frontier;
mod host_probe_parse;
mod observation;
mod profile;
mod recommendation;
mod snapshot;
mod std_host_probe;
mod wizard;

pub use adapter::{HardwareProbeAdapter, HostProbePlan, HostProbeSource, HostProbeStep};
pub use driver_probe::{
    DriverProbeObservation, DriverProbePlan, DriverProbeReport, DriverProbeSource,
    DriverProbeState, HardwareDriverProbeAdapter,
};
pub use frontier::{
    FrontierFeatureState, FrontierHardwareAssessment, FrontierHardwareClass,
    FrontierHardwareClassifier, FrontierHostFacts, FrontierHostProbeAdapter, FrontierProbeSource,
    HardwareFactSource, StdFrontierProbeSource,
};
pub use host_probe_parse::{
    linux_gpu_identity_from_lspci, linux_vram_gb_from_nvidia_smi, linux_vram_gb_from_rocm_smi,
    macos_gpu_identity_from_system_report, unix_available_gb_from_df_output,
    windows_bytes_gb_from_powershell_output, windows_gpu_identity_from_powershell_output,
    windows_gpu_probe_powershell_script, windows_ram_probe_powershell_script,
    windows_storage_probe_powershell_script, windows_vram_gb_from_powershell_output,
};
pub use observation::{Confidence, HardwareObservation};
pub use profile::{
    AcceleratorKind, AcceleratorVendor, Architecture, DriverState, HardwareProfile,
    OperatingSystem, PerformanceClass, WarningCode,
};
pub use recommendation::HardwareRecommendationInputs;
pub use snapshot::ProbeSnapshot;
pub use wizard::HardwareWizard;
