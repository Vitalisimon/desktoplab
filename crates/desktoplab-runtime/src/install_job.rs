use crate::{InstallPlan, RuntimeCommand, RuntimeManager};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InstallHostCapacity {
    disk_available_gb: u32,
    network_available: bool,
}

impl InstallHostCapacity {
    #[must_use]
    pub fn new(disk_available_gb: u32, network_available: bool) -> Self {
        Self {
            disk_available_gb,
            network_available,
        }
    }

    #[must_use]
    pub fn disk_available_gb(&self) -> u32 {
        self.disk_available_gb
    }

    #[must_use]
    pub fn network_available(&self) -> bool {
        self.network_available
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInstallRequest {
    plan: InstallPlan,
    setup_plan_accepted: bool,
}

impl RuntimeInstallRequest {
    #[must_use]
    pub fn new(plan: InstallPlan) -> Self {
        Self {
            plan,
            setup_plan_accepted: false,
        }
    }

    #[must_use]
    pub fn with_setup_plan_accepted(mut self, accepted: bool) -> Self {
        self.setup_plan_accepted = accepted;
        self
    }

    #[must_use]
    pub fn setup_plan_accepted(&self) -> bool {
        self.setup_plan_accepted
    }

    pub(crate) fn into_plan(self) -> InstallPlan {
        self.plan
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeInstallError {
    SetupPlanNotAccepted,
    MissingVerificationMetadata,
    UnknownSetupChoice,
    UnsafeInstallerSource,
    ExternallyManagedRuntime,
    InsufficientDisk { required_gb: u32, available_gb: u32 },
    NetworkUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInstallApproval {
    AutomaticAfterSetupAcceptance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInstallStatus {
    Queued,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInstallJob {
    plan: InstallPlan,
    status: RuntimeInstallStatus,
    approval: RuntimeInstallApproval,
}

impl RuntimeInstallJob {
    #[must_use]
    pub(crate) fn queued(plan: InstallPlan) -> Self {
        Self {
            plan,
            status: RuntimeInstallStatus::Queued,
            approval: RuntimeInstallApproval::AutomaticAfterSetupAcceptance,
        }
    }

    #[must_use]
    pub fn status(&self) -> RuntimeInstallStatus {
        self.status
    }

    #[must_use]
    pub fn approval(&self) -> RuntimeInstallApproval {
        self.approval
    }

    #[must_use]
    pub fn plan_preview(&self) -> String {
        self.plan.explanation()
    }

    pub fn complete_install(&self, manager: &mut RuntimeManager, version: impl Into<String>) {
        manager.apply(RuntimeCommand::mark_installed(
            self.plan.runtime_id().clone(),
            version,
        ));
    }
}
