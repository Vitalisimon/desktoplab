use crate::StorageError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamKind {
    Session,
    Job,
}

impl StreamKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::Job => "job",
        }
    }

    #[must_use]
    pub fn from_storage(value: &str) -> Self {
        match value {
            "job" => Self::Job,
            _ => Self::Session,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedactionStatus {
    Clean,
    Redacted,
}

impl RedactionStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Redacted => "redacted",
        }
    }

    #[must_use]
    pub fn from_storage(value: &str) -> Self {
        match value {
            "redacted" => Self::Redacted,
            _ => Self::Clean,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelope {
    event_id: String,
    stream_id: String,
    stream_kind: StreamKind,
    sequence: u64,
    schema_version: u16,
    occurred_at: String,
    recorded_at: String,
    actor: String,
    event_type: String,
    payload: String,
    correlation_id: String,
    redaction_status: RedactionStatus,
    trust_context: String,
}

impl EventEnvelope {
    #[must_use]
    pub fn new(
        event_id: impl Into<String>,
        stream_id: impl Into<String>,
        stream_kind: StreamKind,
        sequence: u64,
        event_type: impl Into<String>,
        payload: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            stream_id: stream_id.into(),
            stream_kind,
            sequence,
            schema_version: 1,
            occurred_at: "1970-01-01T00:00:00Z".to_string(),
            recorded_at: "1970-01-01T00:00:00Z".to_string(),
            actor: "desktoplab".to_string(),
            event_type: event_type.into(),
            payload: payload.into(),
            correlation_id: String::new(),
            redaction_status: RedactionStatus::Clean,
            trust_context: "local".to_string(),
        }
    }

    #[must_use]
    pub fn with_redaction_status(mut self, redaction_status: RedactionStatus) -> Self {
        self.redaction_status = redaction_status;
        self
    }

    #[must_use]
    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    #[must_use]
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    #[must_use]
    pub fn stream_kind(&self) -> StreamKind {
        self.stream_kind
    }

    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    #[must_use]
    pub fn payload(&self) -> &str {
        &self.payload
    }

    #[must_use]
    pub fn redaction_status(&self) -> RedactionStatus {
        self.redaction_status
    }

    pub(crate) fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub(crate) fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    pub(crate) fn recorded_at(&self) -> &str {
        &self.recorded_at
    }

    pub(crate) fn actor(&self) -> &str {
        &self.actor
    }

    pub(crate) fn correlation_id(&self) -> &str {
        &self.correlation_id
    }

    pub(crate) fn trust_context(&self) -> &str {
        &self.trust_context
    }

    pub(crate) fn from_storage(row: StoredEventRow) -> Self {
        Self {
            event_id: row.event_id,
            stream_id: row.stream_id,
            stream_kind: StreamKind::from_storage(&row.stream_kind),
            sequence: row.sequence,
            schema_version: row.schema_version,
            occurred_at: row.occurred_at,
            recorded_at: row.recorded_at,
            actor: row.actor,
            event_type: row.event_type,
            payload: row.payload,
            correlation_id: row.correlation_id,
            redaction_status: RedactionStatus::from_storage(&row.redaction_status),
            trust_context: row.trust_context,
        }
    }
}

pub(crate) struct StoredEventRow {
    pub event_id: String,
    pub stream_id: String,
    pub stream_kind: String,
    pub sequence: u64,
    pub schema_version: u16,
    pub occurred_at: String,
    pub recorded_at: String,
    pub actor: String,
    pub event_type: String,
    pub payload: String,
    pub correlation_id: String,
    pub redaction_status: String,
    pub trust_context: String,
}

pub trait EventStore {
    fn append_event(&self, event: EventEnvelope) -> Result<(), StorageError>;

    fn replay_stream(&self, stream_id: &str) -> Result<Vec<EventEnvelope>, StorageError>;
}
