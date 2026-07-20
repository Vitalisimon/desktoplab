use desktoplab_control_plane::{
    ControlPlane, ControlPlaneHttpServer, DiscoveryError, HttpServerConfig, HttpServerError,
    HttpServerHandle, LocalApiAuth, LocalApiDiscoveryDocument, LocalApiDiscoveryPath,
    LocalApiDiscoveryWriter, LocalApiRouter, LocalAuthToken, VersionInfo,
};
use std::fmt;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackagedLocalApiConfig {
    port: u16,
    app_data_dir: Option<PathBuf>,
    discovery_path: Option<PathBuf>,
    managed_runtime_owner_id: Option<String>,
    shutdown_managed_ollama: bool,
    shutdown_evidence_path_for_test: Option<PathBuf>,
}

impl PackagedLocalApiConfig {
    #[must_use]
    pub fn random_loopback() -> Self {
        Self {
            port: 0,
            app_data_dir: None,
            discovery_path: None,
            managed_runtime_owner_id: None,
            shutdown_managed_ollama: false,
            shutdown_evidence_path_for_test: None,
        }
    }

    pub fn for_user_home(home: &Path) -> Result<Self, DiscoveryError> {
        let discovery = LocalApiDiscoveryPath::for_user_home(home, "desktoplab")?;
        let app_data_dir = discovery
            .as_path()
            .parent()
            .ok_or(DiscoveryError::MissingParent)?
            .to_path_buf();

        Ok(Self::random_loopback()
            .with_app_data_dir(app_data_dir)
            .with_discovery_path(discovery.as_path()))
    }

    #[must_use]
    pub fn for_app_data_dir(app_data_dir: impl AsRef<Path>) -> Self {
        let app_data_dir = app_data_dir.as_ref();
        Self::random_loopback()
            .with_app_data_dir(app_data_dir)
            .with_discovery_path(app_data_dir.join("local-api-discovery.json"))
    }

    #[must_use]
    pub fn explicit_dev_port(port: u16) -> Self {
        Self {
            port,
            app_data_dir: None,
            discovery_path: None,
            managed_runtime_owner_id: None,
            shutdown_managed_ollama: false,
            shutdown_evidence_path_for_test: None,
        }
    }

    #[must_use]
    pub fn with_app_data_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.app_data_dir = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn with_discovery_path(mut self, path: impl AsRef<Path>) -> Self {
        self.discovery_path = Some(path.as_ref().to_path_buf());
        self
    }

    #[must_use]
    pub fn with_managed_runtime_owner_id(mut self, owner_id: impl Into<String>) -> Self {
        self.managed_runtime_owner_id = Some(owner_id.into());
        self
    }

    #[must_use]
    pub const fn with_managed_ollama_shutdown(mut self, enabled: bool) -> Self {
        self.shutdown_managed_ollama = enabled;
        self
    }

    #[must_use]
    pub fn with_shutdown_evidence_path_for_test(mut self, path: impl AsRef<Path>) -> Self {
        self.shutdown_evidence_path_for_test = Some(path.as_ref().to_path_buf());
        self
    }
}

pub struct PackagedLocalApi {
    bound_addr: SocketAddr,
    base_url: String,
    auth_token: LocalAuthToken,
    discovery_path: Option<PathBuf>,
    shutdown_managed_ollama: bool,
    managed_runtime_owner_id: Option<String>,
    managed_ollama_marker_path: Option<PathBuf>,
    shutdown_evidence_path_for_test: Option<PathBuf>,
    handle: Option<HttpServerHandle>,
}

impl PackagedLocalApi {
    pub fn start(config: PackagedLocalApiConfig) -> Result<Self, PackagedLocalApiError> {
        let auth_token = LocalAuthToken::for_desktop_session();
        let control_plane = Arc::new(Mutex::new(ControlPlane::new(VersionInfo::new(
            "0.1.0", "v1",
        ))));
        control_plane
            .lock()
            .expect("control plane lock should not be poisoned")
            .mark_ready();
        let managed_runtime_owner_id = config.app_data_dir.as_ref().map(|_| {
            config
                .managed_runtime_owner_id
                .clone()
                .unwrap_or_else(|| LocalAuthToken::for_desktop_session().as_str().to_string())
        });
        let router = router_for_config(&config, managed_runtime_owner_id.as_deref())?;
        let server = ControlPlaneHttpServer::bind_with_router(
            HttpServerConfig::loopback(config.port)
                .map_err(PackagedLocalApiError::Http)?
                .with_auth(LocalApiAuth::required(auth_token.clone())),
            control_plane,
            router,
        )
        .map_err(PackagedLocalApiError::Http)?;
        let bound_addr = server.local_addr();
        let base_url = format!("http://{}", bound_addr);
        if let Some(discovery_path) = &config.discovery_path {
            let discovery = LocalApiDiscoveryPath::from_path(discovery_path);
            let document =
                LocalApiDiscoveryDocument::new(&base_url, std::process::id(), 0, &auth_token)
                    .map_err(PackagedLocalApiError::Discovery)?;
            LocalApiDiscoveryWriter::write(&discovery, &document)
                .map_err(PackagedLocalApiError::Discovery)?;
        }
        let handle = server.spawn();

        Ok(Self {
            bound_addr,
            base_url,
            auth_token,
            discovery_path: config.discovery_path,
            shutdown_managed_ollama: config.shutdown_managed_ollama,
            managed_runtime_owner_id,
            managed_ollama_marker_path: config
                .app_data_dir
                .as_ref()
                .map(managed_ollama_marker_path),
            shutdown_evidence_path_for_test: config.shutdown_evidence_path_for_test,
            handle: Some(handle),
        })
    }

    #[must_use]
    pub const fn bound_addr(&self) -> SocketAddr {
        self.bound_addr
    }

    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    #[must_use]
    pub fn auth_token(&self) -> &str {
        self.auth_token.as_str()
    }

    pub fn shutdown(mut self) -> Result<(), PackagedLocalApiError> {
        if let Some(handle) = self.handle.take() {
            handle.shutdown().map_err(PackagedLocalApiError::Http)?;
        }
        self.remove_discovery()?;
        self.shutdown_managed_runtimes()?;
        Ok(())
    }

    fn remove_discovery(&mut self) -> Result<(), PackagedLocalApiError> {
        let Some(path) = self.discovery_path.take() else {
            return Ok(());
        };
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(PackagedLocalApiError::Io(error.to_string())),
        }
    }

    fn shutdown_managed_runtimes(&mut self) -> Result<(), PackagedLocalApiError> {
        if !std::mem::take(&mut self.shutdown_managed_ollama) {
            return Ok(());
        }
        let Some(marker_path) = &self.managed_ollama_marker_path else {
            return Ok(());
        };
        let Some(owner_id) = &self.managed_runtime_owner_id else {
            return Ok(());
        };
        if !runtime_marker_matches(marker_path, owner_id) {
            return Ok(());
        }
        if let Some(path) = self.shutdown_evidence_path_for_test.take() {
            std::fs::write(path, "ollama shutdown requested\n")
                .map_err(|error| PackagedLocalApiError::Io(error.to_string()))?;
        } else {
            quit_ollama();
        }
        match std::fs::remove_file(marker_path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(PackagedLocalApiError::Io(error.to_string())),
        }
        Ok(())
    }
}

impl Drop for PackagedLocalApi {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.shutdown();
        }
        let _ = self.remove_discovery();
        let _ = self.shutdown_managed_runtimes();
    }
}

#[cfg(target_os = "macos")]
fn quit_ollama() {
    let _ = Command::new("osascript")
        .args(["-e", r#"tell application "Ollama" to quit"#])
        .output();
    let _ = Command::new("pkill").args(["-x", "ollama"]).output();
    let _ = Command::new("pkill").args(["-x", "Ollama"]).output();
}

#[cfg(target_os = "linux")]
fn quit_ollama() {
    let _ = Command::new("pkill").args(["-x", "ollama"]).output();
}

#[cfg(target_os = "windows")]
fn quit_ollama() {
    let _ = Command::new("taskkill")
        .args(["/IM", "ollama.exe", "/T"])
        .output();
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn quit_ollama() {}

#[derive(Debug)]
pub enum PackagedLocalApiError {
    Discovery(DiscoveryError),
    Http(HttpServerError),
    Io(String),
    Storage(String),
}

impl fmt::Display for PackagedLocalApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Discovery(error) => write!(formatter, "{error}"),
            Self::Http(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "io error: {error}"),
            Self::Storage(error) => write!(formatter, "{error}"),
        }
    }
}

impl std::error::Error for PackagedLocalApiError {}

fn router_for_config(
    config: &PackagedLocalApiConfig,
    managed_runtime_owner_id: Option<&str>,
) -> Result<LocalApiRouter, PackagedLocalApiError> {
    let Some(app_data_dir) = &config.app_data_dir else {
        return Ok(LocalApiRouter::default());
    };
    std::fs::create_dir_all(app_data_dir)
        .map_err(|error| PackagedLocalApiError::Io(error.to_string()))?;
    LocalApiRouter::with_storage_path(desktoplab_storage_path(app_data_dir))
        .map(|router| {
            let router = router.with_openai_codex_bridge_dir(provider_bridge_dir(app_data_dir));
            match managed_runtime_owner_id {
                Some(owner_id) => router.with_managed_runtime_ownership(
                    managed_ollama_marker_path(app_data_dir),
                    owner_id,
                ),
                None => router,
            }
        })
        .map_err(|error| PackagedLocalApiError::Storage(error.to_string()))
}

fn runtime_marker_matches(path: &Path, owner_id: &str) -> bool {
    std::fs::read_to_string(path).is_ok_and(|marker| marker.trim() == owner_id)
}

fn desktoplab_storage_path(app_data_dir: impl AsRef<Path>) -> PathBuf {
    app_data_dir.as_ref().join("desktoplab.sqlite")
}

fn managed_ollama_marker_path(app_data_dir: impl AsRef<Path>) -> PathBuf {
    app_data_dir
        .as_ref()
        .join("runtime")
        .join("ollama-owned-by-desktoplab")
}

fn provider_bridge_dir(app_data_dir: impl AsRef<Path>) -> PathBuf {
    app_data_dir.as_ref().join("provider-bridges")
}
