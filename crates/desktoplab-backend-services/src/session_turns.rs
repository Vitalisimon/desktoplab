use desktoplab_storage::StorageError;
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionTurnSnapshot {
    turn_id: String,
    prompt: String,
    state: String,
}

impl SessionTurnSnapshot {
    pub fn turn_id(&self) -> &str {
        &self.turn_id
    }
    pub fn prompt(&self) -> &str {
        &self.prompt
    }
    pub fn state(&self) -> &str {
        &self.state
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct SessionTurnQueue {
    next_turn_number: u64,
    turns: Vec<SessionTurnSnapshot>,
    cancellation: Option<CancellationState>,
}

#[derive(Clone, Debug)]
struct CancellationState {
    requested_at_ms: u64,
    deadline_at_ms: u64,
    state: String,
}

impl SessionTurnQueue {
    pub fn enqueue(&mut self, prompt: impl Into<String>) -> SessionTurnSnapshot {
        self.next_turn_number += 1;
        let turn = SessionTurnSnapshot {
            turn_id: format!("turn.{}", self.next_turn_number),
            prompt: prompt.into(),
            state: "queued".to_string(),
        };
        self.turns.push(turn.clone());
        turn
    }

    pub fn claim_next(&mut self) -> Option<SessionTurnSnapshot> {
        let turn = self.turns.iter_mut().find(|turn| turn.state == "queued")?;
        turn.state = "in_progress".to_string();
        Some(turn.clone())
    }

    pub fn complete(&mut self, turn_id: &str) -> bool {
        let before = self.turns.len();
        self.turns.retain(|turn| turn.turn_id != turn_id);
        before != self.turns.len()
    }

    pub fn recover_interrupted(&mut self) {
        for turn in &mut self.turns {
            if turn.state == "in_progress" {
                turn.state = "queued".to_string();
            }
        }
        if self
            .cancellation
            .as_ref()
            .is_some_and(|state| state.state == "requested")
        {
            self.cancellation = None;
        }
    }

    pub fn snapshots(&self) -> Vec<SessionTurnSnapshot> {
        self.turns.clone()
    }

    pub fn request_cancel(&mut self, requested_at_ms: u64, grace_ms: u64) {
        self.cancellation = Some(CancellationState {
            requested_at_ms,
            deadline_at_ms: requested_at_ms.saturating_add(grace_ms),
            state: "requested".to_string(),
        });
    }

    pub fn acknowledge_cancel(&mut self) {
        if let Some(state) = self.cancellation.as_mut() {
            state.state = "acknowledged".to_string();
        }
    }

    pub fn force_cancel_if_due(&mut self, now_ms: u64) -> bool {
        let Some(state) = self.cancellation.as_mut() else {
            return false;
        };
        if state.state != "requested" || now_ms < state.deadline_at_ms {
            return false;
        }
        state.state = "forced".to_string();
        true
    }

    pub fn cancellation_state(&self) -> Option<&str> {
        self.cancellation.as_ref().map(|state| state.state.as_str())
    }

    pub fn to_value(&self) -> Value {
        json!({
            "nextTurnNumber":self.next_turn_number,
            "turns":self.turns.iter().map(|turn| json!({"turnId":turn.turn_id,"prompt":turn.prompt,"state":turn.state})).collect::<Vec<_>>(),
            "cancellation":self.cancellation.as_ref().map(|state| json!({"requestedAtMs":state.requested_at_ms,"deadlineAtMs":state.deadline_at_ms,"state":state.state}))
        })
    }

    pub fn from_value(value: Option<&Value>) -> Result<Self, StorageError> {
        let Some(value) = value else {
            return Ok(Self::default());
        };
        let empty_turns = Vec::new();
        let turns = value
            .get("turns")
            .and_then(Value::as_array)
            .unwrap_or(&empty_turns)
            .iter()
            .map(|turn| {
                Ok(SessionTurnSnapshot {
                    turn_id: required_string(turn, "turnId")?,
                    prompt: required_string(turn, "prompt")?,
                    state: required_string(turn, "state")?,
                })
            })
            .collect::<Result<Vec<_>, StorageError>>()?;
        let cancellation = value
            .get("cancellation")
            .filter(|item| !item.is_null())
            .map(|item| -> Result<CancellationState, StorageError> {
                Ok(CancellationState {
                    requested_at_ms: item
                        .get("requestedAtMs")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    deadline_at_ms: item
                        .get("deadlineAtMs")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                    state: required_string(item, "state")?,
                })
            })
            .transpose()?;
        Ok(Self {
            next_turn_number: value
                .get("nextTurnNumber")
                .and_then(Value::as_u64)
                .unwrap_or_default(),
            turns,
            cancellation,
        })
    }
}

fn required_string(value: &Value, field: &str) -> Result<String, StorageError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| StorageError::InvalidJson(format!("{field} missing")))
}
