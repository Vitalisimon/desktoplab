#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticServiceFamily {
    Storage,
    Registry,
    Runtime,
    Model,
    Provider,
    Plugin,
    WorkspaceScan,
    Session,
    Job,
}

impl DiagnosticServiceFamily {
    fn as_str(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Registry => "registry",
            Self::Runtime => "runtime",
            Self::Model => "model",
            Self::Provider => "provider",
            Self::Plugin => "plugin",
            Self::WorkspaceScan => "workspace_scan",
            Self::Session => "session",
            Self::Job => "job",
        }
    }
}

impl DiagnosticService {
    pub(crate) fn family(&self) -> DiagnosticServiceFamily {
        self.family
    }

    pub(crate) fn state(&self) -> &DiagnosticServiceState {
        &self.state
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiagnosticServiceState {
    Ready,
    Degraded(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDownloadDiagnosticFailure {
    InsufficientDisk,
    NetworkUnavailable,
    RuntimeUnavailable,
    VerificationFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagingDiagnostics {
    app_version: String,
    package_channel: String,
    artifact_target: String,
    update_state: String,
    evidence: String,
}

impl PackagingDiagnostics {
    #[must_use]
    pub fn new(
        app_version: impl Into<String>,
        package_channel: impl Into<String>,
        artifact_target: impl Into<String>,
        update_state: impl Into<String>,
        evidence: impl Into<String>,
    ) -> Self {
        Self {
            app_version: app_version.into(),
            package_channel: package_channel.into(),
            artifact_target: artifact_target.into(),
            update_state: update_state.into(),
            evidence: evidence.into(),
        }
    }
}

impl ModelDownloadDiagnosticFailure {
    fn reason(self) -> &'static str {
        match self {
            Self::InsufficientDisk => "insufficient disk for model download",
            Self::NetworkUnavailable => "network unavailable during model download",
            Self::RuntimeUnavailable => "local runtime unavailable for model download",
            Self::VerificationFailed => "model verification failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DiagnosticService {
    family: DiagnosticServiceFamily,
    state: DiagnosticServiceState,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackendDiagnosticsService {
    offline: bool,
    services: Vec<DiagnosticService>,
    notes: Vec<String>,
}

impl BackendDiagnosticsService {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn offline() -> Self {
        Self {
            offline: true,
            services: Vec::new(),
            notes: vec!["offline=true".to_string()],
        }
    }

    #[must_use]
    pub fn with_service(
        mut self,
        family: DiagnosticServiceFamily,
        state: DiagnosticServiceState,
    ) -> Self {
        self.services.push(DiagnosticService { family, state });
        self
    }

    #[must_use]
    pub fn with_model_download_failure(
        mut self,
        model_id: &str,
        failure: ModelDownloadDiagnosticFailure,
        evidence: &str,
    ) -> Self {
        self.services.push(DiagnosticService {
            family: DiagnosticServiceFamily::Model,
            state: DiagnosticServiceState::Degraded(format!("{} {}", model_id, failure.reason())),
        });
        self.notes
            .push(format!("model_download {} {}", model_id, evidence));
        self
    }

    #[must_use]
    pub fn with_packaging_diagnostics(mut self, diagnostics: PackagingDiagnostics) -> Self {
        self.notes.push(format!(
            "packaging app_version={} package_channel={} artifact_target={} update_state={} support_evidence=local_only evidence={}",
            diagnostics.app_version,
            diagnostics.package_channel,
            diagnostics.artifact_target,
            diagnostics.update_state,
            diagnostics.evidence
        ));
        self
    }

    #[must_use]
    pub fn with_setup_context(mut self, runtime_id: &str, model_id: &str, job_state: &str) -> Self {
        self.notes.push(format!(
            "setup runtime_id={runtime_id} model_id={model_id} job_state={job_state}"
        ));
        self
    }

    #[must_use]
    pub fn with_hardware_fact(mut self, label: &str, value: &str) -> Self {
        self.notes.push(format!("hardware {label}={value}"));
        self
    }

    #[must_use]
    pub fn snapshot(self) -> DiagnosticsSnapshot {
        DiagnosticsSnapshot {
            offline: self.offline,
            services: self.services,
            notes: self.notes,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticsSnapshot {
    offline: bool,
    services: Vec<DiagnosticService>,
    notes: Vec<String>,
}

impl DiagnosticsSnapshot {
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    #[must_use]
    pub fn has_family(&self, family: DiagnosticServiceFamily) -> bool {
        self.services.iter().any(|service| service.family == family)
    }

    #[must_use]
    pub fn degraded_reasons(&self) -> Vec<String> {
        self.services
            .iter()
            .filter_map(|service| match &service.state {
                DiagnosticServiceState::Ready => None,
                DiagnosticServiceState::Degraded(reason) => {
                    Some(format!("{}:{reason}", service.family.as_str()))
                }
            })
            .collect()
    }

    #[must_use]
    pub fn offline(&self) -> bool {
        self.offline
    }

    #[must_use]
    pub fn bundle(&self) -> String {
        self.notes
            .iter()
            .map(|note| redact(note))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn services(&self) -> &[DiagnosticService] {
        &self.services
    }
}

pub(crate) fn redact_diagnostic(value: &str) -> String {
    redact(value)
}

fn redact(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            if part.contains("token=")
                || part.contains("api_key=")
                || part.contains("signing_identity=")
                || part.contains("local_path=")
                || part.contains("workspace_path=")
            {
                let key = part.split_once('=').map_or(part, |(key, _)| key);
                format!("{key}=[REDACTED]")
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
