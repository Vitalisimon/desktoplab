use desktoplab_events::{JobEvent, JobId};

pub(crate) fn next_sequence(events: &[JobEvent], job_id: &JobId) -> u64 {
    events
        .iter()
        .filter(|event| event.job_id() == job_id)
        .map(JobEvent::sequence)
        .max()
        .unwrap_or(0)
        + 1
}

pub(crate) fn sanitize_message(value: &str) -> String {
    let redacted = value
        .split_whitespace()
        .map(|part| {
            if part.contains("sk-") || part.to_ascii_lowercase().contains("token=") {
                "[REDACTED]"
            } else {
                part
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    bound_message(&redacted)
}

fn bound_message(value: &str) -> String {
    const MAX_LEN: usize = 96;
    if value.len() <= MAX_LEN {
        return value.to_string();
    }
    format!("{}...", &value[..MAX_LEN - 3])
}
