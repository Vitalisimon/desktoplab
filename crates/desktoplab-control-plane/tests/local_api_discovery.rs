use desktoplab_control_plane::{
    DiscoveryProcessState, LocalApiDiscoveryDocument, LocalApiDiscoveryPath,
    LocalApiDiscoveryWriter, LocalAuthToken,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn discovery_document_redacts_token_and_records_packaged_api_location() {
    let token = LocalAuthToken::explicit_for_test("raw-secret-token");
    let document =
        LocalApiDiscoveryDocument::new("http://127.0.0.1:48123", 42, 1_719_000_000, &token)
            .expect("discovery document should be valid");

    let serialized = document.to_json();

    assert!(serialized.contains(r#""schemaVersion":1"#));
    assert!(serialized.contains(r#""baseUrl":"http://127.0.0.1:48123""#));
    assert!(serialized.contains(r#""pid":42"#));
    assert!(serialized.contains(r#""createdAt":1719000000"#));
    assert!(serialized.contains(r#""tokenRedacted":"[REDACTED_LOCAL_API_TOKEN]""#));
    assert!(!serialized.contains("raw-secret-token"));
}

#[test]
fn discovery_path_is_per_user_and_per_app_not_repository_local() {
    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "DesktopLab")
        .expect("discovery path should build");

    assert!(path.as_path().starts_with(home.path()));
    assert!(
        path.as_path()
            .ends_with("DesktopLab/local-api-discovery.json")
    );
    assert!(
        !path
            .as_path()
            .ends_with(".desktoplab/local-api-discovery.json")
    );
}

#[test]
fn stale_process_ids_are_detected_without_reading_token_material() {
    let token = LocalAuthToken::explicit_for_test("raw-secret-token");
    let document = LocalApiDiscoveryDocument::new(
        "http://127.0.0.1:48123",
        current_process_id(),
        1_719_000_000,
        &token,
    )
    .expect("discovery document should be valid");
    assert_eq!(
        document.process_state(|pid| pid == current_process_id()),
        DiscoveryProcessState::Running
    );

    let stale = LocalApiDiscoveryDocument::new(
        "http://127.0.0.1:48123",
        current_process_id() + 100_000,
        1_719_000_000,
        &token,
    )
    .expect("discovery document should be valid");
    assert_eq!(
        stale.process_state(|pid| pid == current_process_id()),
        DiscoveryProcessState::Stale
    );
}

#[test]
fn discovery_writer_persists_document_without_raw_token() {
    let home = TempDir::new().expect("home fixture should exist");
    let path = LocalApiDiscoveryPath::for_user_home(home.path(), "DesktopLab")
        .expect("discovery path should build");
    let token = LocalAuthToken::explicit_for_test("raw-secret-token");
    let document =
        LocalApiDiscoveryDocument::new("http://127.0.0.1:48123", 42, 1_719_000_000, &token)
            .expect("discovery document should be valid");

    LocalApiDiscoveryWriter::write(&path, &document).expect("discovery should write");
    let persisted = std::fs::read_to_string(path.as_path()).expect("discovery should persist");

    assert!(persisted.contains(r#""tokenRedacted":"[REDACTED_LOCAL_API_TOKEN]""#));
    assert!(!persisted.contains("raw-secret-token"));
}

#[test]
fn discovery_source_stays_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/discovery.rs",
        include_str!("../src/discovery.rs"),
        260,
    )
    .expect("discovery source should stay below the initial line-count guard");
}

fn current_process_id() -> u32 {
    std::process::id()
}
