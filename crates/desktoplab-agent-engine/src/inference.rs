use crate::{
    OpenAiCompatibleEndpoint, OpenAiCompatibleEndpointError, OpenAiCompatibleEndpointPolicy,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInferenceAdapter {
    backend_id: String,
    runtime_id: String,
    model_id: String,
    endpoint: Option<String>,
}

impl LocalInferenceAdapter {
    #[must_use]
    pub fn ollama(
        backend_id: impl Into<String>,
        runtime_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Self {
        Self {
            backend_id: backend_id.into(),
            runtime_id: runtime_id.into(),
            model_id: model_id.into(),
            endpoint: None,
        }
    }

    #[must_use]
    pub fn with_openai_compatible_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn try_with_openai_compatible_endpoint(
        mut self,
        endpoint: impl AsRef<str>,
        policy: OpenAiCompatibleEndpointPolicy,
    ) -> Result<Self, OpenAiCompatibleEndpointError> {
        let endpoint = OpenAiCompatibleEndpoint::validate(endpoint.as_ref(), policy)?;
        self.endpoint = Some(endpoint.url().to_string());
        Ok(self)
    }

    pub fn complete(
        &self,
        prompt: &str,
        transport: &dyn LocalInferenceTransport,
    ) -> Result<LocalInferenceResult, LocalInferenceError> {
        let Some(endpoint) = &self.endpoint else {
            return Err(LocalInferenceError::NotConfigured);
        };
        let request = LocalInferenceRequest {
            endpoint: endpoint.clone(),
            model_id: self.model_id.clone(),
            prompt: prompt.to_string(),
        };
        let output = transport.post_chat(&request)?;
        Ok(LocalInferenceResult {
            output,
            evidence: LocalInferenceEvidence {
                backend_id: self.backend_id.clone(),
                runtime_id: self.runtime_id.clone(),
                model_id: self.model_id.clone(),
                endpoint: endpoint.clone(),
            },
        })
    }
}

pub trait LocalInferenceTransport {
    fn post_chat(&self, request: &LocalInferenceRequest) -> Result<String, LocalInferenceError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInferenceRequest {
    endpoint: String,
    model_id: String,
    prompt: String,
}

impl LocalInferenceRequest {
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInferenceResult {
    output: String,
    evidence: LocalInferenceEvidence,
}

impl LocalInferenceResult {
    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }

    #[must_use]
    pub fn evidence(&self) -> &LocalInferenceEvidence {
        &self.evidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalInferenceEvidence {
    backend_id: String,
    runtime_id: String,
    model_id: String,
    endpoint: String,
}

impl LocalInferenceEvidence {
    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalInferenceError {
    NotConfigured,
    TransportFailed(String),
}
