use crate::{InstallPlan, RuntimeHealth, RuntimeId, RuntimeStatus, VerificationResult};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioEndpointProbe {
    endpoint: String,
    available: bool,
    reason: Option<String>,
    models: Vec<String>,
}

impl LmStudioEndpointProbe {
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
pub struct LmStudioEndpointDetection {
    endpoint: String,
    available: bool,
    reason: Option<String>,
    models: Vec<String>,
}

impl LmStudioEndpointDetection {
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.available
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        !self.available
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn models(&self) -> &[String] {
        &self.models
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioGuidedSetupPlan {
    target: String,
    steps: Vec<String>,
}

impl LmStudioGuidedSetupPlan {
    #[must_use]
    pub fn mode(&self) -> &str {
        "guided"
    }

    #[must_use]
    pub fn can_install_automatically(&self) -> bool {
        false
    }

    #[must_use]
    pub fn explanation(&self) -> String {
        let mut parts = vec![format!("target: {}", self.target)];
        parts.extend(self.steps.iter().map(|step| format!("step: {step}")));
        parts.join("\n")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioLocalEndpointMetadata {
    url: String,
}

impl LmStudioLocalEndpointMetadata {
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

pub trait LmStudioHostAdapter {
    fn list_openai_compatible_models(&self, endpoint: &str) -> Vec<String>;
}

#[derive(Clone, Debug, Default)]
pub struct LmStudioRuntime;

impl LmStudioRuntime {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        static ID: std::sync::OnceLock<RuntimeId> = std::sync::OnceLock::new();
        ID.get_or_init(|| RuntimeId::new("runtime.lm-studio"))
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        "LM Studio"
    }

    #[must_use]
    pub fn capabilities(&self) -> Vec<String> {
        vec![
            "runtime.local".to_string(),
            "runtime.lifecycle.guided".to_string(),
            "models.inventory".to_string(),
            "api.openai-compatible.local".to_string(),
        ]
    }

    #[must_use]
    pub fn install_plan(&self, target: impl Into<String>) -> InstallPlan {
        InstallPlan::new(self.runtime_id().clone(), self.display_name())
            .with_requirement(format!("target: {}", target.into()))
            .with_step("download or open LM Studio installer on demand")
            .with_step("verify local server readiness")
            .with_step("discover local OpenAI-compatible endpoint")
    }

    #[must_use]
    pub fn detect_endpoint(&self, probe: LmStudioEndpointProbe) -> LmStudioEndpointDetection {
        LmStudioEndpointDetection {
            endpoint: probe.endpoint,
            available: probe.available,
            reason: probe.reason,
            models: probe.models,
        }
    }

    #[must_use]
    pub fn guided_setup_plan(&self, target: impl Into<String>) -> LmStudioGuidedSetupPlan {
        LmStudioGuidedSetupPlan {
            target: target.into(),
            steps: vec![
                "download or update LM Studio on demand".to_string(),
                "open LM Studio manually".to_string(),
                "start local server".to_string(),
                "verify OpenAI-compatible endpoint".to_string(),
            ],
        }
    }

    #[must_use]
    pub fn status_from_endpoint(&self, detection: LmStudioEndpointDetection) -> RuntimeStatus {
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
                .unwrap_or_else(|| "endpoint unavailable".to_string()),
        )
    }

    #[must_use]
    pub fn local_endpoint_metadata(&self, url: impl Into<String>) -> LmStudioLocalEndpointMetadata {
        LmStudioLocalEndpointMetadata { url: url.into() }
    }

    #[must_use]
    pub fn model_inventory(
        &self,
        endpoint: &str,
        adapter: &impl LmStudioHostAdapter,
    ) -> Vec<String> {
        adapter.list_openai_compatible_models(endpoint)
    }

    #[must_use]
    pub fn verify(&self, health: RuntimeHealth) -> VerificationResult {
        if health.is_healthy() {
            VerificationResult::passed()
        } else {
            VerificationResult::failed(
                health
                    .reason()
                    .unwrap_or("lm studio endpoint verification failed"),
            )
        }
    }
}
