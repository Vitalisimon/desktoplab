#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupToFirstPromptOutcome {
    pub(crate) workspace_id: String,
    pub(crate) runtime_job_id: String,
    pub(crate) model_job_id: String,
    pub(crate) session_id: String,
    pub(crate) evidence_label: &'static str,
    pub(crate) runtime_state: &'static str,
    pub(crate) model_state: &'static str,
    pub(crate) route_status: &'static str,
    pub(crate) setup_preview_observed: bool,
    pub(crate) loop_event_observed: bool,
    pub(crate) blocked_route_observed: bool,
    pub(crate) used_external_network: bool,
    pub(crate) certifying: bool,
}

impl SetupToFirstPromptOutcome {
    pub(crate) fn dry_run(workspace_id: String) -> Self {
        Self {
            workspace_id,
            runtime_job_id: String::new(),
            model_job_id: String::new(),
            session_id: String::new(),
            evidence_label: "fixture-dry-run",
            runtime_state: "not_run",
            model_state: "not_run",
            route_status: "not_run",
            setup_preview_observed: false,
            loop_event_observed: false,
            blocked_route_observed: false,
            used_external_network: false,
            certifying: false,
        }
    }

    #[must_use]
    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    #[must_use]
    pub fn runtime_job_id(&self) -> &str {
        &self.runtime_job_id
    }

    #[must_use]
    pub fn model_job_id(&self) -> &str {
        &self.model_job_id
    }

    #[must_use]
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    #[must_use]
    pub fn evidence_label(&self) -> &str {
        self.evidence_label
    }

    #[must_use]
    pub fn runtime_state(&self) -> &str {
        self.runtime_state
    }

    #[must_use]
    pub fn model_state(&self) -> &str {
        self.model_state
    }

    #[must_use]
    pub fn route_status(&self) -> &str {
        self.route_status
    }

    #[must_use]
    pub fn setup_preview_observed(&self) -> bool {
        self.setup_preview_observed
    }

    #[must_use]
    pub fn loop_event_observed(&self) -> bool {
        self.loop_event_observed
    }

    #[must_use]
    pub fn blocked_route_observed(&self) -> bool {
        self.blocked_route_observed
    }

    #[must_use]
    pub fn used_external_network(&self) -> bool {
        self.used_external_network
    }

    #[must_use]
    pub fn certifying(&self) -> bool {
        self.certifying
    }
}
