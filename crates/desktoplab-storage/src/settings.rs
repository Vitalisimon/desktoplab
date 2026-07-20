use crate::{SecretRejected, StorageError};

const SECRET_SETTING_KEYS: &[&str] = &[
    "access_token",
    "api_key",
    "password",
    "private_key",
    "refresh_token",
    "secret",
    "ssh_key",
    "token",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingRecord {
    key: String,
    value: SettingValue,
    updated_at: String,
}

impl SettingRecord {
    #[must_use]
    pub fn new(key: impl Into<String>, value: SettingValue) -> Self {
        Self {
            key: key.into(),
            value,
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }

    #[must_use]
    pub fn from_storage(
        key: impl Into<String>,
        value: SettingValue,
        updated_at: impl Into<String>,
    ) -> Self {
        Self {
            key: key.into(),
            value,
            updated_at: updated_at.into(),
        }
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub fn value(&self) -> &SettingValue {
        &self.value
    }

    pub(crate) fn updated_at(&self) -> &str {
        &self.updated_at
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingValue {
    Boolean(bool),
    Integer(i64),
    Json(String),
    SecretReference(String),
    String(String),
}

impl SettingValue {
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Json(_) => "json",
            Self::SecretReference(_) => "secret_reference",
            Self::String(_) => "string",
        }
    }

    pub(crate) fn raw_value(&self) -> String {
        match self {
            Self::Boolean(value) => value.to_string(),
            Self::Integer(value) => value.to_string(),
            Self::Json(value) | Self::SecretReference(value) | Self::String(value) => {
                value.to_string()
            }
        }
    }

    pub(crate) fn from_storage(kind: &str, value: String) -> Result<Self, StorageError> {
        match kind {
            "boolean" => value
                .parse::<bool>()
                .map(Self::Boolean)
                .map_err(|error| StorageError::Sqlite(error.to_string())),
            "integer" => value
                .parse::<i64>()
                .map(Self::Integer)
                .map_err(|error| StorageError::Sqlite(error.to_string())),
            "json" => Ok(Self::Json(value)),
            "secret_reference" => Ok(Self::SecretReference(value)),
            "string" => Ok(Self::String(value)),
            other => Err(StorageError::Sqlite(format!(
                "unknown setting value kind: {other}"
            ))),
        }
    }
}

pub(crate) fn reject_raw_secret_setting(record: &SettingRecord) -> Result<(), StorageError> {
    if matches!(record.value(), SettingValue::SecretReference(_)) {
        return Ok(());
    }

    let key = record.key().to_ascii_lowercase();
    let key_requires_secret_reference = SECRET_SETTING_KEYS
        .iter()
        .any(|secret_key| key.contains(secret_key));

    if key_requires_secret_reference {
        return Err(StorageError::SecretRejected(SecretRejected::new(
            "settings store accepts secret references only",
        )));
    }

    Ok(())
}
