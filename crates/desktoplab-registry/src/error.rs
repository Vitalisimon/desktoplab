#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    InvalidManifest(String),
    InvalidJson(String),
    NoSafeCatalog(String),
    SignatureRejected(String),
    SourceUnavailable(String),
    StorageUnavailable(String),
}

impl From<serde_json::Error> for RegistryError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidJson(error.to_string())
    }
}

impl From<desktoplab_storage::StorageError> for RegistryError {
    fn from(error: desktoplab_storage::StorageError) -> Self {
        Self::StorageUnavailable(error.to_string())
    }
}
