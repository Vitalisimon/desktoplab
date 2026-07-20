use crate::{
    InstallPlan, LmStudioLocalEndpointMetadata, LmStudioRuntime, OllamaBinaryVerification,
    OllamaRuntime, RuntimeId, RuntimeStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaInstallerAdapter {
    target: String,
}

impl OllamaInstallerAdapter {
    #[must_use]
    pub fn for_target(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }

    #[must_use]
    pub fn install_plan(&self) -> InstallPlan {
        OllamaRuntime::new().platform_install_plan(&self.target)
    }

    #[must_use]
    pub fn verify_installer(&self, expected: &str, actual: &str) -> RuntimeStatus {
        let verification = if expected == actual {
            OllamaBinaryVerification::passed()
        } else {
            OllamaBinaryVerification::failed("downloaded binary checksum mismatch")
        };
        OllamaRuntime::new().status_from_binary_verification(verification)
    }

    #[must_use]
    pub fn guided_setup_plan(&self) -> GuidedRuntimeSetupPlan {
        GuidedRuntimeSetupPlan::new(&self.target, "install Ollama manually")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GuidedRuntimeSetupPlan {
    target: String,
    step: String,
}

impl GuidedRuntimeSetupPlan {
    fn new(target: impl Into<String>, step: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            step: step.into(),
        }
    }

    #[must_use]
    pub fn can_install_automatically(&self) -> bool {
        false
    }

    #[must_use]
    pub fn explanation(&self) -> String {
        format!("target: {}\nstep: {}", self.target, self.step)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioProductionAdapter {
    endpoint: String,
}

impl LmStudioProductionAdapter {
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    #[must_use]
    pub fn can_be_stopped_by_desktoplab(&self) -> bool {
        false
    }

    #[must_use]
    pub fn endpoint_metadata(&self) -> LmStudioLocalEndpointMetadata {
        LmStudioRuntime::new().local_endpoint_metadata(&self.endpoint)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeRepairKind {
    VerifyDownload,
    GuidedInstall,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeRepairPlan {
    kind: RuntimeRepairKind,
    log_excerpt: String,
}

impl RuntimeRepairPlan {
    #[must_use]
    pub fn kind(&self) -> RuntimeRepairKind {
        self.kind
    }

    #[must_use]
    pub fn log_excerpt(&self) -> &str {
        &self.log_excerpt
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeRepairInventory {
    plans: Vec<(String, RuntimeRepairPlan)>,
}

impl RuntimeRepairInventory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_blocked_runtime(
        mut self,
        runtime_id: impl Into<String>,
        _name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        self.plans.push((
            runtime_id.into(),
            RuntimeRepairPlan {
                kind: RuntimeRepairKind::VerifyDownload,
                log_excerpt: redact(&reason.into()),
            },
        ));
        self
    }

    #[must_use]
    pub fn with_missing_runtime(
        mut self,
        runtime_id: impl Into<String>,
        _name: impl Into<String>,
    ) -> Self {
        self.plans.push((
            runtime_id.into(),
            RuntimeRepairPlan {
                kind: RuntimeRepairKind::GuidedInstall,
                log_excerpt: "runtime not installed".to_string(),
            },
        ));
        self
    }

    #[must_use]
    pub fn repair_plan(&self, runtime_id: &str) -> Option<&RuntimeRepairPlan> {
        self.plans
            .iter()
            .find(|(known_id, _)| known_id == runtime_id)
            .map(|(_, plan)| plan)
    }
}

fn redact(value: &str) -> String {
    if value.contains("sk-") {
        return "[REDACTED]".to_string();
    }
    value.to_string()
}

#[allow(dead_code)]
fn runtime_id(value: &str) -> RuntimeId {
    RuntimeId::new(value)
}
