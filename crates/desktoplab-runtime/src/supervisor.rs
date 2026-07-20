use crate::{RuntimeHealth, RuntimeId, RuntimeState, RuntimeStatus};
use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeManagementMode {
    DesktopLabManaged,
    External,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeProcessSpec {
    runtime_id: RuntimeId,
    name: String,
    management_mode: RuntimeManagementMode,
    log_limit: usize,
}

impl RuntimeProcessSpec {
    #[must_use]
    pub fn managed(runtime_id: RuntimeId, name: impl Into<String>) -> Self {
        Self {
            runtime_id,
            name: name.into(),
            management_mode: RuntimeManagementMode::DesktopLabManaged,
            log_limit: 200,
        }
    }

    #[must_use]
    pub fn external(runtime_id: RuntimeId, name: impl Into<String>) -> Self {
        Self {
            runtime_id,
            name: name.into(),
            management_mode: RuntimeManagementMode::External,
            log_limit: 200,
        }
    }

    #[must_use]
    pub fn with_log_limit(mut self, log_limit: usize) -> Self {
        self.log_limit = log_limit;
        self
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn management_mode(&self) -> RuntimeManagementMode {
        self.management_mode
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeSupervisorError {
    ExternallyManaged(RuntimeId),
}

pub struct RuntimeProcessSupervisor {
    spec: RuntimeProcessSpec,
    status: RuntimeStatus,
    health_checks: VecDeque<RuntimeHealth>,
    logs: VecDeque<String>,
}

impl RuntimeProcessSupervisor {
    #[must_use]
    pub fn new(spec: RuntimeProcessSpec) -> Self {
        let status = RuntimeStatus::not_installed(spec.runtime_id().clone(), spec.name());
        Self {
            spec,
            status,
            health_checks: VecDeque::new(),
            logs: VecDeque::new(),
        }
    }

    #[must_use]
    pub fn with_health_checks(mut self, checks: impl IntoIterator<Item = RuntimeHealth>) -> Self {
        self.health_checks = checks.into_iter().collect();
        self
    }

    pub fn start(&mut self) -> Result<RuntimeSupervisorReport, RuntimeSupervisorError> {
        let mut report = RuntimeSupervisorReport::default();

        self.status.set_state(RuntimeState::Starting);
        report.push_transition(RuntimeState::Starting);

        self.status.set_state(RuntimeState::Running);
        report.push_transition(RuntimeState::Running);

        let health = self
            .health_checks
            .pop_front()
            .unwrap_or_else(RuntimeHealth::healthy);
        if health.is_healthy() {
            self.status.set_state(RuntimeState::Ready);
            report.push_transition(RuntimeState::Ready);
        } else {
            self.status
                .apply_verification(crate::VerificationResult::failed(
                    health.reason().unwrap_or("runtime health check failed"),
                ));
            report.push_transition(RuntimeState::VerificationFailed);
        }

        Ok(report)
    }

    pub fn stop(&mut self) -> Result<RuntimeSupervisorReport, RuntimeSupervisorError> {
        if self.spec.management_mode() == RuntimeManagementMode::External {
            return Err(RuntimeSupervisorError::ExternallyManaged(
                self.spec.runtime_id().clone(),
            ));
        }

        self.status.set_state(RuntimeState::Stopped);
        let mut report = RuntimeSupervisorReport::default();
        report.push_transition(RuntimeState::Stopped);
        Ok(report)
    }

    pub fn record_log(&mut self, line: impl AsRef<str>) {
        self.logs.push_back(redact_log_line(line.as_ref()));
        while self.logs.len() > self.spec.log_limit {
            self.logs.pop_front();
        }
    }

    #[must_use]
    pub fn logs(&self) -> Vec<String> {
        self.logs.iter().cloned().collect()
    }

    #[must_use]
    pub fn status(&self) -> &RuntimeStatus {
        &self.status
    }

    #[must_use]
    pub fn spec(&self) -> &RuntimeProcessSpec {
        &self.spec
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeSupervisorReport {
    state_transitions: Vec<RuntimeState>,
}

impl RuntimeSupervisorReport {
    fn push_transition(&mut self, state: RuntimeState) {
        self.state_transitions.push(state);
    }

    #[must_use]
    pub fn state_transitions(&self) -> Vec<RuntimeState> {
        self.state_transitions.clone()
    }
}

fn redact_log_line(line: &str) -> String {
    line.split_whitespace()
        .map(redact_log_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_log_token(token: &str) -> String {
    for prefix in ["api_key=", "password=", "secret=", "token="] {
        if token.to_ascii_lowercase().starts_with(prefix) {
            return format!("{prefix}[REDACTED]");
        }
    }

    token.to_string()
}
