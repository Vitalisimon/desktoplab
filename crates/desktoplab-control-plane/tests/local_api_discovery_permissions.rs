use desktoplab_control_plane::{
    DiscoveryPermissionState, LocalApiDiscoveryDocument, LocalApiDiscoveryPath,
    LocalApiDiscoveryWriter, LocalAuthToken,
};
use tempfile::TempDir;

#[test]
#[cfg(unix)]
fn unix_discovery_files_are_written_user_only() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "DesktopLab")
        .expect("discovery path should build");
    let token = LocalAuthToken::explicit_for_test("raw-secret-token");
    let document =
        LocalApiDiscoveryDocument::new("http://127.0.0.1:48123", 42, 1_719_000_000, &token)
            .expect("discovery document should be valid");

    LocalApiDiscoveryWriter::write(&path, &document).expect("discovery should write");

    let mode = std::fs::metadata(path.as_path())
        .expect("metadata should read")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    assert_eq!(
        LocalApiDiscoveryWriter::verify_permissions(&path).expect("permissions should verify"),
        DiscoveryPermissionState::UserOnly
    );
}

#[test]
#[cfg(unix)]
fn unsafe_discovery_permissions_block_packaged_bootstrap() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "DesktopLab")
        .expect("discovery path should build");
    let token = LocalAuthToken::explicit_for_test("raw-secret-token");
    let document =
        LocalApiDiscoveryDocument::new("http://127.0.0.1:48123", 42, 1_719_000_000, &token)
            .expect("discovery document should be valid");
    LocalApiDiscoveryWriter::write(&path, &document).expect("discovery should write");
    std::fs::set_permissions(path.as_path(), std::fs::Permissions::from_mode(0o644))
        .expect("permissions should change");

    assert_eq!(
        LocalApiDiscoveryWriter::verify_permissions(&path).expect("permissions should verify"),
        DiscoveryPermissionState::Unsafe
    );
    assert!(
        !LocalApiDiscoveryWriter::verify_permissions(&path)
            .expect("permissions should verify")
            .allows_packaged_bootstrap()
    );
}

#[test]
#[cfg(unix)]
fn linux_packaged_discovery_uses_user_scoped_config_path() {
    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "desktoplab")
        .expect("discovery path should build");

    assert_eq!(
        path.as_path(),
        home.path()
            .join(".config")
            .join("desktoplab")
            .join("local-api-discovery.json")
    );
}

#[test]
#[cfg(not(unix))]
fn unsupported_permission_verification_is_explicit() {
    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "DesktopLab")
        .expect("discovery path should build");

    assert_eq!(
        LocalApiDiscoveryWriter::verify_permissions(&path).expect("permissions should verify"),
        DiscoveryPermissionState::VerificationUnavailable
    );
}
