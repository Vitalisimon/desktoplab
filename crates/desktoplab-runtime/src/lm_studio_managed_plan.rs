use std::path::{Component, Path, PathBuf};

const RELEASE: &str = "0.0.20-1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LmStudioManagedPhase {
    Detect,
    Acquire,
    VerifyArtifact,
    Install,
    StartDaemon,
    DownloadModel,
    LoadModel,
    StartServer,
    Health,
    PersistOwnership,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LmStudioManagedPlanError {
    VendorTermsNotAccepted,
    UnsupportedPlatform,
    UnsafeManagedRoot,
    UnsafeModelId,
    UnsafePort,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioManagedArtifact {
    version: &'static str,
    url: String,
    sha512: &'static str,
}

impl LmStudioManagedArtifact {
    #[must_use]
    pub fn version(&self) -> &str {
        self.version
    }

    #[must_use]
    pub fn url(&self) -> &str {
        &self.url
    }

    #[must_use]
    pub fn sha512(&self) -> &str {
        self.sha512
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LmStudioManagedPlan {
    root: PathBuf,
    managed_home: PathBuf,
    cache_archive: PathBuf,
    extract_root: PathBuf,
    lms_path: PathBuf,
    marker_path: PathBuf,
    model_id: String,
    api_model_id: String,
    port: u16,
    artifact: LmStudioManagedArtifact,
    phases: Vec<LmStudioManagedPhase>,
}

impl LmStudioManagedPlan {
    pub fn new(
        root: impl Into<PathBuf>,
        target: &str,
        model_id: impl Into<String>,
        api_model_id: impl Into<String>,
        port: u16,
        vendor_terms_accepted: bool,
    ) -> Result<Self, LmStudioManagedPlanError> {
        if !vendor_terms_accepted {
            return Err(LmStudioManagedPlanError::VendorTermsNotAccepted);
        }
        let root = root.into();
        if !safe_managed_root(&root) {
            return Err(LmStudioManagedPlanError::UnsafeManagedRoot);
        }
        let model_id = model_id.into();
        let api_model_id = api_model_id.into();
        if !safe_identifier(&model_id) || !safe_identifier(&api_model_id) {
            return Err(LmStudioManagedPlanError::UnsafeModelId);
        }
        if port < 1024 {
            return Err(LmStudioManagedPlanError::UnsafePort);
        }
        let artifact = artifact_for(target).ok_or(LmStudioManagedPlanError::UnsupportedPlatform)?;
        let managed_home = root.join("home");
        Ok(Self {
            cache_archive: root.join("cache").join("llmster.tar.gz"),
            extract_root: root.join("bootstrap"),
            lms_path: managed_home.join(".lmstudio").join("bin").join("lms"),
            marker_path: root.join("ownership.json"),
            root,
            managed_home,
            model_id,
            api_model_id,
            port,
            artifact,
            phases: vec![
                LmStudioManagedPhase::Detect,
                LmStudioManagedPhase::Acquire,
                LmStudioManagedPhase::VerifyArtifact,
                LmStudioManagedPhase::Install,
                LmStudioManagedPhase::StartDaemon,
                LmStudioManagedPhase::DownloadModel,
                LmStudioManagedPhase::LoadModel,
                LmStudioManagedPhase::StartServer,
                LmStudioManagedPhase::Health,
                LmStudioManagedPhase::PersistOwnership,
            ],
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn managed_home(&self) -> &Path {
        &self.managed_home
    }

    #[must_use]
    pub fn cache_archive(&self) -> &Path {
        &self.cache_archive
    }

    #[must_use]
    pub fn extract_root(&self) -> &Path {
        &self.extract_root
    }

    #[must_use]
    pub fn bootstrap_path(&self) -> PathBuf {
        self.extract_root.join("llmster")
    }

    #[must_use]
    pub fn lms_path(&self) -> &Path {
        &self.lms_path
    }

    #[must_use]
    pub fn marker_path(&self) -> &Path {
        &self.marker_path
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn api_model_id(&self) -> &str {
        &self.api_model_id
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
    pub fn artifact(&self) -> &LmStudioManagedArtifact {
        &self.artifact
    }

    #[must_use]
    pub fn phases(&self) -> &[LmStudioManagedPhase] {
        &self.phases
    }
}

fn safe_managed_root(root: &Path) -> bool {
    root.is_absolute()
        && !root
            .components()
            .any(|component| component == Component::ParentDir)
        && root
            .components()
            .any(|component| matches!(component, Component::Normal(name) if name == "DesktopLab"))
        && root.file_name().is_some_and(|name| name == "lm-studio")
        && root
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|name| name == "runtimes")
}

fn safe_identifier(value: &str) -> bool {
    !value.is_empty()
        && !value.contains("..")
        && !value.starts_with('/')
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || matches!(character, '/' | '-' | '_' | '.' | '@' | ':')
        })
}

fn artifact_for(target: &str) -> Option<LmStudioManagedArtifact> {
    let (name, sha512) = match target {
        "darwin-arm64" => (
            "0.0.20-1-darwin-arm64.full",
            "4f451b88a79c5e94d79920b8837413450a936bfb8d7687a9e2c201e2f315866a2593aa2d34fe9805174b377cba0013f28030151170e993051b4d7edec1d0bb72",
        ),
        "linux-arm64" => (
            "0.0.20-1-linux-arm64.full",
            "1350c6d5e13c0da4a0f53e3c6a7b1cf67384f4ae3a84804f8d54980f1c30b6b7f87df4b0a5e69013e31fcc8792dec3950e48420a16acaf0a7ee44e9a04731967",
        ),
        "linux-x64" => (
            "0.0.20-1-linux-x64.full",
            "1d1c4faf89bb1529b27a1a0beac871cd0975707719ba663ed1c1d9b7ca0f027300e6c4fa66b338dbe7cd391400118e501198f5b652645163f3b410848e2552b6",
        ),
        _ => return None,
    };
    Some(LmStudioManagedArtifact {
        version: RELEASE,
        url: format!("https://llmster.lmstudio.ai/download/{name}.tar.gz"),
        sha512,
    })
}
