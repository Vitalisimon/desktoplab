use desktoplab_agent_engine::{AgentLoop, AgentRunRequest};
use desktoplab_backend_services::{
    CatalogChannel, SessionService, SessionServiceStore, SetupCatalogEntry, SetupWizardApiService,
    SetupWizardPolicy, SetupWizardRegistryState,
};
use desktoplab_hardware_wizard::ProbeSnapshot;
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::ToolGateway;

use crate::SetupToFirstPromptOutcome;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SetupToFirstPromptMode {
    DryRun,
    LocalServices,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupToFirstPromptHarness {
    mode: SetupToFirstPromptMode,
    workspace_path: Option<String>,
    setup_accepted: bool,
    runtime_install_started: bool,
    model_download_started: bool,
    model_ready: bool,
    prompt: Option<String>,
}

impl SetupToFirstPromptHarness {
    #[must_use]
    pub fn new(mode: SetupToFirstPromptMode) -> Self {
        Self {
            mode,
            workspace_path: None,
            setup_accepted: false,
            runtime_install_started: false,
            model_download_started: false,
            model_ready: false,
            prompt: None,
        }
    }

    #[must_use]
    pub fn open_workspace(mut self, path: impl Into<String>) -> Self {
        self.workspace_path = Some(path.into());
        self
    }

    #[must_use]
    pub fn accept_setup(mut self) -> Self {
        self.setup_accepted = true;
        self
    }

    #[must_use]
    pub fn start_runtime_install(mut self) -> Self {
        self.runtime_install_started = true;
        self
    }

    #[must_use]
    pub fn start_model_download(mut self) -> Self {
        self.model_download_started = true;
        self
    }

    #[must_use]
    pub fn verify_model_ready(mut self) -> Self {
        self.model_ready = true;
        self
    }

    #[must_use]
    pub fn create_agent_session(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn run(self) -> Result<SetupToFirstPromptOutcome, &'static str> {
        let workspace_path = self.workspace_path.as_deref().unwrap_or_default();
        if workspace_path.is_empty() {
            return Err("workspace_not_open");
        }
        if desktoplab_workspace::GitRepository::open(std::path::Path::new(workspace_path)).is_err()
        {
            return Err("workspace_not_git_repository");
        }
        if !self.setup_accepted {
            return Err("setup_not_accepted");
        }
        if !self.runtime_install_started {
            return Err("runtime_install_not_started");
        }
        if !self.model_download_started {
            return Err("model_download_not_started");
        }
        if !self.model_ready && self.mode == SetupToFirstPromptMode::DryRun {
            return Err("model_not_ready");
        }
        if self.prompt.as_deref().unwrap_or_default().trim().is_empty() {
            return Err("prompt_missing");
        }
        match self.mode {
            SetupToFirstPromptMode::DryRun => Ok(SetupToFirstPromptOutcome::dry_run(workspace_id(
                workspace_path,
            ))),
            SetupToFirstPromptMode::LocalServices => Ok(self.run_local_services(workspace_path)),
        }
    }

    fn run_local_services(&self, workspace_path: &str) -> SetupToFirstPromptOutcome {
        let setup = SetupWizardApiService::new();
        let preview = setup.preview(
            host_snapshot(),
            SetupWizardRegistryState::Ready,
            SetupWizardPolicy::stable_only(),
            setup_catalog(),
        );
        let acceptance = setup.accept(preview);
        let mut sessions = SessionService::new(SessionServiceStore::default());
        let workspace_id = workspace_id(workspace_path);
        let session = sessions.create_session(&workspace_id, "backend.ollama");
        let (route_status, loop_event_observed, blocked_route_observed) =
            record_prompt_route(self, &mut sessions, session.session_id());
        SetupToFirstPromptOutcome {
            workspace_id,
            runtime_job_id: job_id(acceptance.started_job_ids(), "runtime.install:"),
            model_job_id: job_id(acceptance.started_job_ids(), "model.download:"),
            session_id: session.session_id().to_string(),
            evidence_label: "fixture-local-services",
            runtime_state: state_for(self.model_ready),
            model_state: state_for(self.model_ready),
            route_status,
            setup_preview_observed: true,
            loop_event_observed,
            blocked_route_observed,
            used_external_network: false,
            certifying: false,
        }
    }
}

fn record_prompt_route(
    harness: &SetupToFirstPromptHarness,
    sessions: &mut SessionService,
    session_id: &str,
) -> (&'static str, bool, bool) {
    if !harness.model_ready {
        sessions.block(session_id, "model download not ready");
        return ("blocked", false, true);
    }
    let mut agent_loop = AgentLoop::new(ToolGateway::new(PolicyEngine::default_conservative()));
    let run = agent_loop.run(
        AgentRunRequest::new(session_id, "backend.ollama")
            .with_prompt(harness.prompt.as_deref().unwrap_or_default())
            .with_backend_response("Local services accepted the first prompt."),
    );
    sessions.append_events(session_id, run.events());
    ("ready", true, false)
}

fn host_snapshot() -> ProbeSnapshot {
    ProbeSnapshot::new()
        .with_operating_system("macos")
        .with_architecture("aarch64")
        .with_cpu("apple-silicon")
        .with_ram_gb(32)
        .with_unified_memory_gb(32)
        .with_storage_available_gb(256)
}

fn setup_catalog() -> Vec<SetupCatalogEntry> {
    vec![
        SetupCatalogEntry::runtime("runtime.ollama", "Ollama", CatalogChannel::Stable),
        SetupCatalogEntry::model(
            "model.agent-candidate",
            "Agent candidate",
            CatalogChannel::Beta,
        ),
    ]
}

fn job_id(job_ids: &[String], prefix: &str) -> String {
    job_ids
        .iter()
        .find(|job| job.starts_with(prefix))
        .cloned()
        .unwrap_or_else(|| format!("{prefix}blocked"))
}

fn workspace_id(workspace_path: &str) -> String {
    let display_name = std::path::Path::new(workspace_path)
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("workspace");
    format!("workspace.{display_name}")
}

fn state_for(model_ready: bool) -> &'static str {
    if model_ready { "ready" } else { "blocked" }
}
