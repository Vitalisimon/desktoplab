use crate::{
    LmStudioEndpointProbe, LmStudioRuntime, OllamaRuntime, RuntimeId, RuntimeProbe, RuntimeState,
    RuntimeStatus,
};
use std::collections::HashMap;

pub trait RuntimeDetector {
    fn detect(&self) -> RuntimeDetectionOutcome;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDetectionOutcome {
    status: RuntimeStatus,
}

impl RuntimeDetectionOutcome {
    #[must_use]
    pub fn new(status: RuntimeStatus) -> Self {
        Self { status }
    }

    #[must_use]
    pub fn status(&self) -> &RuntimeStatus {
        &self.status
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryRuntimeInventoryStore {
    statuses: HashMap<RuntimeId, RuntimeStatus>,
}

impl InMemoryRuntimeInventoryStore {
    pub fn save(&mut self, status: RuntimeStatus) {
        self.statuses.insert(status.id().clone(), status);
    }

    #[must_use]
    pub fn load(&self, runtime_id: &RuntimeId) -> Option<RuntimeStatus> {
        self.statuses.get(runtime_id).cloned()
    }

    #[must_use]
    pub fn inventory(&self) -> Vec<RuntimeStatus> {
        self.statuses.values().cloned().collect()
    }
}

pub struct RuntimeDetectionService {
    detectors: Vec<Box<dyn RuntimeDetector>>,
    store: InMemoryRuntimeInventoryStore,
}

impl RuntimeDetectionService {
    #[must_use]
    pub fn new(store: InMemoryRuntimeInventoryStore) -> Self {
        Self {
            detectors: Vec::new(),
            store,
        }
    }

    pub fn register_detector(&mut self, detector: impl RuntimeDetector + 'static) {
        self.detectors.push(Box::new(detector));
    }

    pub fn detect_all(&mut self) -> RuntimeDetectionReport {
        let mut report = RuntimeDetectionReport::default();

        for detector in &self.detectors {
            let outcome = detector.detect();
            let status = outcome.status().clone();
            report.push_status(status.clone());
            self.store.save(status);
        }

        report
    }

    #[must_use]
    pub fn store(&self) -> &InMemoryRuntimeInventoryStore {
        &self.store
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeDetectionReport {
    next_sequence: u64,
    statuses: HashMap<RuntimeId, RuntimeStatus>,
    events: Vec<RuntimeDetectionEvent>,
}

impl RuntimeDetectionReport {
    fn push_status(&mut self, status: RuntimeStatus) {
        self.next_sequence += 1;
        self.events.push(RuntimeDetectionEvent {
            sequence: self.next_sequence,
            runtime_id: status.id().clone(),
            kind: event_kind_for_state(status.state()),
        });
        self.statuses.insert(status.id().clone(), status);
    }

    #[must_use]
    pub fn status(&self, runtime_id: &RuntimeId) -> Option<&RuntimeStatus> {
        self.statuses.get(runtime_id)
    }

    #[must_use]
    pub fn events(&self) -> &[RuntimeDetectionEvent] {
        &self.events
    }

    #[must_use]
    pub fn event_kinds(&self) -> Vec<RuntimeDetectionEventKind> {
        self.events
            .iter()
            .map(RuntimeDetectionEvent::kind)
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeDetectionEvent {
    sequence: u64,
    runtime_id: RuntimeId,
    kind: RuntimeDetectionEventKind,
}

impl RuntimeDetectionEvent {
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    #[must_use]
    pub fn kind(&self) -> RuntimeDetectionEventKind {
        self.kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeDetectionEventKind {
    RuntimeDetected,
    RuntimeMissing,
    RuntimeDegraded,
}

pub struct OllamaRuntimeDetector {
    runtime: OllamaRuntime,
    probe: RuntimeProbe,
}

impl OllamaRuntimeDetector {
    #[must_use]
    pub fn new(runtime: OllamaRuntime, probe: RuntimeProbe) -> Self {
        Self { runtime, probe }
    }
}

impl RuntimeDetector for OllamaRuntimeDetector {
    fn detect(&self) -> RuntimeDetectionOutcome {
        let detection = self.runtime.detect(self.probe.clone());
        let status = if detection.is_installed() {
            RuntimeStatus::installed(
                self.runtime.runtime_id().clone(),
                self.runtime.display_name(),
                detection.version().unwrap_or("unknown"),
            )
        } else {
            RuntimeStatus::not_installed(
                self.runtime.runtime_id().clone(),
                self.runtime.display_name(),
            )
        };

        RuntimeDetectionOutcome::new(status)
    }
}

pub struct LmStudioRuntimeDetector {
    runtime: LmStudioRuntime,
    probe: LmStudioEndpointProbe,
}

impl LmStudioRuntimeDetector {
    #[must_use]
    pub fn new(runtime: LmStudioRuntime, probe: LmStudioEndpointProbe) -> Self {
        Self { runtime, probe }
    }
}

impl RuntimeDetector for LmStudioRuntimeDetector {
    fn detect(&self) -> RuntimeDetectionOutcome {
        let detection = self.runtime.detect_endpoint(self.probe.clone());
        let status = if detection.is_available() {
            RuntimeStatus::installed(
                self.runtime.runtime_id().clone(),
                self.runtime.display_name(),
                detection.endpoint(),
            )
        } else {
            RuntimeStatus::degraded(
                self.runtime.runtime_id().clone(),
                self.runtime.display_name(),
                detection.reason().unwrap_or("endpoint unavailable"),
            )
        };

        RuntimeDetectionOutcome::new(status)
    }
}

fn event_kind_for_state(state: RuntimeState) -> RuntimeDetectionEventKind {
    match state {
        RuntimeState::Degraded | RuntimeState::VerificationFailed => {
            RuntimeDetectionEventKind::RuntimeDegraded
        }
        RuntimeState::NotInstalled | RuntimeState::Unknown => {
            RuntimeDetectionEventKind::RuntimeMissing
        }
        RuntimeState::Installed
        | RuntimeState::Ready
        | RuntimeState::Starting
        | RuntimeState::Running
        | RuntimeState::Stopped => RuntimeDetectionEventKind::RuntimeDetected,
    }
}
