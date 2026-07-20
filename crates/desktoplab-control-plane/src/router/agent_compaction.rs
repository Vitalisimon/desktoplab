use desktoplab_agent_session::AgentSession;
use desktoplab_redaction::redact_sensitive;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const COMPACTION_THRESHOLD_TURNS: usize = 24;
const RECENT_TURNS_TO_KEEP: usize = 12;
const MAX_COMPACTED_TURNS: usize = 64;
const MAX_TURN_BYTES: usize = 192;
const MAX_SUMMARY_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentContextCompaction {
    summary: String,
    through_event_sequence: usize,
    source_turns: usize,
    source_bytes: usize,
    source_hash: String,
}

impl AgentContextCompaction {
    pub(crate) fn build(session: &AgentSession) -> Option<Self> {
        let turns = super::agent_transcript::indexed_context_turns(session);
        if turns.len() <= COMPACTION_THRESHOLD_TURNS {
            return None;
        }
        let compacted_count = turns.len().saturating_sub(RECENT_TURNS_TO_KEEP);
        let compacted = &turns[..compacted_count];
        let through_event_sequence = compacted.last()?.0;
        let source = compacted
            .iter()
            .map(|(sequence, turn)| format!("{sequence}:{turn}"))
            .collect::<Vec<_>>()
            .join("\n");
        let selected = selected_turns(compacted);
        let omitted = compacted.len().saturating_sub(selected.len());
        let mut summary = format!(
            "compaction=desktoplab.extractive.v1 source_turns={} source_bytes={} omitted_turns={omitted}\n",
            compacted.len(),
            source.len()
        );
        for (sequence, turn) in selected {
            summary.push_str(&format!(
                "event {sequence}: {}\n",
                truncate_utf8(turn, MAX_TURN_BYTES)
            ));
        }
        summary = truncate_utf8(&summary, MAX_SUMMARY_BYTES);
        Some(Self {
            summary: redact_sensitive(&summary),
            through_event_sequence,
            source_turns: compacted.len(),
            source_bytes: source.len(),
            source_hash: format!("{:x}", Sha256::digest(source.as_bytes())),
        })
    }

    pub(crate) fn summary(&self) -> &str {
        &self.summary
    }

    pub(crate) fn through_event_sequence(&self) -> usize {
        self.through_event_sequence
    }

    pub(crate) fn to_json(&self) -> Value {
        json!({
            "summary":self.summary,"throughEventSequence":self.through_event_sequence,
            "sourceTurns":self.source_turns,"sourceBytes":self.source_bytes,
            "sourceHash":self.source_hash
        })
    }

    pub(crate) fn from_json(value: &Value) -> Option<Self> {
        let summary = value.get("summary")?.as_str()?;
        let through_event_sequence = value.get("throughEventSequence")?.as_u64()? as usize;
        let source_turns = value.get("sourceTurns")?.as_u64()? as usize;
        let source_bytes = value.get("sourceBytes")?.as_u64()? as usize;
        let source_hash = value.get("sourceHash")?.as_str()?;
        if summary.len() > MAX_SUMMARY_BYTES
            || through_event_sequence == 0
            || source_turns == 0
            || source_bytes == 0
            || source_hash.len() != 64
            || !source_hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return None;
        }
        Some(Self {
            summary: redact_sensitive(summary),
            through_event_sequence,
            source_turns,
            source_bytes,
            source_hash: source_hash.to_string(),
        })
    }
}

fn selected_turns(turns: &[(usize, String)]) -> Vec<&(usize, String)> {
    if turns.len() <= MAX_COMPACTED_TURNS {
        return turns.iter().collect();
    }
    let first = MAX_COMPACTED_TURNS / 4;
    let last = MAX_COMPACTED_TURNS - first;
    turns[..first]
        .iter()
        .chain(turns[turns.len() - last..].iter())
        .collect()
}

fn truncate_utf8(value: &str, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{} [truncated]", &value[..end])
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_session::{AgentSession, SessionEvent};

    use super::{AgentContextCompaction, MAX_SUMMARY_BYTES};

    #[test]
    fn compaction_is_bounded_redacted_and_excludes_recent_turns() {
        let mut session = AgentSession::new("session.long", "backend.ollama");
        for turn in 0..40 {
            session.apply(SessionEvent::planning_started(format!(
                "user turn {turn} token=secret"
            )));
            session.apply(SessionEvent::backend_response_received(format!(
                "assistant turn {turn}"
            )));
        }

        let compaction = AgentContextCompaction::build(&session).unwrap();
        assert!(compaction.summary().len() <= MAX_SUMMARY_BYTES);
        assert!(compaction.summary().contains("desktoplab.extractive.v1"));
        assert!(compaction.summary().contains("token=[REDACTED]"));
        assert!(!compaction.summary().contains("token=secret"));
        assert!(!compaction.summary().contains("assistant turn 39"));
        assert_eq!(
            compaction.to_json()["sourceHash"].as_str().unwrap().len(),
            64
        );
        assert_eq!(
            AgentContextCompaction::from_json(&compaction.to_json()),
            Some(compaction)
        );
    }

    #[test]
    fn persisted_compaction_rejects_unbounded_or_invalid_evidence() {
        let oversized = serde_json::json!({
            "summary":"x".repeat(MAX_SUMMARY_BYTES + 1),
            "throughEventSequence":1,"sourceTurns":1,"sourceBytes":1,
            "sourceHash":"a".repeat(64)
        });
        assert!(AgentContextCompaction::from_json(&oversized).is_none());
    }
}
