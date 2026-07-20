use desktoplab_model_manager::ModelDownloadError;
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelRouteError {
    CloudProviderRequired,
    InsufficientMemory { required_gb: u32, available_gb: u32 },
    UnknownModel,
    UnknownSetupChoice,
    Download(ModelDownloadError),
}

impl ModelRouteError {
    pub(crate) fn reason(&self) -> &'static str {
        match self {
            Self::UnknownModel => "unknown model",
            Self::CloudProviderRequired => "cloud model requires provider connection",
            Self::InsufficientMemory { .. } => "not enough memory for this model",
            Self::UnknownSetupChoice => "unknown setup choice",
            Self::Download(ModelDownloadError::SetupPlanNotAccepted) => "setup plan not accepted",
            Self::Download(ModelDownloadError::InsufficientDisk { .. }) => "insufficient disk",
            Self::Download(ModelDownloadError::NetworkUnavailable) => "network unavailable",
            Self::Download(ModelDownloadError::ResumeUnsupported) => "resume unsupported",
            Self::Download(ModelDownloadError::UnsupportedRuntime(_)) => "unsupported runtime",
            Self::Download(ModelDownloadError::UnsafeModelReference(_)) => "unsafe model reference",
        }
    }

    pub(crate) fn retry_class(&self) -> &'static str {
        match self {
            Self::CloudProviderRequired => "provider",
            Self::InsufficientMemory { .. } => "user_action",
            Self::Download(ModelDownloadError::NetworkUnavailable) => "offline",
            Self::Download(ModelDownloadError::InsufficientDisk { .. }) => "user_action",
            _ => "non_retryable",
        }
    }

    pub(crate) fn runtime_id(&self) -> Value {
        match self {
            Self::CloudProviderRequired => json!("runtime.ollama-cloud"),
            Self::Download(ModelDownloadError::UnsupportedRuntime(runtime_id)) => json!(runtime_id),
            _ => Value::Null,
        }
    }

    pub(crate) fn disk_details(&self) -> Option<(u64, u64)> {
        match self {
            Self::Download(ModelDownloadError::InsufficientDisk {
                required_mb,
                available_mb,
            }) => Some((*required_mb, *available_mb)),
            _ => None,
        }
    }

    pub(crate) fn memory_details(&self) -> Option<(u32, u32)> {
        match self {
            Self::InsufficientMemory {
                required_gb,
                available_gb,
            } => Some((*required_gb, *available_gb)),
            _ => None,
        }
    }
}
