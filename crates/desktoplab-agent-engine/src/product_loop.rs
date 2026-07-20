use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ApprovalDecision;
use desktoplab_tool_gateway::TerminalCommandRequest;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentPlanStore {
    plans: Rc<RefCell<BTreeMap<String, String>>>,
}

impl AgentPlanStore {
    pub fn put(&self, session_id: &str, plan: String) {
        self.plans.borrow_mut().insert(session_id.to_string(), plan);
    }

    #[must_use]
    pub fn get(&self, session_id: &str) -> Option<String> {
        self.plans.borrow().get(session_id).cloned()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionBackendAvailability {
    Available(String),
    Unavailable(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentPlanner {
    store: AgentPlanStore,
}

impl AgentPlanner {
    #[must_use]
    pub fn new(store: AgentPlanStore) -> Self {
        Self { store }
    }

    #[must_use]
    pub fn plan(
        &self,
        session_id: &str,
        prompt: &str,
        availability: ExecutionBackendAvailability,
    ) -> AgentPlan {
        match availability {
            ExecutionBackendAvailability::Available(backend_id) => {
                let plan = format!("backend={backend_id};prompt={prompt}");
                self.store.put(session_id, plan.clone());
                AgentPlan::new("planned", None, plan, vec!["plan_event"])
            }
            ExecutionBackendAvailability::Unavailable(reason) => AgentPlan::new(
                "blocked",
                Some("configure execution backend"),
                reason,
                vec!["plan_blocked"],
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentPlan {
    status: String,
    next_action: Option<String>,
    body: String,
    events: Vec<&'static str>,
}

impl AgentPlan {
    fn new(
        status: &str,
        next_action: Option<&str>,
        body: String,
        events: Vec<&'static str>,
    ) -> Self {
        Self {
            status: status.to_string(),
            next_action: next_action.map(ToString::to_string),
            body,
            events,
        }
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub fn next_action(&self) -> Option<&str> {
        self.next_action.as_deref()
    }

    #[must_use]
    pub fn events(&self) -> &[&'static str] {
        &self.events
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileEditEngine {
    root: PathBuf,
}

impl FileEditEngine {
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn apply(
        &self,
        relative_path: &str,
        expected_existing: &str,
        replacement: &str,
    ) -> Result<FileEditResult, String> {
        if relative_path.contains("..") || relative_path.starts_with('/') {
            return Err("outside_workspace".to_string());
        }
        let target = self.root.join(relative_path);
        let existing = fs::read_to_string(&target).map_err(|error| error.to_string())?;
        if existing != expected_existing {
            return Err("conflict".to_string());
        }
        fs::write(&target, replacement).map_err(|error| error.to_string())?;
        Ok(FileEditResult {
            diff_evidence: format!(
                "-{}+{}",
                expected_existing.trim_end(),
                replacement.trim_end()
            ),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileEditResult {
    diff_evidence: String,
}

impl FileEditResult {
    #[must_use]
    pub fn diff_evidence(&self) -> &str {
        &self.diff_evidence
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestFeedbackLoop {
    max_output: usize,
}

impl TestFeedbackLoop {
    #[must_use]
    pub fn new(max_output: usize) -> Self {
        Self { max_output }
    }

    #[must_use]
    pub fn capture(
        &self,
        command: &str,
        approval: Option<ApprovalDecision>,
        output: &str,
    ) -> TestFeedback {
        if approval != Some(ApprovalDecision::Approved) {
            return TestFeedback::new("approval_required", "");
        }
        let redacted = redact(output);
        let prefix = if redacted.contains("[REDACTED]") {
            "[REDACTED] "
        } else {
            ""
        };
        let mut summary = format!("{prefix}{command}:{redacted}");
        if summary.len() > self.max_output {
            summary.truncate(self.max_output);
        }
        TestFeedback::new("captured", &summary)
    }

    #[must_use]
    pub fn propose_command(&self, workspace_id: &str, command: &str) -> TestCommandProposal {
        TestCommandProposal {
            command: command.to_string(),
            terminal_request: TerminalCommandRequest::for_workspace(workspace_id, command),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestCommandProposal {
    command: String,
    terminal_request: TerminalCommandRequest,
}

impl TestCommandProposal {
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }

    #[must_use]
    pub fn terminal_request(&self) -> &TerminalCommandRequest {
        &self.terminal_request
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TestFeedback {
    status: String,
    summary: String,
}

impl TestFeedback {
    fn new(status: &str, summary: &str) -> Self {
        Self {
            status: status.to_string(),
            summary: summary.to_string(),
        }
    }

    #[must_use]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[must_use]
    pub fn summary(&self) -> &str {
        &self.summary
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionControl {
    session_id: String,
    state: SessionControlState,
}

impl SessionControl {
    #[must_use]
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            state: SessionControlState::Running,
        }
    }

    pub fn pause(&mut self) {
        self.state = SessionControlState::Paused;
    }

    pub fn resume(&mut self) {
        self.state = SessionControlState::Running;
    }

    pub fn cancel(&mut self) {
        self.state = SessionControlState::Cancelled;
    }

    #[must_use]
    pub fn can_execute_tools(&self) -> bool {
        self.state == SessionControlState::Running
    }

    #[must_use]
    pub fn can_mutate_files(&self) -> bool {
        self.state != SessionControlState::Cancelled
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionControlState {
    Running,
    Paused,
    Cancelled,
}

fn redact(output: &str) -> String {
    output
        .split_whitespace()
        .map(|part| {
            if part.contains("TOKEN=") || part.contains("secret") {
                "[REDACTED]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
