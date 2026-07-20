use desktoplab_downloads::{
    DownloadCapacity, DownloadError, DownloadFailure, DownloadFailureClass, DownloadJob,
    DownloadJobState, DownloadRequest, DownloadSignatureDecision,
};
use xtask::check_logical_line_limit;

#[test]
fn progress_events_are_ordered() {
    let mut job = DownloadJob::start(
        DownloadRequest::new(
            "download.runtime.ollama",
            "https://desktoplab.test/ollama",
            100,
        )
        .with_expected_checksum("sha256:test"),
        DownloadCapacity::new(1_000),
        DownloadSignatureDecision::Trusted,
    )
    .expect("download should start");

    job.record_progress(40);
    job.record_progress(100);

    assert_eq!(job.event_sequences(), vec![1, 2, 3, 4]);
    assert_eq!(
        job.progress_bytes(),
        100,
        "progress should retain the latest observed byte count"
    );
}

#[test]
fn checksum_mismatch_fails_closed() {
    let mut job = DownloadJob::start(
        DownloadRequest::new("download.model.qwen", "https://desktoplab.test/qwen", 100)
            .with_expected_checksum("sha256:expected"),
        DownloadCapacity::new(1_000),
        DownloadSignatureDecision::Trusted,
    )
    .expect("download should start");

    let error = job
        .complete_with_checksum("sha256:actual")
        .expect_err("checksum mismatch should fail closed");

    assert_eq!(
        error,
        DownloadError::ChecksumMismatch {
            expected: "sha256:expected".to_string(),
            actual: "sha256:actual".to_string(),
        }
    );
    assert_eq!(job.state(), DownloadJobState::Failed);
}

#[test]
fn cancellation_persists_cancelled_state() {
    let mut job = DownloadJob::start(
        DownloadRequest::new("download.registry", "https://desktoplab.test/registry", 10),
        DownloadCapacity::new(100),
        DownloadSignatureDecision::Trusted,
    )
    .expect("download should start");

    job.cancel();
    let restored = DownloadJob::from_metadata(job.metadata());

    assert_eq!(restored.state(), DownloadJobState::Cancelled);
}

#[test]
fn retryable_and_non_retryable_failures_are_classified() {
    assert_eq!(
        DownloadFailure::network_timeout().class(),
        DownloadFailureClass::Retryable
    );
    assert_eq!(
        DownloadFailure::checksum_mismatch().class(),
        DownloadFailureClass::NonRetryable
    );
}

#[test]
fn download_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-downloads/src/lib.rs",
        include_str!("../src/lib.rs"),
        280,
    )
    .expect("download source should stay below the initial line-count guard");
}
