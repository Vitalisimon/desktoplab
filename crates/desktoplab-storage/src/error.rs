use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretRejected {
    message: String,
}

impl SecretRejected {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageError {
    InvalidJson(String),
    SecretRejected(SecretRejected),
    Sqlite(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => write!(formatter, "INVALID_JSON: {message}"),
            Self::SecretRejected(error) => {
                write!(formatter, "SECRET_REJECTED: {}", error.message())
            }
            Self::Sqlite(message) => write!(formatter, "SQLITE: {message}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error.to_string())
    }
}
