use desktoplab_backend_services::{
    JobRetryClass, JobService, JobServiceStore, JobState, SseReplay,
};
use desktoplab_model_manager::{ModelDownloadPlan, ModelManager};
use xtask::check_logical_line_limit;

#[test]
fn job_state_persists_across_service_restart() {
    let store = JobServiceStore::default();
    let mut first_service = JobService::new(store.clone());
    let job = first_service.create_job("model.download");
    first_service.start(job.id()).expect("job should start");

    let restarted = JobService::new(store);

    assert_eq!(
        restarted
            .get_job(job.id())
            .expect("job should persist")
            .state(),
        JobState::Running
    );
}

#[test]
fn cancellation_emits_event_and_changes_state() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let job = service.create_job("runtime.install");

    service.cancel(job.id()).expect("job should cancel");

    assert_eq!(
        service.get_job(job.id()).expect("job should exist").state(),
        JobState::Cancelled
    );
    assert_eq!(
        service.replay(job.id(), 0).event_names(),
        vec!["queued", "cancelled"]
    );
}

#[test]
fn retry_respects_retry_classification() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let retryable = service.create_job("registry.refresh");
    let non_retryable = service.create_job("model.verify");

    service
        .fail(retryable.id(), JobRetryClass::Retryable)
        .expect("job should fail");
    service
        .fail(non_retryable.id(), JobRetryClass::NonRetryable)
        .expect("job should fail");

    assert!(service.retry(retryable.id()).is_ok());
    assert_eq!(service.retry(non_retryable.id()), Err("retry_not_allowed"));
}

#[test]
fn sse_stream_replays_ordered_events_after_last_seen_sequence() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let job = service.create_job("setup.plan");
    service.start(job.id()).expect("job should start");
    service.cancel(job.id()).expect("job should cancel");

    let replay: SseReplay = service.replay(job.id(), 1);

    assert_eq!(replay.sequences(), vec![2, 3]);
    assert_eq!(replay.event_names(), vec!["started", "cancelled"]);
}

#[test]
fn runtime_download_job_transitions_through_progress_to_success() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let job = service.create_job("runtime.download");

    service.start(job.id()).expect("job should start");
    service
        .progress(job.id(), 45, "downloading Ollama installer")
        .expect("job should record progress");
    service.succeed(job.id()).expect("job should complete");

    assert_eq!(
        service.get_job(job.id()).expect("job should exist").state(),
        JobState::Succeeded
    );
    assert_eq!(
        service.replay(job.id(), 0).event_names(),
        vec!["queued", "started", "progress", "succeeded"]
    );
}

#[test]
fn runtime_download_failures_keep_retry_classification() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let retryable = service.create_job("runtime.download");
    let blocked = service.create_job("runtime.download");

    service
        .fail_with_message(
            retryable.id(),
            JobRetryClass::Retryable,
            "source temporarily unavailable",
        )
        .expect("retryable failure should be recorded");
    service
        .fail_with_message(
            blocked.id(),
            JobRetryClass::NonRetryable,
            "checksum mismatch",
        )
        .expect("blocked failure should be recorded");

    assert!(service.retry(retryable.id()).is_ok());
    assert_eq!(service.retry(blocked.id()), Err("retry_not_allowed"));
}

#[test]
fn runtime_download_events_are_redacted_and_bounded() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let job = service.create_job("runtime.download");
    let noisy = format!("downloading token=sk-secret {}", "x".repeat(180));

    service
        .progress(job.id(), 10, noisy)
        .expect("progress should be recorded");

    let details = service.replay(job.id(), 0).messages();
    assert!(details.iter().any(|detail| detail.contains("[REDACTED]")));
    assert!(details.iter().all(|detail| detail.len() <= 96));
    assert!(details.iter().all(|detail| !detail.contains("sk-secret")));
}

#[test]
fn model_download_job_records_plan_metadata_and_progress() {
    let store = JobServiceStore::default();
    let mut service = JobService::new(store);
    let catalog = ModelManager::new().default_family_catalog();
    let variant = catalog
        .variants()
        .first()
        .expect("default agent catalog should contain a downloadable variant");
    let expected_family_id = variant.family_id().to_string();
    let expected_model_id = variant.model_id().to_string();
    let plan = ModelDownloadPlan::from_variant(variant, true);

    let job = service.create_model_download_job(&plan);
    service
        .start(job.id())
        .expect("model download should start");
    service
        .progress(
            job.id(),
            50,
            format!(
                "{} token=secret",
                JobService::model_download_metadata(&plan)
            ),
        )
        .expect("model progress should be recorded");
    service
        .succeed(job.id())
        .expect("model download should finish");

    assert_eq!(
        service.get_job(job.id()).expect("job should exist").state(),
        JobState::Succeeded
    );
    assert_eq!(
        service.replay(job.id(), 0).event_names(),
        vec!["queued", "started", "progress", "succeeded"]
    );
    let details = service.replay(job.id(), 0).messages();
    assert!(
        details
            .iter()
            .any(|detail| detail.contains(&expected_family_id))
    );
    assert!(
        details
            .iter()
            .any(|detail| detail.contains(&expected_model_id))
    );
    assert!(
        details
            .iter()
            .all(|detail| !detail.contains("token=secret"))
    );
}

#[test]
fn job_service_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/jobs.rs",
        include_str!("../src/jobs.rs"),
        300,
    )
    .expect("job service source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/jobs/helpers.rs",
        include_str!("../src/jobs/helpers.rs"),
        80,
    )
    .expect("job helper source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/jobs/replay.rs",
        include_str!("../src/jobs/replay.rs"),
        100,
    )
    .expect("job replay source should stay below the line-count guard");
}
