use crate::LocalAuthToken;
use std::fmt;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalApiDiscoveryDocument {
    base_url: String,
    pid: u32,
    created_at: u64,
    token_redacted: &'static str,
    schema_version: u16,
}

impl LocalApiDiscoveryDocument {
    pub fn new(
        base_url: impl Into<String>,
        pid: u32,
        created_at: u64,
        token: &LocalAuthToken,
    ) -> Result<Self, DiscoveryError> {
        let base_url = base_url.into();
        if !base_url.starts_with("http://127.0.0.1:") && !base_url.starts_with("http://localhost:")
        {
            return Err(DiscoveryError::InvalidBaseUrl);
        }

        Ok(Self {
            base_url,
            pid,
            created_at,
            token_redacted: token.redacted(),
            schema_version: SCHEMA_VERSION,
        })
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[must_use]
    pub const fn pid(&self) -> u32 {
        self.pid
    }

    #[must_use]
    pub const fn created_at(&self) -> u64 {
        self.created_at
    }

    #[must_use]
    pub const fn token_redacted(&self) -> &'static str {
        self.token_redacted
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    #[must_use]
    pub fn process_state(&self, is_running: impl FnOnce(u32) -> bool) -> DiscoveryProcessState {
        if is_running(self.pid) {
            DiscoveryProcessState::Running
        } else {
            DiscoveryProcessState::Stale
        }
    }

    #[must_use]
    pub fn to_json(&self) -> String {
        format!(
            r#"{{"schemaVersion":{},"baseUrl":"{}","pid":{},"createdAt":{},"tokenRedacted":"{}"}}"#,
            self.schema_version, self.base_url, self.pid, self.created_at, self.token_redacted
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryProcessState {
    Running,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalApiDiscoveryPath {
    path: PathBuf,
}

impl LocalApiDiscoveryPath {
    #[must_use]
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_user_home(home: &Path, app_name: &str) -> Result<Self, DiscoveryError> {
        let app_name = app_name.trim();
        if app_name.is_empty() {
            return Err(DiscoveryError::InvalidAppName);
        }

        Ok(Self {
            path: home
                .join(".config")
                .join(app_name)
                .join("local-api-discovery.json"),
        })
    }

    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.path
    }
}

pub struct LocalApiDiscoveryWriter;

impl LocalApiDiscoveryWriter {
    pub fn write(
        path: &LocalApiDiscoveryPath,
        document: &LocalApiDiscoveryDocument,
    ) -> Result<(), DiscoveryError> {
        let parent = path
            .as_path()
            .parent()
            .ok_or(DiscoveryError::MissingParent)?;
        std::fs::create_dir_all(parent).map_err(|error| DiscoveryError::Io(error.to_string()))?;
        write_user_only(path.as_path(), document.to_json().as_bytes())?;
        Ok(())
    }

    pub fn verify_permissions(
        path: &LocalApiDiscoveryPath,
    ) -> Result<DiscoveryPermissionState, DiscoveryError> {
        verify_user_only(path.as_path())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiscoveryPermissionState {
    UserOnly,
    Unsafe,
    VerificationUnavailable,
}

impl DiscoveryPermissionState {
    #[must_use]
    pub const fn allows_packaged_bootstrap(self) -> bool {
        matches!(self, Self::UserOnly | Self::VerificationUnavailable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DiscoveryError {
    InvalidAppName,
    InvalidBaseUrl,
    MissingParent,
    Io(String),
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAppName => write!(formatter, "invalid app name"),
            Self::InvalidBaseUrl => write!(formatter, "invalid local api base url"),
            Self::MissingParent => write!(formatter, "discovery path is missing parent"),
            Self::Io(error) => write!(formatter, "io error: {error}"),
        }
    }
}

impl std::error::Error for DiscoveryError {}

fn write_user_only(path: &Path, bytes: &[u8]) -> Result<(), DiscoveryError> {
    desktoplab_storage::AtomicFileStore::replace(
        path,
        bytes,
        desktoplab_storage::PrivateFileMode::OwnerOnly,
    )
    .map_err(|error| DiscoveryError::Io(error.to_string()))
}

#[cfg(unix)]
fn verify_user_only(path: &Path) -> Result<DiscoveryPermissionState, DiscoveryError> {
    use std::os::unix::fs::PermissionsExt;

    let mode = std::fs::metadata(path)
        .map_err(|error| DiscoveryError::Io(error.to_string()))?
        .permissions()
        .mode()
        & 0o777;
    if mode == 0o600 {
        Ok(DiscoveryPermissionState::UserOnly)
    } else {
        Ok(DiscoveryPermissionState::Unsafe)
    }
}

#[cfg(not(unix))]
fn verify_user_only(_path: &Path) -> Result<DiscoveryPermissionState, DiscoveryError> {
    Ok(DiscoveryPermissionState::VerificationUnavailable)
}
