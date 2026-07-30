use std::{fs, io, path::Path};

use serde_json::{Value, json};

use crate::LmStudioManagedPlan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LmStudioOwnershipMarker {
    pub(crate) pid: u64,
    pub(crate) endpoint: String,
    pub(crate) model_id: String,
    pub(crate) api_model_id: String,
    pub(crate) artifact_version: String,
    pub(crate) artifact_sha512: String,
}

impl LmStudioOwnershipMarker {
    pub(crate) fn new(plan: &LmStudioManagedPlan, pid: u64) -> Self {
        Self {
            pid,
            endpoint: plan.endpoint(),
            model_id: plan.model_id().to_string(),
            api_model_id: plan.api_model_id().to_string(),
            artifact_version: plan.artifact().version().to_string(),
            artifact_sha512: plan.artifact().sha512().to_string(),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "schemaVersion":1,
            "runtimeId":"runtime.lm-studio",
            "ownership":"desktoplab_managed",
            "pid":self.pid,
            "endpoint":self.endpoint,
            "modelId":self.model_id,
            "apiModelId":self.api_model_id,
            "artifactVersion":self.artifact_version,
            "artifactSha512":self.artifact_sha512
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        if value.get("schemaVersion").and_then(Value::as_u64) != Some(1)
            || value.get("runtimeId").and_then(Value::as_str) != Some("runtime.lm-studio")
            || value.get("ownership").and_then(Value::as_str) != Some("desktoplab_managed")
        {
            return None;
        }
        Some(Self {
            pid: value.get("pid")?.as_u64()?,
            endpoint: value.get("endpoint")?.as_str()?.to_string(),
            model_id: value.get("modelId")?.as_str()?.to_string(),
            api_model_id: value.get("apiModelId")?.as_str()?.to_string(),
            artifact_version: value.get("artifactVersion")?.as_str()?.to_string(),
            artifact_sha512: value.get("artifactSha512")?.as_str()?.to_string(),
        })
    }

    pub(crate) fn matches_plan(&self, plan: &LmStudioManagedPlan) -> bool {
        self.endpoint == plan.endpoint()
            && self.model_id == plan.model_id()
            && self.api_model_id == plan.api_model_id()
            && self.artifact_version == plan.artifact().version()
            && self.artifact_sha512 == plan.artifact().sha512()
    }
}

pub(crate) fn read_marker(path: &Path) -> Result<Option<LmStudioOwnershipMarker>, io::Error> {
    match fs::read(path) {
        Ok(bytes) => {
            let value = serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            LmStudioOwnershipMarker::from_json(&value)
                .map(Some)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid marker"))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn write_marker(path: &Path, marker: &LmStudioOwnershipMarker) -> Result<(), io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "marker parent missing"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join("ownership.json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&marker.to_json()).map_err(io::Error::other)?,
    )?;
    fs::rename(temporary, path)
}

pub(crate) fn clear_marker(path: &Path) -> Result<(), io::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}
