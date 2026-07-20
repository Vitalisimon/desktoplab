use crate::{
    InstallPlan, InstallerSource, ProcessCommand, RuntimeHealth, RuntimeId,
    RuntimeInstallExecutionStrategy, RuntimeProcessSpec, RuntimeStatus, VerificationResult,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MlxLmInstallPlanError {
    UnsupportedPlatform { target: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmModelRefError {
    model_ref: String,
}

impl MlxLmModelRefError {
    #[must_use]
    pub fn model_ref(&self) -> &str {
        &self.model_ref
    }
}

impl MlxLmInstallPlanError {
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::UnsupportedPlatform { target } => {
                format!("MLX-LM Server requires an Apple Silicon Mac. Current target: {target}.")
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmEndpointProbe {
    endpoint: String,
    available: bool,
    reason: Option<String>,
    models: Vec<String>,
}

impl MlxLmEndpointProbe {
    #[must_use]
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            available: true,
            reason: None,
            models: Vec::new(),
        }
    }

    #[must_use]
    pub fn mark_unavailable(mut self, reason: impl Into<String>) -> Self {
        self.available = false;
        self.reason = Some(reason.into());
        self
    }

    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.models.push(model.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmEndpointDetection {
    endpoint: String,
    available: bool,
    reason: Option<String>,
    models: Vec<String>,
}

impl MlxLmEndpointDetection {
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    #[must_use]
    pub fn models(&self) -> &[String] {
        &self.models
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmLocalEndpointMetadata {
    url: String,
}

impl MlxLmLocalEndpointMetadata {
    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn is_openai_compatible(&self) -> bool {
        true
    }

    #[must_use]
    pub fn requires_provider_credential(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Default)]
pub struct MlxLmRuntime;

impl MlxLmRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        static ID: std::sync::OnceLock<RuntimeId> = std::sync::OnceLock::new();
        ID.get_or_init(|| RuntimeId::new("runtime.mlx-lm"))
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        "MLX-LM Server"
    }

    #[must_use]
    pub fn capabilities(&self) -> Vec<String> {
        vec![
            "runtime.local".to_string(),
            "runtime.apple-silicon".to_string(),
            "runtime.lifecycle.python".to_string(),
            "models.inventory".to_string(),
            "api.openai-compatible.local".to_string(),
        ]
    }

    pub fn try_install_plan(
        &self,
        target: impl Into<String>,
    ) -> Result<InstallPlan, MlxLmInstallPlanError> {
        let target = target.into();
        if !matches!(target.as_str(), "macos-aarch64" | "darwin-arm64") {
            return Err(MlxLmInstallPlanError::UnsupportedPlatform { target });
        }

        Ok(
            InstallPlan::new(self.runtime_id().clone(), self.display_name())
                .with_target_platform("darwin-arm64")
                .with_execution_strategy(RuntimeInstallExecutionStrategy::PythonEnvironment)
                .with_requirement("Apple Silicon")
                .with_requirement("Python environment")
                .with_network_required(true)
                .with_installer_source(InstallerSource::signed_url(
                    "https://pypi.org/project/mlx-lm/",
                    "pypi-release-metadata",
                ))
                .with_step("create isolated DesktopLab Python environment")
                .with_step("pip install mlx-lm")
                .with_step("start mlx_lm.server for the selected model")
                .with_step("verify local OpenAI-compatible endpoint")
                .with_verification_step("/v1/models responds locally"),
        )
    }

    pub fn validate_model_ref(
        &self,
        model_ref: impl Into<String>,
    ) -> Result<String, MlxLmModelRefError> {
        let model_ref = model_ref.into();
        let valid = !model_ref.trim().is_empty()
            && model_ref.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '/')
            })
            && !model_ref.contains("..")
            && !model_ref.starts_with('/')
            && !model_ref.ends_with('/');
        if valid {
            Ok(model_ref)
        } else {
            Err(MlxLmModelRefError { model_ref })
        }
    }

    #[must_use]
    pub fn download_command(&self, model_ref: impl Into<String>) -> Option<ProcessCommand> {
        let model_ref = self.validate_model_ref(model_ref).ok()?;
        Some(
            ProcessCommand::new("mlx_lm.generate")
                .arg("--model")
                .arg(model_ref)
                .arg("--prompt")
                .arg("DesktopLab model readiness check.")
                .arg("--max-tokens")
                .arg("1"),
        )
    }

    #[must_use]
    pub fn detect_endpoint(&self, probe: MlxLmEndpointProbe) -> MlxLmEndpointDetection {
        MlxLmEndpointDetection {
            endpoint: probe.endpoint,
            available: probe.available,
            reason: probe.reason,
            models: probe.models,
        }
    }

    #[must_use]
    pub fn local_endpoint_metadata(&self, url: impl Into<String>) -> MlxLmLocalEndpointMetadata {
        MlxLmLocalEndpointMetadata { url: url.into() }
    }

    #[must_use]
    pub fn start_command(&self, model_ref: impl Into<String>) -> ProcessCommand {
        ProcessCommand::new("mlx_lm.server")
            .arg("--model")
            .arg(model_ref.into())
    }

    #[must_use]
    pub fn process_spec(&self) -> RuntimeProcessSpec {
        RuntimeProcessSpec::managed(self.runtime_id().clone(), self.display_name())
    }

    #[must_use]
    pub fn status_from_endpoint(&self, detection: MlxLmEndpointDetection) -> RuntimeStatus {
        if detection.is_available() {
            let mut status =
                RuntimeStatus::installed(self.runtime_id().clone(), self.display_name(), "unknown");
            status.apply_verification(VerificationResult::passed());
            return status;
        }

        RuntimeStatus::degraded(
            self.runtime_id().clone(),
            self.display_name(),
            detection
                .reason
                .unwrap_or_else(|| "MLX-LM endpoint unavailable".to_string()),
        )
    }

    #[must_use]
    pub fn verify(&self, health: RuntimeHealth) -> VerificationResult {
        if health.is_healthy() {
            VerificationResult::passed()
        } else {
            VerificationResult::failed(
                health
                    .reason()
                    .unwrap_or("MLX-LM endpoint verification failed"),
            )
        }
    }
}
