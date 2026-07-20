use desktoplab_hardware_wizard::{
    DriverProbeObservation, DriverProbePlan, DriverProbeSource, DriverProbeState,
    HardwareDriverProbeAdapter,
};
use xtask::check_logical_line_limit;

#[test]
fn missing_driver_probe_is_explicit_and_not_confirmed() {
    let adapter = HardwareDriverProbeAdapter::new(FixtureDriverSource::default());
    let report = adapter.report();

    assert_eq!(report.cuda().state(), DriverProbeState::Unknown);
    assert_eq!(report.rocm().state(), DriverProbeState::Unknown);
    assert_eq!(report.metal().state(), DriverProbeState::Unknown);
    assert!(!report.cuda().is_confirmed());
}

#[test]
fn supported_driver_probe_carries_version() {
    let adapter = HardwareDriverProbeAdapter::new(FixtureDriverSource {
        cuda: DriverProbeObservation::confirmed("12.5"),
        rocm: DriverProbeObservation::unsupported("not installed"),
        metal: DriverProbeObservation::confirmed("3"),
    });
    let report = adapter.report();

    assert_eq!(report.cuda().version(), Some("12.5"));
    assert_eq!(report.rocm().state(), DriverProbeState::Unsupported);
    assert_eq!(report.metal().version(), Some("3"));
}

#[test]
fn v2_driver_probe_plan_requires_no_elevated_permission() {
    let plan = DriverProbePlan::v2();

    assert!(!plan.requires_elevated_permissions());
}

#[test]
fn hardware_driver_probe_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-hardware-wizard/src/driver_probe.rs",
        include_str!("../src/driver_probe.rs"),
        240,
    )
    .expect("driver probe source should stay focused");
}

#[derive(Default)]
struct FixtureDriverSource {
    cuda: DriverProbeObservation,
    rocm: DriverProbeObservation,
    metal: DriverProbeObservation,
}

impl DriverProbeSource for FixtureDriverSource {
    fn cuda(&self) -> DriverProbeObservation {
        self.cuda.clone()
    }

    fn rocm(&self) -> DriverProbeObservation {
        self.rocm.clone()
    }

    fn metal(&self) -> DriverProbeObservation {
        self.metal.clone()
    }
}
