use desktoplab_events::{
    EventStream, FailureReason, FailureReasonCode, JobEventKind, JobId, JobQueue, JobState,
    Progress, RetryClassification,
};
use xtask::check_logical_line_limit;

#[test]
fn queued_and_started_jobs_emit_ordered_events() {
    let mut queue = JobQueue::default();
    let job_id = queue.enqueue("runtime.install");

    queue.start(&job_id).expect("job should start");
    queue.progress(&job_id, Progress::new(40, "downloading"));

    let events = queue.events_for(&job_id);

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].sequence(), 1);
    assert_eq!(events[0].kind(), JobEventKind::Queued);
    assert_eq!(events[1].sequence(), 2);
    assert_eq!(events[1].kind(), JobEventKind::Started);
    assert_eq!(events[2].sequence(), 3);
    assert_eq!(events[2].kind(), JobEventKind::Progress);
    assert_eq!(
        events[2].progress(),
        Some(&Progress::new(40, "downloading"))
    );
}

#[test]
fn cancellation_changes_state_predictably_and_emits_event() {
    let mut queue = JobQueue::default();
    let job_id = queue.enqueue("model.download");

    queue.start(&job_id).unwrap();
    queue
        .cancel(&job_id, "user requested cancellation")
        .unwrap();

    let job = queue.get(&job_id).expect("job should exist");
    let events = queue.events_for(&job_id);

    assert_eq!(job.state(), JobState::Cancelled);
    assert_eq!(events.last().unwrap().kind(), JobEventKind::Cancelled);
    assert_eq!(
        events.last().unwrap().message(),
        Some("user requested cancellation")
    );
}

#[test]
fn failures_include_machine_readable_reason_codes() {
    let mut queue = JobQueue::default();
    let job_id = queue.enqueue("registry.refresh");
    let failure = FailureReason::new(
        FailureReasonCode::NetworkUnavailable,
        "registry could not be reached",
    )
    .with_retry_classification(RetryClassification::Retryable);

    queue.start(&job_id).unwrap();
    queue.fail(&job_id, failure.clone()).unwrap();

    let job = queue.get(&job_id).expect("job should exist");
    let event = queue.events_for(&job_id).last().cloned().unwrap();

    assert_eq!(job.state(), JobState::Failed);
    assert_eq!(event.kind(), JobEventKind::Failed);
    assert_eq!(event.failure(), Some(&failure));
    assert_eq!(
        event.failure().unwrap().code(),
        FailureReasonCode::NetworkUnavailable
    );
    assert_eq!(
        event.failure().unwrap().retry_classification(),
        RetryClassification::Retryable
    );
}

#[test]
fn blocked_and_failed_states_are_distinct() {
    let mut queue = JobQueue::default();
    let blocked_job = queue.enqueue("provider.egress");
    let failed_job = queue.enqueue("runtime.install");

    queue
        .block(
            &blocked_job,
            FailureReason::new(FailureReasonCode::PolicyDenied, "provider egress denied"),
        )
        .unwrap();
    queue
        .fail(
            &failed_job,
            FailureReason::new(
                FailureReasonCode::RuntimeUnavailable,
                "runtime is not installed",
            ),
        )
        .unwrap();

    assert_eq!(queue.get(&blocked_job).unwrap().state(), JobState::Blocked);
    assert_eq!(queue.get(&failed_job).unwrap().state(), JobState::Failed);
}

#[test]
fn event_stream_replays_events_in_sequence_order_even_if_inserted_out_of_order() {
    let job_id = JobId::new("job.manual");
    let mut stream = EventStream::default();

    stream.record_for_test(job_id.clone(), 2, JobEventKind::Started);
    stream.record_for_test(job_id.clone(), 1, JobEventKind::Queued);

    let events = stream.replay(&job_id);

    assert_eq!(events[0].sequence(), 1);
    assert_eq!(events[0].kind(), JobEventKind::Queued);
    assert_eq!(events[1].sequence(), 2);
    assert_eq!(events[1].kind(), JobEventKind::Started);
}

#[test]
fn event_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-events/src/lib.rs",
        include_str!("../src/lib.rs"),
        250,
    )
    .expect("events lib should stay below the initial line-count guard");
}
