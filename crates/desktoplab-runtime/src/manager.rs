use std::collections::HashMap;

use crate::{RuntimeId, RuntimeLifecycleBoundary, RuntimeState, RuntimeStatus};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeCommand {
    MarkInstalled {
        runtime_id: RuntimeId,
        version: String,
    },
    Start {
        runtime_id: RuntimeId,
    },
    Stop {
        runtime_id: RuntimeId,
    },
}

impl RuntimeCommand {
    #[must_use]
    pub fn mark_installed(runtime_id: RuntimeId, version: impl Into<String>) -> Self {
        Self::MarkInstalled {
            runtime_id,
            version: version.into(),
        }
    }

    #[must_use]
    pub fn start(runtime_id: RuntimeId) -> Self {
        Self::Start { runtime_id }
    }

    #[must_use]
    pub fn stop(runtime_id: RuntimeId) -> Self {
        Self::Stop { runtime_id }
    }
}

#[derive(Clone, Debug, Default)]
pub struct RuntimeManager {
    runtimes: HashMap<RuntimeId, RuntimeStatus>,
    audit_log: Vec<RuntimeCommand>,
}

impl RuntimeManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_runtime(&mut self, runtime_id: RuntimeId, name: impl Into<String>) {
        self.runtimes.insert(
            runtime_id.clone(),
            RuntimeStatus::not_installed(runtime_id, name),
        );
    }

    pub fn apply(&mut self, command: RuntimeCommand) {
        match &command {
            RuntimeCommand::MarkInstalled {
                runtime_id,
                version,
            } => {
                if let Some(status) = self.runtimes.get_mut(runtime_id) {
                    status.set_installed(version);
                }
            }
            RuntimeCommand::Start { runtime_id } => {
                if let Some(status) = self.runtimes.get_mut(runtime_id) {
                    status.set_state(RuntimeState::Running);
                }
            }
            RuntimeCommand::Stop { runtime_id } => {
                if let Some(status) = self.runtimes.get_mut(runtime_id) {
                    status.set_state(RuntimeState::Stopped);
                }
            }
        }
        self.audit_log.push(command);
    }

    pub fn set_lifecycle(
        &mut self,
        runtime_id: &RuntimeId,
        update_lifecycle: RuntimeLifecycleBoundary,
        uninstall_lifecycle: RuntimeLifecycleBoundary,
    ) {
        if let Some(status) = self.runtimes.get_mut(runtime_id) {
            status.set_lifecycle(update_lifecycle, uninstall_lifecycle);
        }
    }

    #[must_use]
    pub fn status(&self, runtime_id: &RuntimeId) -> RuntimeStatus {
        self.runtimes
            .get(runtime_id)
            .cloned()
            .unwrap_or_else(|| RuntimeStatus::missing(runtime_id.clone()))
    }

    #[must_use]
    pub fn inventory(&self) -> Vec<RuntimeStatus> {
        self.runtimes.values().cloned().collect()
    }

    #[must_use]
    pub fn audit_log(&self) -> &[RuntimeCommand] {
        &self.audit_log
    }
}
