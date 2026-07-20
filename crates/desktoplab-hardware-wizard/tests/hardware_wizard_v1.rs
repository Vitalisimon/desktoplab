use desktoplab_hardware_wizard::{
    AcceleratorKind, AcceleratorVendor, Architecture, Confidence, DriverState, HardwareObservation,
    HardwareWizard, OperatingSystem, PerformanceClass, ProbeSnapshot, WarningCode,
};
use xtask::check_logical_line_limit;

#[test]
fn normalizes_baseline_profiles_across_supported_operating_systems() {
    let wizard = HardwareWizard::v1();

    let mac = wizard.profile(
        ProbeSnapshot::new()
            .with_operating_system("macOS")
            .with_architecture("arm64")
            .with_cpu("Apple M4 Pro")
            .with_ram_gb(48)
            .with_unified_memory_gb(48)
            .with_storage_available_gb(900),
    );
    let windows = wizard.profile(
        ProbeSnapshot::new()
            .with_operating_system("Windows 11 Pro")
            .with_architecture("x86_64")
            .with_cpu("Ryzen AI Max")
            .with_ram_gb(64)
            .with_gpu("AMD Radeon 8060S")
            .with_vram_gb(32)
            .with_storage_available_gb(512),
    );
    let linux = wizard.profile(
        ProbeSnapshot::new()
            .with_operating_system("Ubuntu 26.04 LTS")
            .with_architecture("amd64")
            .with_cpu("NVIDIA Grace")
            .with_ram_gb(128)
            .with_gpu("NVIDIA RTX")
            .with_vram_gb(96)
            .with_storage_available_gb(2_000),
    );

    assert_eq!(mac.operating_system().value(), OperatingSystem::Macos);
    assert_eq!(mac.architecture().value(), Architecture::Aarch64);
    assert_eq!(mac.performance_class(), PerformanceClass::Strong);
    assert_eq!(windows.operating_system().value(), OperatingSystem::Windows);
    assert_eq!(windows.architecture().value(), Architecture::X86_64);
    assert_eq!(windows.performance_class(), PerformanceClass::Strong);
    assert_eq!(linux.operating_system().value(), OperatingSystem::Linux);
    assert_eq!(linux.architecture().value(), Architecture::X86_64);
    assert_eq!(linux.performance_class(), PerformanceClass::Workstation);
}

#[test]
fn missing_optional_probes_degrade_explicitly_without_blocking_profile_creation() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("linux")
            .with_architecture("x86_64")
            .with_cpu("Intel Core")
            .with_ram_gb(16)
            .with_storage_available_gb(120),
    );

    assert_eq!(profile.gpu().confidence(), Confidence::Unknown);
    assert_eq!(profile.vram_gb().confidence(), Confidence::Unknown);
    assert_eq!(
        profile.unified_memory_gb().confidence(),
        Confidence::Unknown
    );
    assert!(profile.is_degraded());
    assert!(profile.has_warning(WarningCode::GpuProbeUnavailable));
    assert!(profile.has_warning(WarningCode::VramProbeUnavailable));
    assert!(profile.has_warning(WarningCode::DriverProbeDeferredToV2));
}

#[test]
fn accelerator_schema_carries_vendor_kind_vram_driver_state_and_confidence() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("linux")
            .with_architecture("x86_64")
            .with_cpu("NVIDIA Grace")
            .with_ram_gb(128)
            .with_gpu("NVIDIA RTX 6000 Ada")
            .with_vram_gb(48)
            .with_storage_available_gb(1_000),
    );

    assert_eq!(
        profile.accelerator_vendor().value(),
        AcceleratorVendor::Nvidia
    );
    assert_eq!(
        profile.accelerator_kind().value(),
        AcceleratorKind::Discrete
    );
    assert_eq!(profile.vram_gb().value(), 48);
    assert_eq!(profile.vram_gb().confidence(), Confidence::Confirmed);
    assert_eq!(profile.driver_state().value(), DriverState::DeferredToV2);
    assert_eq!(profile.driver_state().confidence(), Confidence::Unknown);
}

#[test]
fn integrated_intel_graphics_do_not_require_dedicated_vram() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("Linux")
            .with_architecture("x86_64")
            .with_cpu("Intel N95")
            .with_ram_gb(8)
            .with_gpu("Intel Corporation Alder Lake-N [UHD Graphics]")
            .with_storage_available_gb(151),
    );

    assert_eq!(
        profile.accelerator_kind().value(),
        AcceleratorKind::Integrated
    );
    assert!(!profile.has_warning(WarningCode::VramProbeUnavailable));
}

#[test]
fn confirmed_unified_memory_replaces_missing_vram_warning_on_apple_silicon() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("macOS")
            .with_architecture("arm64")
            .with_cpu("Apple M4 Max")
            .with_gpu("Apple M4 Max")
            .with_ram_gb(64)
            .with_unified_memory_gb(64)
            .with_storage_available_gb(900),
    );

    assert_eq!(
        profile.accelerator_vendor().value(),
        AcceleratorVendor::Apple
    );
    assert_eq!(
        profile.accelerator_kind().value(),
        AcceleratorKind::UnifiedMemory
    );
    assert_eq!(
        profile.unified_memory_gb().confidence(),
        Confidence::Confirmed
    );
    assert!(!profile.has_warning(WarningCode::VramProbeUnavailable));
}

#[test]
fn large_unified_memory_profiles_are_labeled_as_local_workstations() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("Linux")
            .with_architecture("arm64")
            .with_cpu("NVIDIA Grace Blackwell")
            .with_gpu("NVIDIA GB10")
            .with_ram_gb(128)
            .with_unified_memory_gb(128)
            .with_storage_available_gb(4_000),
    );
    let recommendation = HardwareWizard::v1().recommendation_inputs(&profile);

    assert_eq!(profile.performance_class(), PerformanceClass::Workstation);
    assert_eq!(recommendation.performance_label(), "Local workstation");
    assert!(!profile.has_warning(WarningCode::VramProbeUnavailable));
}

#[test]
fn recommendation_inputs_include_warnings_and_expected_limitations() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("Windows")
            .with_architecture("x64")
            .with_cpu("low power cpu")
            .with_ram_gb(8)
            .with_storage_available_gb(24),
    );

    let recommendation = HardwareWizard::v1().recommendation_inputs(&profile);

    assert_eq!(recommendation.performance_class(), PerformanceClass::Light);
    assert!(recommendation.has_warning(WarningCode::LimitedMemory));
    assert!(recommendation.has_warning(WarningCode::LowStorage));
    assert!(
        recommendation.expected_limitations().contains(
            &"small local models or cloud/external backends are more realistic".to_string()
        )
    );
    assert!(
        !recommendation
            .expected_limitations()
            .iter()
            .any(|limitation| limitation.contains("driver pass"))
    );
    assert!(
        !recommendation
            .expected_limitations()
            .contains(&"accelerator confidence requires v2 driver/runtime probing".to_string())
    );
    assert!(
        recommendation
            .setup_inputs()
            .contains(&"storage.available_gb".to_string())
    );
}

#[test]
fn unsupported_required_probes_are_not_presented_as_confirmed() {
    let profile = HardwareWizard::v1().profile(
        ProbeSnapshot::new()
            .with_operating_system("Solaris")
            .with_architecture("sparc")
            .with_ram(HardwareObservation::unsupported(32)),
    );

    assert_eq!(
        profile.operating_system().confidence(),
        Confidence::Unsupported
    );
    assert_eq!(profile.architecture().confidence(), Confidence::Unsupported);
    assert_eq!(profile.ram_gb().confidence(), Confidence::Unsupported);
    assert_eq!(profile.performance_class(), PerformanceClass::Unknown);
    assert!(profile.has_warning(WarningCode::UnsupportedOperatingSystem));
    assert!(profile.has_warning(WarningCode::UnsupportedArchitecture));
}

#[test]
fn hardware_wizard_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-hardware-wizard/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/observation.rs",
            include_str!("../src/observation.rs"),
            250,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/profile.rs",
            include_str!("../src/profile.rs"),
            250,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/recommendation.rs",
            include_str!("../src/recommendation.rs"),
            250,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/snapshot.rs",
            include_str!("../src/snapshot.rs"),
            250,
        ),
        (
            "crates/desktoplab-hardware-wizard/src/wizard.rs",
            include_str!("../src/wizard.rs"),
            250,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("hardware wizard source should stay below the initial line-count guard");
    }
}
