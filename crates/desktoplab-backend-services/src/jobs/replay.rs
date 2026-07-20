use desktoplab_events::{JobEvent, JobEventKind};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SseReplay {
    pub(crate) events: Vec<JobEvent>,
}

impl SseReplay {
    pub(crate) fn new(events: Vec<JobEvent>) -> Self {
        Self { events }
    }

    #[must_use]
    pub fn sequences(&self) -> Vec<u64> {
        self.events.iter().map(JobEvent::sequence).collect()
    }

    #[must_use]
    pub fn event_names(&self) -> Vec<&'static str> {
        self.events
            .iter()
            .map(|event| event_name(event.kind()))
            .collect()
    }

    #[must_use]
    pub fn messages(&self) -> Vec<String> {
        self.events
            .iter()
            .filter_map(|event| {
                event.message().map(ToString::to_string).or_else(|| {
                    event
                        .progress()
                        .map(|progress| progress.message().to_string())
                })
            })
            .collect()
    }
}

fn event_name(kind: JobEventKind) -> &'static str {
    match kind {
        JobEventKind::Queued => "queued",
        JobEventKind::Started => "started",
        JobEventKind::Progress => "progress",
        JobEventKind::Succeeded => "succeeded",
        JobEventKind::Failed => "failed",
        JobEventKind::Cancelled => "cancelled",
        JobEventKind::Blocked => "blocked",
    }
}
