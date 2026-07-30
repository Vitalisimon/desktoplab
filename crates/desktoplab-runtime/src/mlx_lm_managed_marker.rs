use std::{fs, io, path::Path};

use serde_json::{Value, json};

use crate::MlxLmManagedPlan;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MlxLmOwnershipMarker {
    pub(crate) pid: u32,
    pub(crate) endpoint: String,
    pub(crate) model_id: String,
    pub(crate) model_revision: String,
    pub(crate) python_version: String,
    pub(crate) mlx_lm_version: String,
    pub(crate) lock_sha256: String,
}

impl MlxLmOwnershipMarker {
    pub(crate) fn new(plan: &MlxLmManagedPlan, pid: u32) -> Self {
        Self {
            pid,
            endpoint: plan.endpoint(),
            model_id: plan.model_id().to_string(),
            model_revision: plan.model_revision().to_string(),
            python_version: plan.python_version().to_string(),
            mlx_lm_version: plan.mlx_lm_version().to_string(),
            lock_sha256: plan.lock_sha256().to_string(),
        }
    }

    pub(crate) fn matches_plan(&self, plan: &MlxLmManagedPlan) -> bool {
        self.endpoint == plan.endpoint()
            && self.model_id == plan.model_id()
            && self.model_revision == plan.model_revision()
            && self.python_version == plan.python_version()
            && self.mlx_lm_version == plan.mlx_lm_version()
            && self.lock_sha256 == plan.lock_sha256()
    }

    fn to_json(&self) -> Value {
        json!({
            "schemaVersion":1,
            "runtimeId":"runtime.mlx-lm",
            "ownership":"desktoplab_managed",
            "pid":self.pid,
            "endpoint":self.endpoint,
            "modelId":self.model_id,
            "modelRevision":self.model_revision,
            "pythonVersion":self.python_version,
            "mlxLmVersion":self.mlx_lm_version,
            "lockSha256":self.lock_sha256
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        if value.get("schemaVersion").and_then(Value::as_u64) != Some(1)
            || value.get("runtimeId").and_then(Value::as_str) != Some("runtime.mlx-lm")
            || value.get("ownership").and_then(Value::as_str) != Some("desktoplab_managed")
        {
            return None;
        }
        Some(Self {
            pid: u32::try_from(value.get("pid")?.as_u64()?).ok()?,
            endpoint: value.get("endpoint")?.as_str()?.to_string(),
            model_id: value.get("modelId")?.as_str()?.to_string(),
            model_revision: value.get("modelRevision")?.as_str()?.to_string(),
            python_version: value.get("pythonVersion")?.as_str()?.to_string(),
            mlx_lm_version: value.get("mlxLmVersion")?.as_str()?.to_string(),
            lock_sha256: value.get("lockSha256")?.as_str()?.to_string(),
        })
    }
}

pub(crate) fn read_marker(path: &Path) -> Result<Option<MlxLmOwnershipMarker>, io::Error> {
    match fs::read(path) {
        Ok(bytes) => {
            let value = serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            MlxLmOwnershipMarker::from_json(&value)
                .map(Some)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid marker"))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn write_marker(path: &Path, marker: &MlxLmOwnershipMarker) -> Result<(), io::Error> {
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
