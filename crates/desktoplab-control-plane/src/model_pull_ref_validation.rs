use desktoplab_model_manager::ModelDownloadError;

use crate::model_routes::ModelRouteError;

pub(crate) fn validate(runtime_id: &str, pull_ref: &str) -> Result<(), ModelRouteError> {
    match runtime_id {
        "runtime.ollama" => desktoplab_runtime::OllamaRuntime::new()
            .validate_model_pull_ref(pull_ref)
            .map(|_| ())
            .map_err(|error| unsafe_reference(error.pull_ref())),
        "runtime.mlx-lm" => desktoplab_runtime::MlxLmRuntime::new()
            .validate_model_ref(pull_ref)
            .map(|_| ())
            .map_err(|error| unsafe_reference(error.model_ref())),
        _ => Ok(()),
    }
}

fn unsafe_reference(reference: &str) -> ModelRouteError {
    ModelRouteError::Download(ModelDownloadError::UnsafeModelReference(
        reference.to_string(),
    ))
}
