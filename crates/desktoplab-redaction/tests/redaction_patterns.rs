use desktoplab_redaction::{
    is_secret_bearing_path, redact_repository_context, redact_sensitive, redact_sensitive_bounded,
    redact_sensitive_with_status,
};
use xtask::check_logical_line_limit;

#[test]
fn redacts_common_credentials_and_provider_prefixes() {
    let raw = "Authorization: Bearer raw-token Cookie: sessionid=raw api_key=sk-live-secret token=plain secret=hidden provider_key=sk-proj-secret nvapi-key=nvapi-abc ghp_abcd";

    let redacted = redact_sensitive(raw);

    for forbidden in [
        "raw-token",
        "sessionid=raw",
        "sk-live-secret",
        "token=plain",
        "secret=hidden",
        "sk-proj-secret",
        "nvapi-abc",
        "ghp_abcd",
    ] {
        assert!(!redacted.contains(forbidden), "leaked {forbidden}");
    }
    assert!(redacted.contains("[REDACTED]"));
}

#[test]
fn redacts_query_style_and_json_style_secret_values() {
    let raw = r#"{"token":"sk-json-secret","headers":{"Authorization":"Bearer json-bearer"},"url":"https://local.test?api_key=query-secret&safe=true"}"#;

    let redacted = redact_sensitive(raw);

    assert!(!redacted.contains("sk-json-secret"));
    assert!(!redacted.contains("json-bearer"));
    assert!(!redacted.contains("query-secret"));
    assert!(redacted.contains("safe=true"));
}

#[test]
fn redacts_private_key_material_as_a_block() {
    let raw = "before\n-----BEGIN OPENSSH PRIVATE KEY-----\nraw-key-material\n-----END OPENSSH PRIVATE KEY-----\nafter";

    let redacted = redact_sensitive(raw);

    assert!(redacted.contains("before"));
    assert!(redacted.contains("after"));
    assert!(redacted.contains("[REDACTED]"));
    assert!(!redacted.contains("raw-key-material"));
    assert!(!redacted.contains("BEGIN OPENSSH PRIVATE KEY"));
}

#[test]
fn reports_whether_redaction_changed_the_payload() {
    assert!(!redact_sensitive_with_status("plain output").redacted());
    assert!(redact_sensitive_with_status("token=secret").redacted());
}

#[test]
fn preserves_non_secret_whitespace_byte_for_byte() {
    let raw = "# Heading\r\n\r\nFirst paragraph.\n\tindented line\n";

    assert_eq!(redact_sensitive(raw), raw);
}

#[test]
fn preserves_whitespace_around_redacted_material() {
    let raw = "before\n\napi_key=raw-secret\n\tafter\n";

    assert_eq!(
        redact_sensitive(raw),
        "before\n\napi_key=[REDACTED]\n\tafter\n"
    );
}

#[test]
fn private_key_block_redaction_preserves_surrounding_line_endings() {
    let raw = "before\r\n-----BEGIN OPENSSH PRIVATE KEY-----\r\nraw-key\r\n-----END OPENSSH PRIVATE KEY-----\r\nafter\r\n";

    assert_eq!(redact_sensitive(raw), "before\r\n[REDACTED]\r\nafter\r\n");
}

#[test]
fn bounded_redaction_removes_secrets_and_limits_length() {
    let result = redact_sensitive_bounded("safe token=sk-secret trailing output", 18);

    assert!(result.redacted());
    assert!(result.value().chars().count() <= 18);
    assert!(!result.value().contains("sk-secret"));
}

#[test]
fn preserves_existing_redaction_markers() {
    assert_eq!(
        redact_sensitive("API_KEY=[REDACTED_SECRET]"),
        "API_KEY=[REDACTED_SECRET]"
    );
}

#[test]
fn repository_context_preserves_lines_while_redacting_each_line() {
    let result = redact_repository_context("safe line\napi_key=raw-secret\nlast line");

    assert_eq!(result.value().lines().count(), 3);
    assert!(result.redacted());
    assert!(!result.value().contains("raw-secret"));
}

#[test]
fn secret_path_policy_covers_common_repo_and_home_credentials() {
    for path in [
        ".env.local",
        ".aws/credentials",
        ".kube/config",
        ".docker/config.json",
        "certs/client.pem",
        "config/secrets.production",
        ".ssh/id_ed25519",
    ] {
        assert!(is_secret_bearing_path(path), "{path}");
    }
    assert!(!is_secret_bearing_path("src/config.rs"));
}

#[test]
fn redaction_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-redaction/src/lib.rs",
        include_str!("../src/lib.rs"),
        260,
    )
    .expect("redaction engine source should stay below the line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-redaction/src/path_policy.rs",
        include_str!("../src/path_policy.rs"),
        100,
    )
    .expect("secret path policy should stay focused");
}
