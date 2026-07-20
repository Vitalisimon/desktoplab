use crate::{
    InstallHostCapacity, InstallerSource, RuntimeCachedInstallerArtifact, RuntimeId,
    RuntimeInstallError, RuntimeInstallExecutionStrategy, RuntimeInstallJob,
    RuntimeInstallManagement, RuntimeInstallRequest,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstallPlan {
    runtime_id: RuntimeId,
    runtime_name: String,
    steps: Vec<String>,
    requirements: Vec<String>,
    disk_requirement_gb: Option<u32>,
    network_required: bool,
    installer_source: Option<InstallerSource>,
    verification_steps: Vec<String>,
    target_platform: Option<String>,
    execution_strategy: Option<RuntimeInstallExecutionStrategy>,
    management: RuntimeInstallManagement,
    bundled: bool,
    cached_artifact: Option<RuntimeCachedInstallerArtifact>,
}

impl InstallPlan {
    #[must_use]
    pub fn new(runtime_id: RuntimeId, runtime_name: impl Into<String>) -> Self {
        Self {
            runtime_id,
            runtime_name: runtime_name.into(),
            steps: Vec::new(),
            requirements: Vec::new(),
            disk_requirement_gb: None,
            network_required: false,
            installer_source: None,
            verification_steps: Vec::new(),
            target_platform: None,
            execution_strategy: None,
            management: RuntimeInstallManagement::DesktopLabManaged,
            bundled: false,
            cached_artifact: None,
        }
    }

    #[must_use]
    pub fn with_step(mut self, step: impl Into<String>) -> Self {
        self.steps.push(step.into());
        self
    }

    #[must_use]
    pub fn with_requirement(mut self, requirement: impl Into<String>) -> Self {
        self.requirements.push(requirement.into());
        self
    }

    #[must_use]
    pub fn with_disk_requirement_gb(mut self, required_gb: u32) -> Self {
        self.disk_requirement_gb = Some(required_gb);
        self.requirements
            .push(format!("disk.available_gb >= {required_gb}"));
        self
    }

    #[must_use]
    pub fn with_network_required(mut self, required: bool) -> Self {
        self.network_required = required;
        if required {
            self.requirements.push("network".to_string());
        }
        self
    }

    #[must_use]
    pub fn with_installer_source(mut self, source: InstallerSource) -> Self {
        self.installer_source = Some(source);
        self
    }

    #[must_use]
    pub fn with_verification_step(mut self, step: impl Into<String>) -> Self {
        self.verification_steps.push(step.into());
        self
    }

    #[must_use]
    pub fn with_target_platform(mut self, platform: impl Into<String>) -> Self {
        self.target_platform = Some(platform.into());
        self
    }

    #[must_use]
    pub fn with_execution_strategy(mut self, strategy: RuntimeInstallExecutionStrategy) -> Self {
        self.execution_strategy = Some(strategy);
        self
    }

    #[must_use]
    pub fn with_management(mut self, management: RuntimeInstallManagement) -> Self {
        self.management = management;
        self
    }

    #[must_use]
    pub fn with_cached_artifact(mut self, artifact: RuntimeCachedInstallerArtifact) -> Self {
        self.cached_artifact = Some(artifact);
        self
    }

    #[must_use]
    pub fn runtime_name(&self) -> &str {
        &self.runtime_name
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    #[must_use]
    pub fn is_bundled(&self) -> bool {
        self.bundled
    }

    #[must_use]
    pub fn disk_requirement_gb(&self) -> Option<u32> {
        self.disk_requirement_gb
    }

    #[must_use]
    pub fn installer_source(&self) -> Option<&InstallerSource> {
        self.installer_source.as_ref()
    }

    #[must_use]
    pub fn target_platform(&self) -> Option<&str> {
        self.target_platform.as_deref()
    }

    #[must_use]
    pub fn execution_strategy(&self) -> Option<RuntimeInstallExecutionStrategy> {
        self.execution_strategy
    }

    #[must_use]
    pub fn explanation(&self) -> String {
        let mut parts = Vec::new();
        parts.push(format!("runtime: {}", self.runtime_name));
        parts.push(format!("runtime_id: {}", self.runtime_id.as_str()));
        parts.extend(self.steps.iter().map(|step| format!("step: {step}")));
        parts.extend(
            self.requirements
                .iter()
                .map(|requirement| format!("requires: {requirement}")),
        );
        if let Some(source) = &self.installer_source {
            parts.push(format!("installer_source: {}", source.url()));
            if let Some(signature) = source.signature() {
                parts.push(format!("installer_signature: {signature}"));
            }
        }
        if let Some(platform) = &self.target_platform {
            parts.push(format!("platform: {platform}"));
        }
        if let Some(strategy) = self.execution_strategy {
            parts.push(format!("execution_strategy: {}", strategy.as_str()));
        }
        if self.has_verified_cached_artifact() {
            parts.push("cached_installer: verified".to_string());
        }
        parts.extend(
            self.verification_steps
                .iter()
                .map(|step| format!("verify: {step}")),
        );
        parts.join("\n")
    }

    pub(crate) fn has_verification_metadata(&self) -> bool {
        self.installer_source
            .as_ref()
            .is_some_and(InstallerSource::has_verification_metadata)
            && !self.verification_steps.is_empty()
    }

    pub(crate) fn has_verified_cached_artifact(&self) -> bool {
        self.cached_artifact
            .as_ref()
            .is_some_and(RuntimeCachedInstallerArtifact::is_verified)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInstallPlanner {
    capacity: InstallHostCapacity,
}

impl RuntimeInstallPlanner {
    #[must_use]
    pub fn new(capacity: InstallHostCapacity) -> Self {
        Self { capacity }
    }

    pub fn plan_job(
        &self,
        request: RuntimeInstallRequest,
    ) -> Result<RuntimeInstallJob, RuntimeInstallError> {
        if !request.setup_plan_accepted() {
            return Err(RuntimeInstallError::SetupPlanNotAccepted);
        }

        let plan = request.into_plan();
        if plan.management == RuntimeInstallManagement::ExternallyManaged {
            return Err(RuntimeInstallError::ExternallyManagedRuntime);
        }

        if !plan.has_verification_metadata() {
            return Err(RuntimeInstallError::MissingVerificationMetadata);
        }

        if plan
            .installer_source
            .as_ref()
            .is_some_and(|source| !source.is_trusted_remote())
        {
            return Err(RuntimeInstallError::UnsafeInstallerSource);
        }

        if let Some(required_gb) = plan.disk_requirement_gb {
            if self.capacity.disk_available_gb() < required_gb {
                return Err(RuntimeInstallError::InsufficientDisk {
                    required_gb,
                    available_gb: self.capacity.disk_available_gb(),
                });
            }
        }

        if plan.network_required
            && !self.capacity.network_available()
            && !plan.has_verified_cached_artifact()
        {
            return Err(RuntimeInstallError::NetworkUnavailable);
        }

        Ok(RuntimeInstallJob::queued(plan))
    }
}
