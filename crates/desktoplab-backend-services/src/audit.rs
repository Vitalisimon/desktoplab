use desktoplab_redaction::redact_sensitive;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditAction {
    PolicyDecision,
    ApprovalDecision,
    ToolExecution,
    RuntimeInstall,
    ModelDownload,
    ProviderEgress,
    PluginTrust,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    sequence: u64,
    action: AuditAction,
    denied: bool,
    details: String,
}

impl AuditRecord {
    #[must_use]
    pub fn action(&self) -> AuditAction {
        self.action
    }

    #[must_use]
    pub fn details(&self) -> &str {
        &self.details
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditDecisionSummary {
    pub sequence: u64,
    pub action: String,
    pub outcome: String,
    pub redacted_details: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalAuditTransparencySnapshot {
    pub scope: String,
    pub records: Vec<AuditDecisionSummary>,
    pub redacted_export: String,
}

#[derive(Clone, Debug, Default)]
pub struct AuditStore {
    inner: Arc<Mutex<AuditData>>,
}

#[derive(Clone, Debug, Default)]
struct AuditData {
    next_sequence: u64,
    records: Vec<AuditRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditQuery {
    All,
    Action(AuditAction),
    Denied,
}

impl AuditQuery {
    #[must_use]
    pub fn all() -> Self {
        Self::All
    }

    #[must_use]
    pub fn action(action: AuditAction) -> Self {
        Self::Action(action)
    }

    #[must_use]
    pub fn denied() -> Self {
        Self::Denied
    }
}

#[derive(Clone, Debug)]
pub struct AuditLogService {
    store: AuditStore,
}

impl AuditLogService {
    #[must_use]
    pub fn new(store: AuditStore) -> Self {
        Self { store }
    }

    pub fn record(&mut self, action: AuditAction, details: impl AsRef<str>) {
        self.push(action, false, details.as_ref());
    }

    pub fn record_denied(&mut self, action: AuditAction, details: impl AsRef<str>) {
        self.push(action, true, details.as_ref());
    }

    #[must_use]
    pub fn query(&self, query: AuditQuery) -> Vec<AuditRecord> {
        self.store
            .inner
            .lock()
            .expect("audit store lock should not be poisoned")
            .records
            .iter()
            .filter(|record| query_matches(query, record))
            .cloned()
            .collect()
    }

    #[must_use]
    pub fn export_redacted(&self, query: AuditQuery) -> String {
        self.summaries(query, usize::MAX)
            .iter()
            .map(|record| {
                format!(
                    "{} {}: {}",
                    record.action, record.outcome, record.redacted_details
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[must_use]
    pub fn summaries(&self, query: AuditQuery, limit: usize) -> Vec<AuditDecisionSummary> {
        self.query(query)
            .iter()
            .rev()
            .take(limit)
            .rev()
            .map(summary_for)
            .collect()
    }

    #[must_use]
    pub fn transparency_snapshot(
        &self,
        query: AuditQuery,
        limit: usize,
    ) -> LocalAuditTransparencySnapshot {
        let records = self.summaries(query, limit);
        let redacted_export = records
            .iter()
            .map(|record| {
                format!(
                    "{} {}: {}",
                    record.action, record.outcome, record.redacted_details
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        LocalAuditTransparencySnapshot {
            scope: "local_single_user".to_string(),
            records,
            redacted_export,
        }
    }

    fn push(&mut self, action: AuditAction, denied: bool, details: &str) {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("audit store lock should not be poisoned");
        data.next_sequence += 1;
        let sequence = data.next_sequence;
        data.records.push(AuditRecord {
            sequence,
            action,
            denied,
            details: redact_sensitive(details),
        });
    }
}

fn query_matches(query: AuditQuery, record: &AuditRecord) -> bool {
    match query {
        AuditQuery::All => true,
        AuditQuery::Action(action) => record.action == action,
        AuditQuery::Denied => record.denied,
    }
}

fn summary_for(record: &AuditRecord) -> AuditDecisionSummary {
    AuditDecisionSummary {
        sequence: record.sequence,
        action: action_key(record.action).to_string(),
        outcome: if record.denied { "denied" } else { "allowed" }.to_string(),
        redacted_details: record.details.clone(),
    }
}

fn action_key(action: AuditAction) -> &'static str {
    match action {
        AuditAction::PolicyDecision => "policy_decision",
        AuditAction::ApprovalDecision => "approval_decision",
        AuditAction::ToolExecution => "tool_execution",
        AuditAction::RuntimeInstall => "runtime_install",
        AuditAction::ModelDownload => "model_download",
        AuditAction::ProviderEgress => "provider_egress",
        AuditAction::PluginTrust => "plugin_trust",
    }
}
