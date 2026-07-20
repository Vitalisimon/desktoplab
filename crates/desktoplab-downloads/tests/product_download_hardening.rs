use desktoplab_downloads::{
    DownloadCapacity, DownloadJob, DownloadRequest, DownloadSignatureDecision,
    ProductDownloadReservation, ResumableDownloadPlan,
};
use xtask::check_logical_line_limit;

#[test]
fn interrupted_downloads_resume_from_safe_temp_metadata() {
    let mut job = DownloadJob::start(
        DownloadRequest::new("download.model.qwen", "https://desktoplab.test/qwen", 1_000)
            .with_expected_checksum("sha256:expected"),
        DownloadCapacity::new(2_000),
        DownloadSignatureDecision::Trusted,
    )
    .expect("download should start");
    job.record_progress(400);

    let resume = ResumableDownloadPlan::from_metadata(job.metadata(), ".desktoplab/tmp/qwen.part")
        .expect("partial metadata should be resumable");

    assert_eq!(resume.resume_from_bytes(), 400);
    assert_eq!(resume.temp_path(), ".desktoplab/tmp/qwen.part");
    assert!(resume.uses_safe_temp_path());
}

#[test]
fn disk_reservation_blocks_before_starting_download() {
    let reservation = ProductDownloadReservation::new(5_000, DownloadCapacity::new(4_000));

    assert!(reservation.reserve().is_err());
    assert!(!reservation.has_side_effects());
}

#[test]
fn product_download_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-downloads/src/productization.rs",
        include_str!("../src/productization.rs"),
        180,
    )
    .expect("product download hardening source should stay focused");
}
