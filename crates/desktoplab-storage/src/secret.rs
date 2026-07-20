use crate::{RedactionStatus, SecretRejected, StorageError};
use serde_json::Value;

const FORBIDDEN_KEYS: &[&str] = &[
    "access_token",
    "api_key",
    "password",
    "private_key",
    "refresh_token",
    "secret",
    "ssh_key",
    "token",
];

pub(crate) fn reject_secret_like_payload(
    payload: &str,
    redaction_status: RedactionStatus,
) -> Result<(), StorageError> {
    let parsed: Value = serde_json::from_str(payload)
        .map_err(|error| StorageError::InvalidJson(error.to_string()))?;

    if contains_forbidden_secret(&parsed, redaction_status) {
        return Err(StorageError::SecretRejected(SecretRejected::new(
            "payload contains forbidden secret-like key",
        )));
    }

    Ok(())
}

fn contains_forbidden_secret(value: &Value, redaction_status: RedactionStatus) -> bool {
    match value {
        Value::Object(entries) => entries.iter().any(|(key, value)| {
            let key_forbidden = FORBIDDEN_KEYS
                .iter()
                .any(|forbidden| key.eq_ignore_ascii_case(forbidden));

            key_forbidden && !is_allowed_redacted_value(value, redaction_status)
                || contains_forbidden_secret(value, redaction_status)
        }),
        Value::Array(items) => items
            .iter()
            .any(|item| contains_forbidden_secret(item, redaction_status)),
        _ => false,
    }
}

fn is_allowed_redacted_value(value: &Value, redaction_status: RedactionStatus) -> bool {
    matches!(
        (redaction_status, value),
        (RedactionStatus::Redacted, Value::String(text)) if text == "[REDACTED]"
    )
}
