use std::path::Path;

#[must_use]
pub fn workspace_open_body(path: &Path) -> String {
    serde_json::json!({ "path": path }).to_string()
}

#[must_use]
pub fn workspace_initialize_body(path: &Path) -> String {
    serde_json::json!({ "path": path, "initializeGit": true }).to_string()
}
