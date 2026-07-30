use std::path::{Component, Path, PathBuf};

const UV_VERSION: &str = "0.12.0";
const PYTHON_VERSION: &str = "3.14.6";
const MLX_LM_VERSION: &str = "0.31.3";
const LOCK_SHA256: &str = "60b758378744dd31603ccf389ada3b80a3a483d75be2b336dd0b4532d59568c6";
const LOCK_CONTENT: &str = include_str!("../../../runtime-catalog/mlx-lm/darwin-arm64-py314.lock");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MlxLmManagedPhase {
    Detect,
    AcquireBootstrap,
    VerifyBootstrap,
    InstallPython,
    CreateEnvironment,
    SyncLockedPackages,
    VerifyEnvironment,
    AcquireModel,
    StartServer,
    Health,
    PersistOwnership,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MlxLmManagedPlanError {
    UnsupportedPlatform,
    UnsafeManagedRoot,
    UnsafeModelIdentity,
    ModelLicenseNotAccepted,
    UnsafePort,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmManagedArtifact {
    version: &'static str,
    url: &'static str,
    sha256: &'static str,
}

impl MlxLmManagedArtifact {
    #[must_use]
    pub fn version(&self) -> &str {
        self.version
    }

    #[must_use]
    pub fn url(&self) -> &str {
        self.url
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        self.sha256
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmManagedPlan {
    root: PathBuf,
    artifact: MlxLmManagedArtifact,
    model_id: String,
    model_revision: String,
    model_license: String,
    model_storage_bytes: u64,
    port: u16,
    phases: Vec<MlxLmManagedPhase>,
}

impl MlxLmManagedPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        root: impl Into<PathBuf>,
        target: &str,
        model_id: impl Into<String>,
        model_revision: impl Into<String>,
        model_license: impl Into<String>,
        model_storage_bytes: u64,
        port: u16,
        model_license_accepted: bool,
    ) -> Result<Self, MlxLmManagedPlanError> {
        if target != "darwin-arm64" {
            return Err(MlxLmManagedPlanError::UnsupportedPlatform);
        }
        let root = root.into();
        if !safe_managed_root(&root) {
            return Err(MlxLmManagedPlanError::UnsafeManagedRoot);
        }
        let model_id = model_id.into();
        let model_revision = model_revision.into();
        let model_license = model_license.into();
        if !safe_model_id(&model_id)
            || !is_lower_hex_revision(&model_revision)
            || model_license != "apache-2.0"
            || model_storage_bytes == 0
        {
            return Err(MlxLmManagedPlanError::UnsafeModelIdentity);
        }
        if !model_license_accepted {
            return Err(MlxLmManagedPlanError::ModelLicenseNotAccepted);
        }
        if port < 1024 {
            return Err(MlxLmManagedPlanError::UnsafePort);
        }
        Ok(Self {
            root,
            artifact: MlxLmManagedArtifact {
                version: UV_VERSION,
                url: "https://github.com/astral-sh/uv/releases/download/0.12.0/uv-aarch64-apple-darwin.tar.gz",
                sha256: "2b9e582af54f84fa50c115427451a6c13e80f43b52f8282b8af5791077317bbf",
            },
            model_id,
            model_revision,
            model_license,
            model_storage_bytes,
            port,
            phases: vec![
                MlxLmManagedPhase::Detect,
                MlxLmManagedPhase::AcquireBootstrap,
                MlxLmManagedPhase::VerifyBootstrap,
                MlxLmManagedPhase::InstallPython,
                MlxLmManagedPhase::CreateEnvironment,
                MlxLmManagedPhase::SyncLockedPackages,
                MlxLmManagedPhase::VerifyEnvironment,
                MlxLmManagedPhase::AcquireModel,
                MlxLmManagedPhase::StartServer,
                MlxLmManagedPhase::Health,
                MlxLmManagedPhase::PersistOwnership,
            ],
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn artifact(&self) -> &MlxLmManagedArtifact {
        &self.artifact
    }

    #[must_use]
    pub fn archive_path(&self) -> PathBuf {
        self.root.join("cache").join("uv.tar.gz")
    }

    #[must_use]
    pub fn bootstrap_root(&self) -> PathBuf {
        self.root.join("bootstrap")
    }

    #[must_use]
    pub fn uv_path(&self) -> PathBuf {
        self.bootstrap_root()
            .join("uv-aarch64-apple-darwin")
            .join("uv")
    }

    #[must_use]
    pub fn python_root(&self) -> PathBuf {
        self.root.join("python")
    }

    #[must_use]
    pub fn environment_root(&self) -> PathBuf {
        self.root.join("environment")
    }

    #[must_use]
    pub fn python_path(&self) -> PathBuf {
        self.environment_root().join("bin").join("python")
    }

    #[must_use]
    pub fn server_path(&self) -> PathBuf {
        self.environment_root().join("bin").join("mlx_lm.server")
    }

    #[must_use]
    pub fn hf_path(&self) -> PathBuf {
        self.environment_root().join("bin").join("hf")
    }

    #[must_use]
    pub fn lock_path(&self) -> PathBuf {
        self.root.join("environment.lock")
    }

    #[must_use]
    pub fn marker_path(&self) -> PathBuf {
        self.root.join("ownership.json")
    }

    #[must_use]
    pub fn model_root(&self) -> PathBuf {
        self.root.join("models").join("selected")
    }

    #[must_use]
    pub fn cache_root(&self) -> PathBuf {
        self.root.join("cache")
    }

    #[must_use]
    pub fn python_version(&self) -> &'static str {
        PYTHON_VERSION
    }

    #[must_use]
    pub fn mlx_lm_version(&self) -> &'static str {
        MLX_LM_VERSION
    }

    #[must_use]
    pub fn lock_sha256(&self) -> &'static str {
        LOCK_SHA256
    }

    #[must_use]
    pub fn lock_content(&self) -> &'static str {
        LOCK_CONTENT
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn model_revision(&self) -> &str {
        &self.model_revision
    }

    #[must_use]
    pub fn model_license(&self) -> &str {
        &self.model_license
    }

    #[must_use]
    pub fn model_storage_bytes(&self) -> u64 {
        self.model_storage_bytes
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    #[must_use]
    pub fn phases(&self) -> &[MlxLmManagedPhase] {
        &self.phases
    }
}

fn safe_managed_root(root: &Path) -> bool {
    root.is_absolute()
        && !root.components().any(|part| part == Component::ParentDir)
        && root
            .components()
            .any(|part| matches!(part, Component::Normal(name) if name == "DesktopLab"))
        && root.file_name().is_some_and(|name| name == "mlx-lm")
        && root
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "runtimes")
}

fn safe_model_id(value: &str) -> bool {
    value.split_once('/').is_some()
        && !value.contains("..")
        && !value.starts_with('/')
        && !value.ends_with('/')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '/' | '-' | '_' | '.')
        })
}

fn is_lower_hex_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}
