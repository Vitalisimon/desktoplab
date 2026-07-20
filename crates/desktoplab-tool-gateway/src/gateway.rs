use desktoplab_policy::{
    Action, ApprovalMode, DecisionOutcome, PolicyDecisionRecord, PolicyEngine, PolicyLayerSnapshot,
};

use crate::{
    TerminalCommandClass, ToolIntent, classify_terminal_command,
    path_security::is_sensitive_workspace_path,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolOutcome {
    Allowed(Action),
    ApprovalRequired(Action),
    Blocked(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovalRequest {
    action: Action,
}

impl ApprovalRequest {
    #[must_use]
    pub fn action(&self) -> Action {
        self.action
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolAuditRecord {
    decision: PolicyDecisionRecord,
    policy_snapshot: PolicyLayerSnapshot,
}

impl ToolAuditRecord {
    #[must_use]
    pub fn decision(&self) -> &PolicyDecisionRecord {
        &self.decision
    }

    #[must_use]
    pub fn policy_snapshot(&self) -> &PolicyLayerSnapshot {
        &self.policy_snapshot
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolGateway {
    policy: PolicyEngine,
    approval_requests: Vec<ApprovalRequest>,
    audit_records: Vec<ToolAuditRecord>,
}

impl ToolGateway {
    #[must_use]
    pub fn new(policy: PolicyEngine) -> Self {
        Self {
            policy,
            approval_requests: Vec::new(),
            audit_records: Vec::new(),
        }
    }

    pub fn authorize(&mut self, intent: ToolIntent) -> ToolOutcome {
        if intent.path().is_some_and(is_sensitive_workspace_path)
            || intent
                .secondary_path()
                .is_some_and(is_sensitive_workspace_path)
        {
            let decision = self.policy.evaluate(Action::ProtectedWorkspaceAccess);
            self.push_audit_record(&decision);
            return ToolOutcome::Blocked("local_only_path".to_string());
        }

        let action = action_for_intent(&intent);
        let decision = self.policy.evaluate(action);
        self.push_audit_record(&decision);

        match decision.outcome() {
            DecisionOutcome::AllowedAutomatic => ToolOutcome::Allowed(action),
            DecisionOutcome::RequiresApproval => {
                self.approval_requests.push(ApprovalRequest { action });
                ToolOutcome::ApprovalRequired(action)
            }
            DecisionOutcome::Denied => ToolOutcome::Blocked("policy_denied".to_string()),
        }
    }

    #[must_use]
    pub fn approval_requests(&self) -> &[ApprovalRequest] {
        &self.approval_requests
    }

    #[must_use]
    pub fn audit_records(&self) -> &[ToolAuditRecord] {
        &self.audit_records
    }

    #[must_use]
    pub fn approval_mode(&self) -> ApprovalMode {
        self.policy.approval_mode()
    }

    fn push_audit_record(&mut self, decision: &desktoplab_policy::PolicyDecision) {
        self.audit_records.push(ToolAuditRecord {
            decision: decision.to_audit_record(),
            policy_snapshot: self.policy.layer_snapshot(decision.action()),
        });
    }
}

fn action_for_intent(intent: &ToolIntent) -> Action {
    match intent {
        ToolIntent::FilesystemList { .. }
        | ToolIntent::FilesystemRead { .. }
        | ToolIntent::SearchText { .. } => Action::FilesystemRead,
        ToolIntent::ProcessPoll { .. }
        | ToolIntent::ProcessStdin { .. }
        | ToolIntent::ProcessKill { .. } => Action::ProcessControl,
        ToolIntent::CreateCheckpoint { .. } => Action::CheckpointCreate,
        ToolIntent::McpInvoke { .. } => Action::McpInvoke,
        ToolIntent::Clarify { .. } => Action::Clarification,
        ToolIntent::FilesystemWrite { path } | ToolIntent::FilesystemPatch { path }
            if is_generated_or_lockfile_path(path) =>
        {
            Action::GeneratedArtifactWrite
        }
        ToolIntent::FilesystemWrite { .. }
        | ToolIntent::FilesystemPatch { .. }
        | ToolIntent::FilesystemCreateDirectory { .. }
        | ToolIntent::FilesystemMove { .. }
        | ToolIntent::FilesystemDelete { .. } => Action::FilesystemWrite,
        ToolIntent::Terminal { command, .. } => action_for_terminal_command(command),
        ToolIntent::ProcessStart { .. } => Action::ProcessStart,
        ToolIntent::TestRun { .. } => Action::TestRun,
        ToolIntent::GitStatus | ToolIntent::GitDiff { .. } => Action::GitRead,
        ToolIntent::GitCommit { .. } => Action::GitCommit,
        ToolIntent::GitPush { .. } => Action::GitPush,
        ToolIntent::RuntimeInstall { .. } => Action::RuntimeInstall,
    }
}

fn action_for_terminal_command(command: &str) -> Action {
    match classify_terminal_command(command) {
        TerminalCommandClass::DependencyInstall => Action::DependencyInstall,
        TerminalCommandClass::GeneratedArtifact | TerminalCommandClass::Routine => {
            Action::TerminalCommand
        }
    }
}

fn is_generated_or_lockfile_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(&normalized);
    matches!(
        file_name,
        "package-lock.json"
            | "pnpm-lock.yaml"
            | "yarn.lock"
            | "Cargo.lock"
            | "poetry.lock"
            | "go.sum"
            | "Package.resolved"
    ) || normalized.starts_with("dist/")
        || normalized.starts_with("build/")
        || normalized.starts_with("target/")
        || normalized.starts_with(".next/")
        || normalized.starts_with("generated/")
}
