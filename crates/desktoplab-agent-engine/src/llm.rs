#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlmExecutionAdapter {
    backend_id: String,
    provider: bool,
    provider_egress_allowed: bool,
    cancel_before_complete: bool,
    deterministic_output: Option<String>,
    deterministic_failure: Option<LlmExecutionError>,
}

impl LlmExecutionAdapter {
    #[must_use]
    pub fn local(backend_id: &str) -> Self {
        Self {
            backend_id: backend_id.to_string(),
            provider: false,
            provider_egress_allowed: true,
            cancel_before_complete: false,
            deterministic_output: None,
            deterministic_failure: None,
        }
    }

    #[must_use]
    pub fn provider(provider_id: &str) -> Self {
        Self {
            backend_id: provider_id.to_string(),
            provider: true,
            provider_egress_allowed: false,
            cancel_before_complete: false,
            deterministic_output: None,
            deterministic_failure: None,
        }
    }

    #[must_use]
    pub fn with_provider_egress_allowed(mut self, allowed: bool) -> Self {
        self.provider_egress_allowed = allowed;
        self
    }

    #[must_use]
    pub fn cancel_before_complete(mut self) -> Self {
        self.cancel_before_complete = true;
        self
    }

    #[must_use]
    pub fn with_deterministic_output(mut self, output: impl Into<String>) -> Self {
        if cfg!(debug_assertions) {
            self.deterministic_output = Some(output.into());
        }
        self
    }

    #[must_use]
    pub fn with_local_inference_failure(mut self) -> Self {
        self.deterministic_failure = Some(LlmExecutionError::LocalInferenceFailed);
        self
    }

    #[must_use]
    pub fn with_external_backend_unavailable(mut self) -> Self {
        self.deterministic_failure = Some(LlmExecutionError::ExternalBackendUnavailable);
        self
    }

    pub fn complete(&self, prompt: &str) -> Result<LlmExecutionStream, LlmExecutionError> {
        if self.provider && !self.provider_egress_allowed {
            return Err(LlmExecutionError::ProviderEgressDenied);
        }
        if let Some(error) = &self.deterministic_failure {
            return Err(error.clone());
        }
        let mut events = vec!["stream_started".to_string(), "delta".to_string()];
        if self.cancel_before_complete {
            events.push("cancelled".to_string());
        } else {
            events.push("stream_completed".to_string());
        }
        Ok(LlmExecutionStream {
            backend_id: self.backend_id.clone(),
            prompt_preview: prompt.chars().take(24).collect(),
            output: self
                .deterministic_output
                .clone()
                .ok_or(LlmExecutionError::LocalInferenceNotConfigured)?,
            events,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LlmExecutionStream {
    backend_id: String,
    prompt_preview: String,
    output: String,
    events: Vec<String>,
}

impl LlmExecutionStream {
    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    #[must_use]
    pub fn events(&self) -> Vec<&str> {
        self.events.iter().map(String::as_str).collect()
    }

    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LlmExecutionError {
    LocalInferenceNotConfigured,
    LocalInferenceFailed,
    ProviderEgressDenied,
    ExternalBackendUnavailable,
}
