use std::io::{Cursor, Write};
use std::time::Duration;

use desktoplab_storage::{
    ArchiveFormat, ArchiveImportError, ArchiveLimits, AtomicFileStore, CrossProcessLock, LockError,
    PrivateFileMode, import_archive,
};
use tempfile::TempDir;

#[test]
fn interrupted_atomic_writer_preserves_previous_complete_state() {
    let fixture = TempDir::new().unwrap();
    let path = fixture.path().join("state.json");
    AtomicFileStore::replace(&path, br#"{"state":"old"}"#, PrivateFileMode::OwnerOnly).unwrap();

    let result = AtomicFileStore::replace_with(&path, PrivateFileMode::OwnerOnly, |file| {
        file.write_all(br#"{"state":"partial"#)?;
        Err(std::io::Error::other("simulated_crash_before_persist"))
    });

    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), br#"{"state":"old"}"#);
    AtomicFileStore::replace(&path, br#"{"state":"new"}"#, PrivateFileMode::OwnerOnly).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), br#"{"state":"new"}"#);
}

#[test]
fn cross_process_lock_fails_busy_without_deleting_owner_file() {
    let fixture = TempDir::new().unwrap();
    let path = fixture.path().join("state.lock");
    let owner = CrossProcessLock::acquire(&path, 1, Duration::ZERO).unwrap();

    assert!(matches!(
        CrossProcessLock::acquire(&path, 2, Duration::from_millis(1)),
        Err(LockError::Busy)
    ));
    assert_eq!(owner.path(), path);
    assert!(path.exists());
    drop(owner);
    assert!(std::fs::read_to_string(path).unwrap().starts_with("pid="));
}

#[test]
#[cfg(unix)]
fn private_atomic_state_is_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TempDir::new().unwrap();
    let path = fixture.path().join("private.json");
    AtomicFileStore::replace(&path, b"{}", PrivateFileMode::OwnerOnly).unwrap();
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
#[cfg(windows)]
fn private_atomic_state_removes_inherited_broad_windows_acl() {
    use std::process::Command;

    let fixture = TempDir::new().unwrap();
    let inherited = Command::new("icacls.exe")
        .arg(fixture.path())
        .args(["/grant", "*S-1-1-0:(OI)(CI)(R)"])
        .status()
        .unwrap();
    assert!(inherited.success());
    let path = fixture.path().join("private.json");
    AtomicFileStore::replace(&path, b"{}", PrivateFileMode::OwnerOnly).unwrap();

    let script = r#"
$forbidden = @('S-1-1-0', 'S-1-5-11', 'S-1-5-32-545')
$entries = (Get-Acl -LiteralPath $env:DESKTOPLAB_ACL_TEST_PATH).Access
if ($null -eq $entries -or $entries.Count -eq 0) { exit 4 }
foreach ($entry in $entries) {
  try {
    $sid = $entry.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
  } catch { exit 5 }
  if ($forbidden -contains $sid) { exit 3 }
}
exit 0
"#;
    let inspected = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .env("DESKTOPLAB_ACL_TEST_PATH", path)
        .status()
        .unwrap();
    assert!(inspected.success());
}

#[test]
fn zip_import_is_atomic_and_bounded() {
    let fixture = TempDir::new().unwrap();
    let bytes = zip_bytes(&[("docs/readme.md", b"safe")]);
    let destination = fixture.path().join("imported");
    let report = import_archive(
        Cursor::new(bytes),
        ArchiveFormat::Zip,
        &destination,
        ArchiveLimits::default(),
    )
    .unwrap();
    assert_eq!(report.entries, 1);
    assert_eq!(
        std::fs::read(destination.join("docs/readme.md")).unwrap(),
        b"safe"
    );
}

#[test]
fn traversal_and_entry_limit_archives_fail_without_destination() {
    let fixture = TempDir::new().unwrap();
    let destination = fixture.path().join("blocked");
    let traversal = zip_bytes(&[("../escape.txt", b"escape")]);
    assert!(matches!(
        import_archive(
            Cursor::new(traversal),
            ArchiveFormat::Zip,
            &destination,
            ArchiveLimits::default()
        ),
        Err(ArchiveImportError::InvalidEntry)
    ));
    assert!(!destination.exists());

    let limited = ArchiveLimits {
        max_entries: 1,
        ..ArchiveLimits::default()
    };
    let oversized = zip_bytes(&[("one.txt", b"1"), ("two.txt", b"2")]);
    assert!(matches!(
        import_archive(
            Cursor::new(oversized),
            ArchiveFormat::Zip,
            &destination,
            limited
        ),
        Err(ArchiveImportError::LimitExceeded)
    ));
    assert!(!destination.exists());
}

#[test]
fn zip_expansion_ratio_is_bounded() {
    let fixture = TempDir::new().unwrap();
    let destination = fixture.path().join("zip-bomb");
    let repeated = vec![b'a'; 32 * 1024];
    let bytes = zip_bytes(&[("repeated.txt", &repeated)]);
    let limits = ArchiveLimits {
        max_expansion_ratio: 2,
        ..ArchiveLimits::default()
    };

    assert!(matches!(
        import_archive(Cursor::new(bytes), ArchiveFormat::Zip, &destination, limits),
        Err(ArchiveImportError::LimitExceeded)
    ));
    assert!(!destination.exists());
}

#[test]
fn tar_link_entry_fails_closed() {
    let fixture = TempDir::new().unwrap();
    let destination = fixture.path().join("blocked-tar");
    let mut bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut bytes);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        header.set_cksum();
        builder
            .append_link(&mut header, "link", "../outside")
            .unwrap();
        builder.finish().unwrap();
    }
    assert!(matches!(
        import_archive(
            Cursor::new(bytes),
            ArchiveFormat::Tar,
            &destination,
            ArchiveLimits::default()
        ),
        Err(ArchiveImportError::LinkEntry)
    ));
    assert!(!destination.exists());
}

#[test]
fn durable_state_sources_stay_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-storage/src/atomic_file.rs",
        include_str!("../src/atomic_file.rs"),
        240,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-storage/src/archive.rs",
        include_str!("../src/archive.rs"),
        280,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-storage/src/windows_acl.rs",
        include_str!("../src/windows_acl.rs"),
        100,
    )
    .unwrap();
}

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        for (name, contents) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }
    cursor.into_inner()
}
