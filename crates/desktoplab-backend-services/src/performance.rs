use crate::WorkspaceScanDecision;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticsBundleGuard {
    max_bytes: usize,
}

impl DiagnosticsBundleGuard {
    #[must_use]
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    pub fn check(&self, bundle: &str) -> Result<(), &'static str> {
        if bundle.len() > self.max_bytes {
            Err("diagnostics_bundle_too_large")
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductizationPerformanceGate {
    max_scan_files: usize,
    max_replay_events: usize,
    max_diagnostics_bytes: usize,
}

impl ProductizationPerformanceGate {
    #[must_use]
    pub fn new(
        max_scan_files: usize,
        max_replay_events: usize,
        max_diagnostics_bytes: usize,
    ) -> Self {
        Self {
            max_scan_files,
            max_replay_events,
            max_diagnostics_bytes,
        }
    }

    #[must_use]
    pub fn pass(
        &self,
        scan: WorkspaceScanDecision,
        replayed_events: usize,
        diagnostics_bytes: usize,
    ) -> bool {
        scan.is_degraded()
            && replayed_events <= self.max_replay_events
            && diagnostics_bytes <= self.max_diagnostics_bytes
            && self.max_scan_files > 0
    }
}
