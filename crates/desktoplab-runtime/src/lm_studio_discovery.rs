use serde_json::Value;
use std::net::IpAddr;

use crate::{
    HttpRuntimeEndpointProbe, ProcessCommand, ProcessRunner, RuntimeEndpointError,
    RuntimeEndpointSpec, SystemProcessRunner,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LmStudioDiscoveryState {
    Ready,
    CliMissing,
    ServerStopped,
    InvalidStatus,
    EndpointUnavailable,
}

impl LmStudioDiscoveryState {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::CliMissing => "cli_missing",
            Self::ServerStopped => "server_stopped",
            Self::InvalidStatus => "invalid_status",
            Self::EndpointUnavailable => "endpoint_unavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioExistingDiscovery {
    state: LmStudioDiscoveryState,
    endpoint: Option<String>,
    models: Vec<String>,
    version: Option<String>,
    daemon_managed: Option<bool>,
    remediation: String,
}

impl LmStudioExistingDiscovery {
    fn failed(state: LmStudioDiscoveryState, remediation: impl Into<String>) -> Self {
        Self {
            state,
            endpoint: None,
            models: Vec::new(),
            version: None,
            daemon_managed: None,
            remediation: remediation.into(),
        }
    }

    #[must_use]
    pub fn ready(&self) -> bool {
        self.state == LmStudioDiscoveryState::Ready
    }

    #[must_use]
    pub fn state(&self) -> LmStudioDiscoveryState {
        self.state
    }

    #[must_use]
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }

    #[must_use]
    pub fn models(&self) -> &[String] {
        &self.models
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn daemon_managed(&self) -> Option<bool> {
        self.daemon_managed
    }

    #[must_use]
    pub fn ownership(&self) -> &'static str {
        if self.state == LmStudioDiscoveryState::CliMissing {
            "none"
        } else {
            "user_owned"
        }
    }

    #[must_use]
    pub fn remediation(&self) -> &str {
        &self.remediation
    }

    #[must_use]
    pub fn evidence(&self) -> String {
        format!(
            "lm-studio discovery state={}; endpoint={}; models={}; ownership={}",
            self.state.as_str(),
            self.endpoint.as_deref().unwrap_or("unavailable"),
            self.models.len(),
            self.ownership()
        )
    }
}

pub trait LmStudioModelProbe {
    fn models(&self, endpoint: &str) -> Result<Vec<String>, RuntimeEndpointError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HttpLmStudioModelProbe;

impl LmStudioModelProbe for HttpLmStudioModelProbe {
    fn models(&self, endpoint: &str) -> Result<Vec<String>, RuntimeEndpointError> {
        let authority = endpoint
            .strip_prefix("http://")
            .and_then(|value| value.rsplit_once(':'))
            .ok_or(RuntimeEndpointError::InvalidEndpoint)?;
        if authority.0 != "localhost"
            && !authority
                .0
                .parse::<IpAddr>()
                .is_ok_and(|address| address.is_loopback())
        {
            return Err(RuntimeEndpointError::NonLocalEndpoint);
        }
        let endpoint = RuntimeEndpointSpec::local(endpoint, "discovery")?;
        HttpRuntimeEndpointProbe::default().discover_models(&endpoint)
    }
}

#[must_use]
pub fn discover_existing_lm_studio(
    runner: &impl ProcessRunner,
    model_probe: &impl LmStudioModelProbe,
) -> LmStudioExistingDiscovery {
    let server = runner.run(
        ProcessCommand::new("lms")
            .arg("server")
            .arg("status")
            .arg("--json")
            .arg("--quiet"),
    );
    if server.exit_code().is_none() {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::CliMissing,
            "Install LM Studio or llmster, then make the official lms CLI available.",
        );
    }
    if !server.succeeded() {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::ServerStopped,
            "Start the LM Studio local server before connecting it to DesktopLab.",
        );
    }
    let Ok(status) = serde_json::from_str::<Value>(server.stdout()) else {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::InvalidStatus,
            "Update LM Studio or llmster so lms server status supports JSON output.",
        );
    };
    if status.get("running").and_then(Value::as_bool) != Some(true) {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::ServerStopped,
            "Start the LM Studio local server before connecting it to DesktopLab.",
        );
    }
    let Some(port) = status
        .get("port")
        .and_then(Value::as_u64)
        .filter(|port| (1..=u16::MAX as u64).contains(port))
    else {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::InvalidStatus,
            "LM Studio reported an invalid local server port.",
        );
    };
    let endpoint = format!("http://127.0.0.1:{port}");
    let Ok(models) = model_probe.models(&endpoint) else {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::EndpointUnavailable,
            "Keep the LM Studio server on loopback and load at least one model.",
        );
    };
    if models.is_empty() {
        return LmStudioExistingDiscovery::failed(
            LmStudioDiscoveryState::EndpointUnavailable,
            "Load at least one model in LM Studio before connecting it to DesktopLab.",
        );
    }
    let daemon = runner.run(
        ProcessCommand::new("lms")
            .arg("daemon")
            .arg("status")
            .arg("--json"),
    );
    let daemon_status = daemon
        .succeeded()
        .then(|| serde_json::from_str::<Value>(daemon.stdout()).ok())
        .flatten();
    LmStudioExistingDiscovery {
        state: LmStudioDiscoveryState::Ready,
        endpoint: Some(endpoint),
        models,
        version: daemon_status
            .as_ref()
            .and_then(|value| value.get("version"))
            .and_then(Value::as_str)
            .map(ToString::to_string),
        daemon_managed: daemon_status
            .as_ref()
            .and_then(|value| value.get("isDaemon"))
            .and_then(Value::as_bool),
        remediation: String::new(),
    }
}

#[must_use]
pub fn discover_system_lm_studio() -> LmStudioExistingDiscovery {
    discover_existing_lm_studio(&SystemProcessRunner, &HttpLmStudioModelProbe)
}
