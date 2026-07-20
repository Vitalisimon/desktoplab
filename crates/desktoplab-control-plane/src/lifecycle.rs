use std::time::{Duration, Instant};

use crate::{ApiSurface, VersionInfo};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LifecycleState {
    Initialized,
    Stopping,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadinessState {
    Starting,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlPlaneStatus {
    Healthy,
    Draining,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShutdownMode {
    Graceful,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StabilityClock {
    started_at: Instant,
    last_route_decision_at: Instant,
}

impl Default for StabilityClock {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            started_at: now,
            last_route_decision_at: now,
        }
    }
}

impl StabilityClock {
    pub(crate) fn mark_route_decision(&mut self) {
        self.last_route_decision_at = Instant::now();
    }

    #[must_use]
    pub(crate) fn uptime_ms(&self) -> u64 {
        duration_ms(self.started_at.elapsed())
    }

    #[must_use]
    pub(crate) fn route_decision_age_ms(&self) -> u64 {
        duration_ms(self.last_route_decision_at.elapsed())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct StabilityBudget {
    memory_budget_mb: u64,
    disk_budget_mb: u64,
}

impl Default for StabilityBudget {
    fn default() -> Self {
        Self {
            memory_budget_mb: 512,
            disk_budget_mb: 2048,
        }
    }
}

impl StabilityBudget {
    #[must_use]
    pub(crate) fn memory_budget_mb(&self) -> u64 {
        self.memory_budget_mb
    }

    #[must_use]
    pub(crate) fn disk_budget_mb(&self) -> u64 {
        self.disk_budget_mb
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPlaneHealth {
    status: ControlPlaneStatus,
}

impl ControlPlaneHealth {
    #[must_use]
    pub fn status(&self) -> ControlPlaneStatus {
        self.status
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPlaneReadiness {
    state: ReadinessState,
}

impl ControlPlaneReadiness {
    #[must_use]
    pub fn state(&self) -> ReadinessState {
        self.state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlPlane {
    version: VersionInfo,
    lifecycle_state: LifecycleState,
    readiness_state: ReadinessState,
    shutdown_mode: Option<ShutdownMode>,
}

impl ControlPlane {
    #[must_use]
    pub fn new(version: VersionInfo) -> Self {
        Self {
            version,
            lifecycle_state: LifecycleState::Initialized,
            readiness_state: ReadinessState::Starting,
            shutdown_mode: None,
        }
    }

    pub fn mark_ready(&mut self) {
        self.readiness_state = ReadinessState::Ready;
    }

    pub fn request_shutdown(&mut self, mode: ShutdownMode) {
        self.lifecycle_state = LifecycleState::Stopping;
        self.shutdown_mode = Some(mode);
    }

    #[must_use]
    pub fn lifecycle_state(&self) -> LifecycleState {
        self.lifecycle_state
    }

    #[must_use]
    pub fn health(&self) -> ControlPlaneHealth {
        let status = match self.lifecycle_state {
            LifecycleState::Initialized => ControlPlaneStatus::Healthy,
            LifecycleState::Stopping => ControlPlaneStatus::Draining,
        };

        ControlPlaneHealth { status }
    }

    #[must_use]
    pub fn readiness(&self) -> ControlPlaneReadiness {
        ControlPlaneReadiness {
            state: self.readiness_state,
        }
    }

    #[must_use]
    pub fn version(&self) -> &VersionInfo {
        &self.version
    }

    #[must_use]
    pub fn api_surface(&self) -> ApiSurface {
        ApiSurface::v1()
    }

    #[must_use]
    pub fn shutdown_mode(&self) -> Option<ShutdownMode> {
        self.shutdown_mode
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
