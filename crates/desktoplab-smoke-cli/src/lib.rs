#![forbid(unsafe_code)]

use desktoplab_backend_services::{
    BackendDiagnosticsService, DiagnosticServiceFamily, DiagnosticServiceState, SessionService,
    SessionServiceStore,
};
use desktoplab_control_plane::{ControlPlane, LocalApiRouter, ReadinessState, VersionInfo};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SmokeCommand {
    Health,
    Status,
    Version,
    Readiness,
    WorkspaceOpen(String),
    SetupPreview,
    SessionStart(String),
    ApprovalResolve(String),
    Diagnostics,
    DoctorLint,
    DiagnosticsExport,
    RuntimeInspect,
    SecurityAudit,
    MigrationStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SmokeOutputFormat {
    Json,
    Plain,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SmokeOutput {
    kind: String,
    body: String,
}

impl SmokeOutput {
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }

    #[must_use]
    pub fn is_json(&self) -> bool {
        self.body.starts_with('{') && self.body.ends_with('}')
    }
}

#[derive(Clone, Debug)]
pub struct SmokeCli<A> {
    api: A,
}

impl<A> SmokeCli<A>
where
    A: SmokeApi,
{
    #[must_use]
    pub fn new(api: A) -> Self {
        Self { api }
    }

    pub fn run(&mut self, command: SmokeCommand) -> SmokeOutput {
        self.run_with_format(command, SmokeOutputFormat::Json)
    }

    pub fn run_with_format(
        &mut self,
        command: SmokeCommand,
        format: SmokeOutputFormat,
    ) -> SmokeOutput {
        let output = self.api.handle(command);
        match format {
            SmokeOutputFormat::Json => output,
            SmokeOutputFormat::Plain => output.as_plain(),
        }
    }
}

pub trait SmokeApi {
    fn handle(&mut self, command: SmokeCommand) -> SmokeOutput;
}

pub struct InProcessSmokeApi {
    control_plane: ControlPlane,
    local_api: LocalApiRouter,
    sessions: SessionService,
}

impl Default for InProcessSmokeApi {
    fn default() -> Self {
        Self {
            control_plane: ControlPlane::new(VersionInfo::new("0.1.0", "v1")),
            local_api: LocalApiRouter::default(),
            sessions: SessionService::new(SessionServiceStore::default()),
        }
    }
}

impl SmokeApi for InProcessSmokeApi {
    fn handle(&mut self, command: SmokeCommand) -> SmokeOutput {
        match command {
            SmokeCommand::Health => output("health", r#"{"status":"healthy"}"#),
            SmokeCommand::Status => {
                let state = readiness_state(self.control_plane.readiness().state());
                output(
                    "status",
                    format!(r#"{{"health":"healthy","readiness":"{state}"}}"#),
                )
            }
            SmokeCommand::Version => output(
                "version",
                r#"{"product_version":"0.1.0","api_version":"v1"}"#,
            ),
            SmokeCommand::Readiness => {
                let state = readiness_state(self.control_plane.readiness().state());
                output("readiness", format!(r#"{{"state":"{state}"}}"#))
            }
            SmokeCommand::WorkspaceOpen(workspace_id) => output(
                "workspace.open",
                format!(r#"{{"workspace_id":"{workspace_id}","opened":true}}"#),
            ),
            SmokeCommand::SetupPreview => output(
                "setup.preview",
                r#"{"runtimes":[],"models":[],"requires_frontend":false}"#,
            ),
            SmokeCommand::SessionStart(workspace_id) => {
                let session = self.sessions.create_session(workspace_id, "backend.local");
                output(
                    "session.start",
                    format!(r#"{{"session_id":"{}"}}"#, session.session_id()),
                )
            }
            SmokeCommand::ApprovalResolve(approval_id) => output(
                "approval.resolve",
                format!(r#"{{"approval_id":"{approval_id}","resolved":true}}"#),
            ),
            SmokeCommand::Diagnostics => {
                let snapshot = BackendDiagnosticsService::offline()
                    .with_service(
                        DiagnosticServiceFamily::Storage,
                        DiagnosticServiceState::Ready,
                    )
                    .snapshot();
                output(
                    "diagnostics",
                    format!(r#"{{"bundle":"{}"}}"#, snapshot.bundle()),
                )
            }
            SmokeCommand::DoctorLint => {
                self.local_route("doctor.lint", "/v1/diagnostics/doctor/lint")
            }
            SmokeCommand::DiagnosticsExport => {
                self.local_route("diagnostics.export", "/v1/diagnostics/export")
            }
            SmokeCommand::RuntimeInspect => {
                self.local_route("runtime.inspect", "/v1/runtime/inspect")
            }
            SmokeCommand::SecurityAudit => self.local_route("security.audit", "/v1/security/audit"),
            SmokeCommand::MigrationStatus => {
                self.local_route("migration.status", "/v1/diagnostics/migrations")
            }
        }
    }
}

impl SmokeOutput {
    fn as_plain(&self) -> Self {
        SmokeOutput {
            kind: self.kind.clone(),
            body: format!("{}: {}", self.kind, plain_summary(&self.body)),
        }
    }
}

impl InProcessSmokeApi {
    fn local_route(&mut self, kind: &str, path: &str) -> SmokeOutput {
        let response = self
            .local_api
            .route("GET", path, "")
            .unwrap_or_else(|| panic!("{path} route should exist"));
        output(kind, response.body().to_string())
    }
}

fn output(kind: impl Into<String>, body: impl Into<String>) -> SmokeOutput {
    SmokeOutput {
        kind: kind.into(),
        body: body.into(),
    }
}

fn readiness_state(state: ReadinessState) -> &'static str {
    match state {
        ReadinessState::Starting => "starting",
        ReadinessState::Ready => "ready",
    }
}

fn plain_summary(body: &str) -> String {
    body.chars().take(160).collect()
}
