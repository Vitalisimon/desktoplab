use desktoplab_workspace::{FilePreview, FilePreviewLimits, FilePreviewState};
use std::fs;
use tempfile::TempDir;

#[cfg(unix)]
use std::os::unix::fs::symlink;

#[test]
#[cfg(unix)]
fn file_preview_denies_symlink_that_escapes_workspace() {
    let fixture = TempDir::new().unwrap();
    let workspace = fixture.path().join("workspace");
    let outside = fixture.path().join("outside");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.txt"), "API_KEY=sk-live-secret").unwrap();
    symlink(
        outside.join("secret.txt"),
        workspace.join("linked-secret.txt"),
    )
    .unwrap();

    let preview = FilePreview::read(
        &workspace,
        "linked-secret.txt",
        FilePreviewLimits::new(1024, 40),
    )
    .expect("escaped symlink should return a denied preview");

    assert_eq!(preview.state(), FilePreviewState::Denied);
    assert_eq!(preview.denied_reason(), Some("path_escape"));
    assert_eq!(preview.text(), None);
}
