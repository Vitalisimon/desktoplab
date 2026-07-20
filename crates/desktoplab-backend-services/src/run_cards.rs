use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;
const MAX_PAYLOAD_BYTES: usize = 256 * 1024;
const MAX_EVENTS: usize = 256;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunCardState {
    Scheduled,
    Running,
    Waiting,
    Stale,
    Completed,
    Failed,
    Cancelled,
}

impl RunCardState {
    fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunCardEvent {
    pub kind: String,
    pub at_ms: u64,
    pub actor_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperatorRunCard {
    schema_version: u32,
    pub run_id: String,
    pub owner_id: String,
    pub target_id: String,
    pub scheduling_intent: String,
    pub attempt: u32,
    pub state: RunCardState,
    pub heartbeat_at_ms: u64,
    pub transcript_summary: Option<String>,
    pub evidence_refs: Vec<String>,
    pub events: Vec<RunCardEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TakeoverRequest<'a> {
    pub new_owner_id: &'a str,
    pub has_takeover_capability: bool,
    pub policy_approved: bool,
    pub at_ms: u64,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RunCardError {
    Invalid(&'static str),
    NotFound,
    OwnerMismatch,
    Terminal,
    TakeoverDenied,
    StateTooLarge,
    Persistence(String),
}

pub struct RunCardService<'a> {
    storage: &'a SqliteStore,
}

impl<'a> RunCardService<'a> {
    pub fn new(storage: &'a SqliteStore) -> Self {
        Self { storage }
    }

    pub fn create(
        &self,
        run_id: &str,
        owner_id: &str,
        target_id: &str,
        scheduling_intent: &str,
        at_ms: u64,
    ) -> Result<OperatorRunCard, RunCardError> {
        for value in [run_id, owner_id, target_id, scheduling_intent] {
            if value.is_empty() || value.len() > 160 {
                return Err(RunCardError::Invalid("invalid_run_card_field"));
            }
        }
        if self.watch(run_id).is_ok() {
            return Err(RunCardError::Invalid("run_card_exists"));
        }
        let card = OperatorRunCard {
            schema_version: SCHEMA_VERSION,
            run_id: run_id.to_string(),
            owner_id: owner_id.to_string(),
            target_id: target_id.to_string(),
            scheduling_intent: scheduling_intent.to_string(),
            attempt: 1,
            state: RunCardState::Scheduled,
            heartbeat_at_ms: at_ms,
            transcript_summary: None,
            evidence_refs: Vec::new(),
            events: vec![event("scheduled", at_ms, owner_id)],
        };
        self.persist(&card)?;
        Ok(card)
    }

    pub fn heartbeat(
        &self,
        run_id: &str,
        owner_id: &str,
        at_ms: u64,
    ) -> Result<OperatorRunCard, RunCardError> {
        self.update(run_id, |card| {
            require_owner(card, owner_id)?;
            if card.state.terminal() {
                return Err(RunCardError::Terminal);
            }
            card.state = RunCardState::Running;
            card.heartbeat_at_ms = at_ms;
            push_event(card, event("heartbeat", at_ms, owner_id));
            Ok(())
        })
    }

    pub fn reconcile_stale(
        &self,
        run_id: &str,
        now_ms: u64,
        stale_after_ms: u64,
    ) -> Result<OperatorRunCard, RunCardError> {
        self.update(run_id, |card| {
            if !card.state.terminal()
                && now_ms.saturating_sub(card.heartbeat_at_ms) > stale_after_ms
                && card.state != RunCardState::Stale
            {
                card.state = RunCardState::Stale;
                let owner = card.owner_id.clone();
                push_event(card, event("heartbeat_lost", now_ms, &owner));
            }
            Ok(())
        })
    }

    pub fn takeover(
        &self,
        run_id: &str,
        request: TakeoverRequest<'_>,
    ) -> Result<OperatorRunCard, RunCardError> {
        self.update(run_id, |card| {
            if card.state.terminal() {
                return Err(RunCardError::Terminal);
            }
            if !request.has_takeover_capability || !request.policy_approved {
                return Err(RunCardError::TakeoverDenied);
            }
            if request.new_owner_id.is_empty() || request.new_owner_id.len() > 160 {
                return Err(RunCardError::Invalid("invalid_takeover_owner"));
            }
            card.owner_id = request.new_owner_id.to_string();
            card.attempt = card.attempt.saturating_add(1);
            card.state = RunCardState::Running;
            card.heartbeat_at_ms = request.at_ms;
            push_event(card, event("takeover", request.at_ms, request.new_owner_id));
            Ok(())
        })
    }

    pub fn complete(
        &self,
        run_id: &str,
        owner_id: &str,
        summary: &str,
        evidence_refs: &[String],
        at_ms: u64,
    ) -> Result<OperatorRunCard, RunCardError> {
        self.update(run_id, |card| {
            require_owner(card, owner_id)?;
            if card.state.terminal() {
                return Err(RunCardError::Terminal);
            }
            if summary.len() > 16 * 1024 || evidence_refs.len() > 64 {
                return Err(RunCardError::Invalid("run_archive_too_large"));
            }
            card.state = RunCardState::Completed;
            card.transcript_summary = Some(summary.to_string());
            card.evidence_refs = evidence_refs.to_vec();
            push_event(card, event("completed", at_ms, owner_id));
            Ok(())
        })
    }

    pub fn watch(&self, run_id: &str) -> Result<OperatorRunCard, RunCardError> {
        let record = self
            .storage
            .get_productization_state(ProductizationRecordKind::OperatorRunCard, run_id)
            .map_err(|error| RunCardError::Persistence(error.to_string()))?
            .ok_or(RunCardError::NotFound)?;
        let card: OperatorRunCard = serde_json::from_str(record.payload())
            .map_err(|error| RunCardError::Persistence(error.to_string()))?;
        if card.schema_version != SCHEMA_VERSION {
            return Err(RunCardError::Persistence(
                "unsupported_run_card_schema".to_string(),
            ));
        }
        Ok(card)
    }

    fn update(
        &self,
        run_id: &str,
        mutation: impl FnOnce(&mut OperatorRunCard) -> Result<(), RunCardError>,
    ) -> Result<OperatorRunCard, RunCardError> {
        let mut card = self.watch(run_id)?;
        mutation(&mut card)?;
        self.persist(&card)?;
        Ok(card)
    }

    fn persist(&self, card: &OperatorRunCard) -> Result<(), RunCardError> {
        let payload = serde_json::to_string(card)
            .map_err(|error| RunCardError::Persistence(error.to_string()))?;
        if payload.len() > MAX_PAYLOAD_BYTES {
            return Err(RunCardError::StateTooLarge);
        }
        self.storage
            .put_productization_state(ProductizationStateRecord::new(
                ProductizationRecordKind::OperatorRunCard,
                &card.run_id,
                payload,
            ))
            .map_err(|error| RunCardError::Persistence(error.to_string()))
    }
}

fn require_owner(card: &OperatorRunCard, owner_id: &str) -> Result<(), RunCardError> {
    if card.owner_id == owner_id {
        Ok(())
    } else {
        Err(RunCardError::OwnerMismatch)
    }
}

fn event(kind: &str, at_ms: u64, actor_id: &str) -> RunCardEvent {
    RunCardEvent {
        kind: kind.to_string(),
        at_ms,
        actor_id: actor_id.to_string(),
    }
}

fn push_event(card: &mut OperatorRunCard, value: RunCardEvent) {
    if card.events.len() == MAX_EVENTS {
        card.events.remove(0);
    }
    card.events.push(value);
}
