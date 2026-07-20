use desktoplab_hardware_wizard::{
    Architecture, Confidence, HardwareObservation, HardwareProbeAdapter, HostProbePlan,
    HostProbeSource, OperatingSystem, PerformanceClass, WarningCode, linux_gpu_identity_from_lspci,
    linux_vram_gb_from_nvidia_smi, linux_vram_gb_from_rocm_smi,
    macos_gpu_identity_from_system_report, unix_available_gb_from_df_output,
};
use xtask::check_logical_line_limit;

#[test]
fn probe_output_normalizes_into_hardware_profile() {
    let adapter = HardwareProbeAdapter::new(FixtureProbeSource {
        operating_system: Some("Ubuntu 26.04 LTS".to_string()),
        architecture: Some("amd64".to_string()),
        cpu: Some("NVIDIA Grace".to_string()),
        ram_gb: HardwareObservation::confirmed(128),
        storage_available_gb: HardwareObservation::confirmed(2_000),
        gpu: Some("NVIDIA RTX".to_string()),
        vram_gb: HardwareObservation::confirmed(96),
        unified_memory_gb: HardwareObservation::unknown(0),
    });

    let profile = adapter.profile();

    assert_eq!(profile.operating_system().value(), OperatingSystem::Linux);
    assert_eq!(profile.architecture().value(), Architecture::X86_64);
    assert_eq!(profile.performance_class(), PerformanceClass::Workstation);
    assert_eq!(profile.storage_available_gb().value(), 2_000);
}

#[test]
fn missing_gpu_and_vram_probes_degrade_explicitly() {
    let adapter = HardwareProbeAdapter::new(FixtureProbeSource {
        operating_system: Some("macOS".to_string()),
        architecture: Some("arm64".to_string()),
        cpu: Some("Apple M4".to_string()),
        ram_gb: HardwareObservation::confirmed(24),
        storage_available_gb: HardwareObservation::confirmed(512),
        gpu: None,
        vram_gb: HardwareObservation::unknown(0),
        unified_memory_gb: HardwareObservation::unknown(0),
    });

    let profile = adapter.profile();

    assert_eq!(profile.gpu().confidence(), Confidence::Unknown);
    assert_eq!(profile.vram_gb().confidence(), Confidence::Unknown);
    assert!(profile.is_degraded());
    assert!(profile.has_warning(WarningCode::GpuProbeUnavailable));
    assert!(profile.has_warning(WarningCode::VramProbeUnavailable));
}

#[test]
fn macos_gpu_identity_is_parsed_from_apple_silicon_system_report() {
    let report = r#"
Graphics/Displays:

    Apple M4 Max:

      Chipset Model: Apple M4 Max
      Type: GPU
      Bus: Built-In
      Total Number of Cores: 40
"#;

    assert_eq!(
        macos_gpu_identity_from_system_report(report, Some("Apple M4 Max")),
        Some("Apple M4 Max".to_string())
    );
}

#[test]
fn linux_gpu_identity_is_parsed_from_lspci_display_devices() {
    let lspci = r#"
00:02.0 VGA compatible controller: Intel Corporation Raptor Lake-P [Iris Xe Graphics]
01:00.0 3D controller: NVIDIA Corporation AD104GL [RTX 4000 SFF Ada Generation]
"#;

    assert_eq!(
        linux_gpu_identity_from_lspci(lspci),
        Some("NVIDIA Corporation AD104GL [RTX 4000 SFF Ada Generation]".to_string())
    );
}

#[test]
fn linux_vram_is_parsed_from_nvidia_smi_mib_output() {
    assert_eq!(linux_vram_gb_from_nvidia_smi("24564 MiB\n"), Some(24));
}

#[test]
fn linux_vram_is_parsed_from_rocm_smi_vram_output() {
    let rocm = "GPU[0]\t\t: VRAM Total Memory (B): 34342961152\n";

    assert_eq!(linux_vram_gb_from_rocm_smi(rocm), Some(32));
}

#[test]
fn unix_storage_parser_handles_physical_and_appimage_filesystems() {
    let physical = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/nvme0n1p2 243937628 73981884 157491568 32% /\n";
    let appimage = "Filesystem 1024-blocks Used Available Capacity Mounted on\nDesktopLab.AppImage 137216 137216 0 100% /tmp/.mount_DesktopLab\n";

    assert_eq!(unix_available_gb_from_df_output(physical), Some(151));
    assert_eq!(unix_available_gb_from_df_output(appimage), Some(0));
}

#[test]
fn unsupported_probe_values_are_not_marked_confirmed() {
    let adapter = HardwareProbeAdapter::new(FixtureProbeSource {
        operating_system: Some("Solaris".to_string()),
        architecture: Some("sparc".to_string()),
        cpu: Some("unknown".to_string()),
        ram_gb: HardwareObservation::unsupported(64),
        storage_available_gb: HardwareObservation::confirmed(512),
        gpu: None,
        vram_gb: HardwareObservation::unknown(0),
        unified_memory_gb: HardwareObservation::unknown(0),
    });

    let profile = adapter.profile();

    assert_eq!(
        profile.operating_system().confidence(),
        Confidence::Unsupported
    );
    assert_eq!(profile.architecture().confidence(), Confidence::Unsupported);
    assert_eq!(profile.ram_gb().confidence(), Confidence::Unsupported);
    assert_eq!(profile.performance_class(), PerformanceClass::Unknown);
}

#[test]
fn v1_probe_plan_requires_no_elevated_permission() {
    let plan = HostProbePlan::v1();

    assert!(!plan.requires_elevated_permissions());
    assert!(plan.steps().iter().all(|step| !step.requires_elevation()));
}

#[test]
fn hardware_probe_adapter_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-hardware-wizard/src/adapter.rs",
            include_str!("../src/adapter.rs"),
            280,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/host_probe_parse.rs",
            include_str!("../src/host_probe_parse.rs"),
            180,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/std_host_probe.rs",
            include_str!("../src/std_host_probe.rs"),
            180,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("hardware probe source files should stay below the line-count guard");
    }
}

struct FixtureProbeSource {
    operating_system: Option<String>,
    architecture: Option<String>,
    cpu: Option<String>,
    ram_gb: HardwareObservation<u32>,
    storage_available_gb: HardwareObservation<u32>,
    gpu: Option<String>,
    vram_gb: HardwareObservation<u32>,
    unified_memory_gb: HardwareObservation<u32>,
}

impl HostProbeSource for FixtureProbeSource {
    fn operating_system(&self) -> Option<String> {
        self.operating_system.clone()
    }

    fn architecture(&self) -> Option<String> {
        self.architecture.clone()
    }

    fn cpu(&self) -> Option<String> {
        self.cpu.clone()
    }

    fn ram_gb(&self) -> HardwareObservation<u32> {
        self.ram_gb.clone()
    }

    fn storage_available_gb(&self) -> HardwareObservation<u32> {
        self.storage_available_gb.clone()
    }

    fn gpu(&self) -> Option<String> {
        self.gpu.clone()
    }

    fn vram_gb(&self) -> HardwareObservation<u32> {
        self.vram_gb.clone()
    }

    fn unified_memory_gb(&self) -> HardwareObservation<u32> {
        self.unified_memory_gb.clone()
    }
}
