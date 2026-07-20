use crate::{HighEndRuntimeContract, RuntimeId, RuntimeLaunchSupport, RuntimeProcessSpec};
use serde_json::Value;
use std::net::{SocketAddr, ToSocketAddrs};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighEndRuntimeOwnership {
    UserOwned,
    DesktopLabOwned,
}

impl HighEndRuntimeOwnership {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UserOwned => "user_owned",
            Self::DesktopLabOwned => "desktoplab_owned",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HighEndRuntimeHealthState {
    Unconfigured,
    Reachable,
    ModelLoading,
    ModelReady,
    Degraded,
    Busy,
    Failed,
}

impl HighEndRuntimeHealthState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unconfigured => "unconfigured",
            Self::Reachable => "reachable",
            Self::ModelLoading => "model_loading",
            Self::ModelReady => "model_ready",
            Self::Degraded => "degraded",
            Self::Busy => "busy",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeEndpointSpec {
    base_url: String,
    host: String,
    port: u16,
    model_id: String,
    health_path: String,
}

impl RuntimeEndpointSpec {
    pub fn local(
        base_url: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Self, RuntimeEndpointError> {
        let base_url = base_url.into();
        let authority = base_url
            .strip_prefix("http://")
            .ok_or(RuntimeEndpointError::UnsupportedScheme)?
            .trim_end_matches('/');
        if authority.contains('/') {
            return Err(RuntimeEndpointError::InvalidEndpoint);
        }
        let (host, port) = authority
            .rsplit_once(':')
            .ok_or(RuntimeEndpointError::MissingPort)?;
        let port = port
            .parse::<u16>()
            .map_err(|_| RuntimeEndpointError::InvalidEndpoint)?;
        if !crate::high_end_http::is_local_host(host) {
            return Err(RuntimeEndpointError::NonLocalEndpoint);
        }
        Ok(Self {
            base_url: format!("http://{host}:{port}"),
            host: host.into(),
            port,
            model_id: model_id.into(),
            health_path: "/health".into(),
        })
    }

    #[must_use]
    pub fn with_health_path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        self.health_path = if path.starts_with('/') {
            path
        } else {
            format!("/{path}")
        };
        self
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    pub(crate) fn socket_addr(&self) -> Result<SocketAddr, RuntimeEndpointError> {
        (self.host.as_str(), self.port)
            .to_socket_addrs()
            .map_err(|_| RuntimeEndpointError::Unreachable)?
            .find(|address| crate::high_end_http::is_local_ip(address.ip()))
            .ok_or(RuntimeEndpointError::NonLocalEndpoint)
    }

    pub(crate) fn host(&self) -> &str {
        &self.host
    }

    pub(crate) fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeEndpointError {
    UnsupportedScheme,
    MissingPort,
    InvalidEndpoint,
    NonLocalEndpoint,
    Unreachable,
    InvalidResponse,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighEndRuntimeHealthEvidence {
    endpoint_reachable: bool,
    endpoint_compatible: bool,
    model_loaded: bool,
    tokenizer_ready: Option<bool>,
    gpu_memory_pressure_percent: Option<u8>,
    queue_depth: Option<u32>,
    reason: Option<String>,
}

impl HighEndRuntimeHealthEvidence {
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            endpoint_reachable: false,
            endpoint_compatible: false,
            model_loaded: false,
            tokenizer_ready: None,
            gpu_memory_pressure_percent: None,
            queue_depth: None,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn endpoint_reachable(&self) -> bool {
        self.endpoint_reachable
    }

    #[must_use]
    pub fn endpoint_compatible(&self) -> bool {
        self.endpoint_compatible
    }

    #[must_use]
    pub fn model_loaded(&self) -> bool {
        self.model_loaded
    }

    #[must_use]
    pub fn tokenizer_ready(&self) -> Option<bool> {
        self.tokenizer_ready
    }

    #[must_use]
    pub fn gpu_memory_pressure_percent(&self) -> Option<u8> {
        self.gpu_memory_pressure_percent
    }

    #[must_use]
    pub fn queue_depth(&self) -> Option<u32> {
        self.queue_depth
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    #[must_use]
    pub fn state(&self) -> HighEndRuntimeHealthState {
        if !self.endpoint_reachable {
            return HighEndRuntimeHealthState::Failed;
        }
        if !self.endpoint_compatible || self.tokenizer_ready == Some(false) {
            return HighEndRuntimeHealthState::Degraded;
        }
        if !self.model_loaded {
            return HighEndRuntimeHealthState::ModelLoading;
        }
        if self.tokenizer_ready != Some(true) {
            return HighEndRuntimeHealthState::Reachable;
        }
        if self
            .gpu_memory_pressure_percent
            .is_some_and(|value| value >= 95)
            || self.queue_depth.is_some_and(|value| value >= 32)
        {
            return HighEndRuntimeHealthState::Busy;
        }
        HighEndRuntimeHealthState::ModelReady
    }
}

pub trait RuntimeEndpointHealthProbe {
    fn probe(&self, endpoint: &RuntimeEndpointSpec) -> HighEndRuntimeHealthEvidence;
}

#[derive(Clone, Copy, Debug)]
pub struct HttpRuntimeEndpointProbe {
    timeout: std::time::Duration,
}

impl Default for HttpRuntimeEndpointProbe {
    fn default() -> Self {
        Self {
            timeout: std::time::Duration::from_secs(2),
        }
    }
}

impl HttpRuntimeEndpointProbe {
    pub fn discover_models(
        &self,
        endpoint: &RuntimeEndpointSpec,
    ) -> Result<Vec<String>, RuntimeEndpointError> {
        let value = crate::high_end_http::http_get_json(endpoint, "/v1/models", self.timeout)?;
        let entries = value
            .get("data")
            .and_then(Value::as_array)
            .ok_or(RuntimeEndpointError::InvalidResponse)?;
        let models = entries
            .iter()
            .filter_map(|entry| entry.get("id").and_then(Value::as_str))
            .filter(|model| !model.trim().is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        (!models.is_empty())
            .then_some(models)
            .ok_or(RuntimeEndpointError::InvalidResponse)
    }
}

impl RuntimeEndpointHealthProbe for HttpRuntimeEndpointProbe {
    fn probe(&self, endpoint: &RuntimeEndpointSpec) -> HighEndRuntimeHealthEvidence {
        let models = match crate::high_end_http::http_get_json(endpoint, "/v1/models", self.timeout)
        {
            Ok(value) => value,
            Err(error) => return HighEndRuntimeHealthEvidence::failed(format!("{error:?}")),
        };
        let Some(entries) = models.get("data").and_then(Value::as_array) else {
            return HighEndRuntimeHealthEvidence {
                endpoint_reachable: true,
                endpoint_compatible: false,
                model_loaded: false,
                tokenizer_ready: None,
                gpu_memory_pressure_percent: None,
                queue_depth: None,
                reason: Some("openai_compatible_model_list_missing".into()),
            };
        };
        let model_loaded = entries.iter().any(|entry| {
            entry.get("id").and_then(Value::as_str) == Some(endpoint.model_id.as_str())
        });
        let health =
            crate::high_end_http::http_get_json(endpoint, &endpoint.health_path, self.timeout).ok();
        HighEndRuntimeHealthEvidence {
            endpoint_reachable: true,
            endpoint_compatible: true,
            model_loaded,
            tokenizer_ready: health
                .as_ref()
                .and_then(|value| value.get("tokenizerReady"))
                .and_then(Value::as_bool),
            gpu_memory_pressure_percent: health
                .as_ref()
                .and_then(|value| value.get("gpuMemoryPressurePercent"))
                .and_then(Value::as_u64)
                .map(|value| value.min(100) as u8),
            queue_depth: health
                .as_ref()
                .and_then(|value| value.get("queueDepth"))
                .and_then(Value::as_u64)
                .map(|value| value.min(u32::MAX as u64) as u32),
            reason: health
                .is_none()
                .then(|| "runtime_health_metadata_unavailable".into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighEndRuntimeLifecycle {
    contract: HighEndRuntimeContract,
    endpoint: RuntimeEndpointSpec,
    ownership: HighEndRuntimeOwnership,
    evidence: HighEndRuntimeHealthEvidence,
}

impl HighEndRuntimeLifecycle {
    #[must_use]
    pub fn attached(
        contract: HighEndRuntimeContract,
        endpoint: RuntimeEndpointSpec,
        evidence: HighEndRuntimeHealthEvidence,
    ) -> Self {
        Self {
            contract,
            endpoint,
            ownership: HighEndRuntimeOwnership::UserOwned,
            evidence,
        }
    }

    pub fn desktoplab_owned_after_launch(
        contract: HighEndRuntimeContract,
        endpoint: RuntimeEndpointSpec,
        evidence: HighEndRuntimeHealthEvidence,
    ) -> Result<Self, HighEndRuntimeLifecycleError> {
        if contract.launch_support() == RuntimeLaunchSupport::AttachOnly {
            return Err(HighEndRuntimeLifecycleError::AttachOnly(
                contract.runtime_id().clone(),
            ));
        }
        Ok(Self {
            contract,
            endpoint,
            ownership: HighEndRuntimeOwnership::DesktopLabOwned,
            evidence,
        })
    }

    pub fn stop_owned(&mut self) -> Result<(), HighEndRuntimeLifecycleError> {
        if self.ownership == HighEndRuntimeOwnership::UserOwned {
            return Err(HighEndRuntimeLifecycleError::UserOwned(
                self.contract.runtime_id().clone(),
            ));
        }
        self.evidence = HighEndRuntimeHealthEvidence::failed("runtime_stopped");
        Ok(())
    }

    #[must_use]
    pub fn contract(&self) -> &HighEndRuntimeContract {
        &self.contract
    }

    #[must_use]
    pub fn endpoint(&self) -> &RuntimeEndpointSpec {
        &self.endpoint
    }

    #[must_use]
    pub fn ownership(&self) -> HighEndRuntimeOwnership {
        self.ownership
    }

    #[must_use]
    pub fn evidence(&self) -> &HighEndRuntimeHealthEvidence {
        &self.evidence
    }

    #[must_use]
    pub fn process_spec(&self) -> RuntimeProcessSpec {
        match self.ownership {
            HighEndRuntimeOwnership::UserOwned => RuntimeProcessSpec::external(
                self.contract.runtime_id().clone(),
                self.contract.family().as_str(),
            ),
            HighEndRuntimeOwnership::DesktopLabOwned => RuntimeProcessSpec::managed(
                self.contract.runtime_id().clone(),
                self.contract.family().as_str(),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HighEndRuntimeLifecycleError {
    UserOwned(RuntimeId),
    AttachOnly(RuntimeId),
}
