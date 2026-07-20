use crate::{
    InstallPlan, InstallerSource, OllamaBinaryVerification, RuntimeDetection, RuntimeHealth,
    RuntimeId, RuntimeInstallExecutionStrategy, RuntimeProbe, RuntimeStatus, VerificationResult,
};

pub trait OllamaHostAdapter {
    fn list_models(&self) -> Vec<String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OllamaInstallPlanError {
    UnsupportedPlatform { target: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaModelPullRefError {
    pull_ref: String,
}

impl OllamaModelPullRefError {
    #[must_use]
    pub fn pull_ref(&self) -> &str {
        &self.pull_ref
    }
}

impl OllamaInstallPlanError {
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::UnsupportedPlatform { target } => {
                format!("Ollama installer is not supported on {target}.")
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct OllamaRuntime;

impl OllamaRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        static ID: std::sync::OnceLock<RuntimeId> = std::sync::OnceLock::new();
        ID.get_or_init(|| RuntimeId::new("runtime.ollama"))
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        "Ollama"
    }

    #[must_use]
    pub fn capabilities(&self) -> Vec<String> {
        vec![
            "runtime.local".to_string(),
            "runtime.lifecycle".to_string(),
            "models.inventory".to_string(),
            "models.download".to_string(),
        ]
    }

    #[must_use]
    pub fn install_plan(&self, target: impl Into<String>) -> InstallPlan {
        InstallPlan::new(self.runtime_id().clone(), self.display_name())
            .with_requirement(format!("target: {}", target.into()))
            .with_requirement("network")
            .with_step("download Ollama installer on demand")
            .with_step("verify downloaded binary")
            .with_step("install runtime")
            .with_step("verify runtime health")
    }

    #[must_use]
    pub fn platform_install_plan(&self, target: impl Into<String>) -> InstallPlan {
        let target = target.into();
        self.try_platform_install_plan(&target)
            .unwrap_or_else(|_| self.install_plan(target))
    }

    pub fn try_platform_install_plan(
        &self,
        target: impl Into<String>,
    ) -> Result<InstallPlan, OllamaInstallPlanError> {
        let target = target.into();
        let (platform, step, source, installer_verification) =
            ollama_platform_install_step(&target).ok_or_else(|| {
                OllamaInstallPlanError::UnsupportedPlatform {
                    target: target.clone(),
                }
            })?;

        Ok(self
            .install_plan(&target)
            .with_target_platform(platform)
            .with_execution_strategy(RuntimeInstallExecutionStrategy::NativeInstaller)
            .with_disk_requirement_gb(4)
            .with_network_required(true)
            .with_installer_source(source)
            .with_step(step)
            .with_verification_step(installer_verification)
            .with_verification_step("runtime health endpoint"))
    }

    pub fn validate_model_pull_ref(
        &self,
        pull_ref: impl Into<String>,
    ) -> Result<String, OllamaModelPullRefError> {
        let pull_ref = pull_ref.into();
        let valid = !pull_ref.trim().is_empty()
            && pull_ref.chars().all(|character| {
                character.is_ascii_alphanumeric()
                    || matches!(character, '.' | '_' | '-' | ':' | '/')
            })
            && !pull_ref.contains("..")
            && !pull_ref.starts_with('/')
            && !pull_ref.ends_with('/');
        if valid {
            Ok(pull_ref)
        } else {
            Err(OllamaModelPullRefError { pull_ref })
        }
    }

    #[must_use]
    pub fn detect(&self, probe: RuntimeProbe) -> RuntimeDetection {
        RuntimeDetection {
            installed: probe.binary_path.is_some(),
            version: probe.version,
            models: probe.models,
        }
    }

    #[must_use]
    pub fn verify(&self, health: RuntimeHealth) -> VerificationResult {
        if health.is_healthy() {
            VerificationResult::passed()
        } else {
            VerificationResult::failed(health.reason().unwrap_or("ollama health check failed"))
        }
    }

    #[must_use]
    pub fn status_from_binary_verification(
        &self,
        verification: OllamaBinaryVerification,
    ) -> RuntimeStatus {
        let mut status =
            RuntimeStatus::installed(self.runtime_id().clone(), self.display_name(), "unknown");
        if verification.passed {
            status.apply_verification(VerificationResult::passed());
        } else {
            status.apply_verification(VerificationResult::failed(
                verification
                    .reason
                    .unwrap_or_else(|| "ollama binary verification failed".to_string()),
            ));
        }
        status
    }

    #[must_use]
    pub fn status_from_health(&self, health: RuntimeHealth) -> RuntimeStatus {
        if health.is_healthy() {
            let mut status =
                RuntimeStatus::installed(self.runtime_id().clone(), self.display_name(), "unknown");
            status.apply_verification(VerificationResult::passed());
            return status;
        }

        RuntimeStatus::degraded(
            self.runtime_id().clone(),
            self.display_name(),
            health.reason().unwrap_or("ollama API unavailable"),
        )
    }

    #[must_use]
    pub fn model_inventory(&self, adapter: &impl OllamaHostAdapter) -> Vec<String> {
        adapter.list_models()
    }
}

fn ollama_platform_install_step(
    target: &str,
) -> Option<(&'static str, &'static str, InstallerSource, &'static str)> {
    match target {
        "darwin-arm64" | "macos-aarch64" => Some((
            "darwin-arm64",
            "open Ollama .dmg installer",
            InstallerSource::signed_url_with_signature(
                "https://ollama.com/download/Ollama.dmg",
                "vendor-signed",
                "ollama-vendor-signature",
            ),
            "signed Ollama installer",
        )),
        "windows-x64" => Some((
            "windows-x64",
            "run OllamaSetup.exe",
            InstallerSource::signed_url_with_signature(
                "https://ollama.com/download/OllamaSetup.exe",
                "vendor-signed",
                "ollama-vendor-signature",
            ),
            "signed Ollama installer",
        )),
        "linux-x64" => Some((
            "linux-x64",
            "run verified install.sh",
            InstallerSource::signed_url("https://ollama.com/install.sh", ""),
            "trusted sha256 digest required before automatic install",
        )),
        _ => None,
    }
}
