use desktoplab_redaction::redact_sensitive;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "format", content = "value", rename_all = "snake_case")]
pub(super) enum BackendEventPayload {
    Json(Value),
    Text(String),
}

impl BackendEventPayload {
    pub(super) fn from_text(value: impl Into<String>) -> Self {
        let redacted = redact_sensitive(&value.into());
        serde_json::from_str(&redacted)
            .map(Self::Json)
            .unwrap_or_else(|_| Self::Text(redacted))
    }

    pub(super) fn from_json(value: Value) -> Self {
        Self::Json(redact_json(value))
    }

    pub(super) fn as_text(&self) -> String {
        match self {
            Self::Json(value) => value.to_string(),
            Self::Text(value) => value.clone(),
        }
    }
}

fn redact_json(value: Value) -> Value {
    match value {
        Value::String(value) => Value::String(redact_sensitive(&value)),
        Value::Array(values) => Value::Array(values.into_iter().map(redact_json).collect()),
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, redact_json(value)))
                .collect(),
        ),
        scalar => scalar,
    }
}
