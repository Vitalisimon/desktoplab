use desktoplab_model_manager::ModelPullProgress;

#[test]
fn parses_ollama_percent_and_downloaded_bytes_from_progress_line() {
    let progress = ModelPullProgress::parse("pulling 8f3a2d: 42% ▕████      ▏ 1.5 GB/3.6 GB");

    assert_eq!(progress.stage(), "pulling");
    assert_eq!(progress.percent(), Some(42));
    assert_eq!(progress.downloaded_bytes(), Some(1_500_000_000));
    assert_eq!(progress.total_bytes(), Some(3_600_000_000));
    assert!(progress.evidence().contains("pulling 8f3a2d"));
}

#[test]
fn parses_ollama_megabyte_progress_line() {
    let progress = ModelPullProgress::parse("pulling manifest: 8% 240 MB/3.0 GB");

    assert_eq!(progress.percent(), Some(8));
    assert_eq!(progress.downloaded_bytes(), Some(240_000_000));
    assert_eq!(progress.total_bytes(), Some(3_000_000_000));
}

#[test]
fn unknown_output_remains_log_evidence_without_crashing() {
    let progress = ModelPullProgress::parse("verifying sha256 digest");

    assert_eq!(progress.stage(), "verifying");
    assert_eq!(progress.percent(), None);
    assert_eq!(progress.downloaded_bytes(), None);
    assert_eq!(progress.total_bytes(), None);
    assert_eq!(progress.evidence(), "verifying sha256 digest");
}
