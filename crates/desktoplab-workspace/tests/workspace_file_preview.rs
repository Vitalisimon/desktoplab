use desktoplab_workspace::{FilePreview, FilePreviewLimits, FilePreviewState};
use std::fs;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn protected_file_preview_is_denied_before_content_is_returned() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join(".env"), "API_KEY=secret\n").unwrap();

    let preview = FilePreview::read(repo.path(), ".env", FilePreviewLimits::new(1024, 40))
        .expect("preview request should return denied state");

    assert_eq!(preview.state(), FilePreviewState::Denied);
    assert_eq!(preview.denied_reason(), Some("local_only_path"));
    assert_eq!(preview.text(), None);
}

#[test]
fn nested_protected_file_preview_is_denied_before_content_is_returned() {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("config/.ssh")).unwrap();
    fs::write(repo.path().join("config/.ssh/id_ed25519"), "PRIVATE KEY\n").unwrap();

    let preview = FilePreview::read(
        repo.path(),
        "config/.ssh/id_ed25519",
        FilePreviewLimits::new(1024, 40),
    )
    .expect("preview request should return denied state");

    assert_eq!(preview.state(), FilePreviewState::Denied);
    assert_eq!(preview.denied_reason(), Some("local_only_path"));
    assert_eq!(preview.text(), None);
}

#[test]
fn binary_file_preview_returns_metadata_without_raw_bytes() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("image.bin"), b"\0PNG\r\n").unwrap();

    let preview = FilePreview::read(repo.path(), "image.bin", FilePreviewLimits::new(1024, 40))
        .expect("binary preview should read metadata");

    assert_eq!(preview.state(), FilePreviewState::Binary);
    assert_eq!(preview.text(), None);
    assert_eq!(preview.original_bytes(), 6);
}

#[test]
fn large_text_preview_is_truncated_with_line_and_byte_metadata() {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("large.txt"), "line1\nline2\nline3\n").unwrap();

    let preview = FilePreview::read(repo.path(), "large.txt", FilePreviewLimits::new(11, 2))
        .expect("text preview should read");

    assert_eq!(preview.state(), FilePreviewState::Text);
    assert_eq!(preview.text(), Some("line1\nline2"));
    assert!(preview.is_truncated());
    assert_eq!(preview.original_lines(), 3);
    assert_eq!(preview.returned_lines(), 2);
}

#[test]
fn text_preview_redacts_token_like_content() {
    let repo = TempDir::new().unwrap();
    fs::write(
        repo.path().join("config.txt"),
        "OPENAI_API_KEY=sk-test-secret\nnormal=true\n",
    )
    .unwrap();

    let preview = FilePreview::read(repo.path(), "config.txt", FilePreviewLimits::new(1024, 40))
        .expect("text preview should read");

    let text = preview.text().expect("text should be returned");
    assert!(text.contains("OPENAI_API_KEY=[REDACTED_SECRET]"));
    assert!(text.contains("normal=true"));
    assert!(!text.contains("sk-test-secret"));
}

#[test]
fn file_preview_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-workspace/src/file_preview.rs",
        include_str!("../src/file_preview.rs"),
        260,
    )
    .expect("workspace file preview source should stay below the line-count guard");
}
