use std::collections::BTreeSet;

use crate::diagnostics::redact_diagnostic;
use crate::{DiagnosticServiceFamily, DiagnosticServiceState, DiagnosticsSnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RepairActionFamily {
    Runtime,
    Model,
    Provider,
    Plugin,
    WorkspaceScan,
    Registry,
    Storage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepairActionMode {
    Executable,
    GuidanceOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepairAction {
    family: RepairActionFamily,
    mode: RepairActionMode,
    reason: String,
}

impl RepairAction {
    #[must_use]
    pub fn family(&self) -> RepairActionFamily {
        self.family
    }

    #[must_use]
    pub fn mode(&self) -> RepairActionMode {
        self.mode
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticsRepairPlanner {
    executable: BTreeSet<RepairActionFamily>,
}

impl DiagnosticsRepairPlanner {
    #[must_use]
    pub fn with_executable(mut self, family: RepairActionFamily) -> Self {
        self.executable.insert(family);
        self
    }

    #[must_use]
    pub fn all_executable() -> Self {
        let mut planner = Self::default();
        for family in [
            RepairActionFamily::Runtime,
            RepairActionFamily::Model,
            RepairActionFamily::Provider,
            RepairActionFamily::Plugin,
            RepairActionFamily::WorkspaceScan,
            RepairActionFamily::Registry,
            RepairActionFamily::Storage,
        ] {
            planner.executable.insert(family);
        }
        planner
    }

    #[must_use]
    pub fn plan(&self, snapshot: &DiagnosticsSnapshot) -> DiagnosticsRepairPlan {
        let actions = snapshot
            .services()
            .iter()
            .filter_map(|service| {
                let DiagnosticServiceState::Degraded(reason) = service.state() else {
                    return None;
                };
                let family = repair_family(service.family())?;
                let mode = repair_mode(family, reason, &self.executable);
                Some(RepairAction {
                    family,
                    mode,
                    reason: redact_diagnostic(reason),
                })
            })
            .collect();
        DiagnosticsRepairPlan { actions }
    }
}

fn repair_mode(
    family: RepairActionFamily,
    reason: &str,
    executable: &BTreeSet<RepairActionFamily>,
) -> RepairActionMode {
    if family == RepairActionFamily::Runtime && reason.contains("os_level_repair_unsupported") {
        return RepairActionMode::GuidanceOnly;
    }
    if executable.contains(&family) {
        RepairActionMode::Executable
    } else {
        RepairActionMode::GuidanceOnly
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticsRepairPlan {
    actions: Vec<RepairAction>,
}

impl DiagnosticsRepairPlan {
    #[must_use]
    pub fn has_action(&self, family: RepairActionFamily, mode: RepairActionMode) -> bool {
        self.actions
            .iter()
            .any(|action| action.family == family && action.mode == mode)
    }

    #[must_use]
    pub fn summary(&self) -> String {
        self.actions
            .iter()
            .map(|action| format!("{:?}:{:?}:{}", action.family, action.mode, action.reason))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[must_use]
    pub fn actions(&self) -> &[RepairAction] {
        &self.actions
    }
}

fn repair_family(family: DiagnosticServiceFamily) -> Option<RepairActionFamily> {
    match family {
        DiagnosticServiceFamily::Runtime => Some(RepairActionFamily::Runtime),
        DiagnosticServiceFamily::Model => Some(RepairActionFamily::Model),
        DiagnosticServiceFamily::Provider => Some(RepairActionFamily::Provider),
        DiagnosticServiceFamily::Plugin => Some(RepairActionFamily::Plugin),
        DiagnosticServiceFamily::WorkspaceScan => Some(RepairActionFamily::WorkspaceScan),
        DiagnosticServiceFamily::Registry => Some(RepairActionFamily::Registry),
        DiagnosticServiceFamily::Storage => Some(RepairActionFamily::Storage),
        DiagnosticServiceFamily::Session | DiagnosticServiceFamily::Job => None,
    }
}
