use super::{BackendEventFrame, BackendEventScope};

const DEFAULT_REPLAY_LIMIT: usize = 256;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventReplayRequest {
    after_sequence: u64,
    expected_stream_id: Option<String>,
    scope: Option<BackendEventScope>,
    max_events: usize,
}

impl EventReplayRequest {
    #[must_use]
    pub fn new() -> Self {
        Self {
            after_sequence: 0,
            expected_stream_id: None,
            scope: None,
            max_events: DEFAULT_REPLAY_LIMIT,
        }
    }

    #[must_use]
    pub fn after_sequence(mut self, sequence: u64) -> Self {
        self.after_sequence = sequence;
        self
    }

    #[must_use]
    pub fn expected_stream_id(mut self, stream_id: impl Into<String>) -> Self {
        self.expected_stream_id = Some(stream_id.into());
        self
    }

    #[must_use]
    pub fn scope(mut self, scope: BackendEventScope) -> Self {
        self.scope = Some(scope);
        self
    }

    #[must_use]
    pub fn max_events(mut self, max_events: usize) -> Self {
        self.max_events = max_events.clamp(1, DEFAULT_REPLAY_LIMIT);
        self
    }

    pub(super) fn after_sequence_value(&self) -> u64 {
        self.after_sequence
    }

    pub(super) fn expected_stream_id_value(&self) -> Option<&str> {
        self.expected_stream_id.as_deref()
    }

    pub(super) fn scope_value(&self) -> Option<BackendEventScope> {
        self.scope
    }

    pub(super) fn max_events_value(&self) -> usize {
        self.max_events
    }
}

impl Default for EventReplayRequest {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventReplayResponse {
    stream_id: String,
    oldest_sequence: Option<u64>,
    latest_sequence: u64,
    next_sequence: u64,
    has_more: bool,
    gap_detected: bool,
    reset_required: bool,
    frames: Vec<BackendEventFrame>,
}

impl EventReplayResponse {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        stream_id: String,
        oldest_sequence: Option<u64>,
        latest_sequence: u64,
        next_sequence: u64,
        has_more: bool,
        gap_detected: bool,
        reset_required: bool,
        frames: Vec<BackendEventFrame>,
    ) -> Self {
        Self {
            stream_id,
            oldest_sequence,
            latest_sequence,
            next_sequence,
            has_more,
            gap_detected,
            reset_required,
            frames,
        }
    }

    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
    pub fn oldest_sequence(&self) -> Option<u64> {
        self.oldest_sequence
    }
    pub fn latest_sequence(&self) -> u64 {
        self.latest_sequence
    }
    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }
    pub fn has_more(&self) -> bool {
        self.has_more
    }
    pub fn gap_detected(&self) -> bool {
        self.gap_detected
    }
    pub fn reset_required(&self) -> bool {
        self.reset_required
    }
    pub fn frames(&self) -> &[BackendEventFrame] {
        &self.frames
    }

    pub fn sequences(&self) -> Vec<u64> {
        self.frames
            .iter()
            .filter_map(BackendEventFrame::sequence)
            .collect()
    }

    pub fn scopes(&self) -> Vec<BackendEventScope> {
        self.frames.iter().filter_map(|frame| frame.scope).collect()
    }

    pub fn payloads(&self) -> Vec<&str> {
        self.frames
            .iter()
            .map(|frame| frame.payload.as_str())
            .collect()
    }
}
