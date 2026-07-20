use desktoplab_backend_services::{
    BackendEventScope, BackendEventStreamService, DiagnosticsBundleGuard, EventReplayRequest,
    ProductizationPerformanceGate, WorkspaceScanGuard,
};
use desktoplab_downloads::{DownloadCapacity, ProductDownloadReservation};

#[test]
fn productization_performance_gate_bounds_large_scan_download_progress_replay_and_diagnostics() {
    let gate = ProductizationPerformanceGate::new(3, 256, 64);
    let scan = WorkspaceScanGuard::new(3).classify(10);
    assert!(scan.is_degraded());

    let reservation = ProductDownloadReservation::new(2_000, DownloadCapacity::new(1_000));
    assert!(reservation.reserve().is_err());

    let mut events = BackendEventStreamService::default();
    for index in 0..300 {
        events.publish(BackendEventScope::Session, format!("event={index}"));
    }
    let replay = events.replay(EventReplayRequest::new());
    assert_eq!(replay.sequences().len(), 256);

    assert!(
        DiagnosticsBundleGuard::new(64)
            .check(&"x".repeat(128))
            .is_err()
    );
    assert!(gate.pass(scan, replay.sequences().len(), 64));
}
