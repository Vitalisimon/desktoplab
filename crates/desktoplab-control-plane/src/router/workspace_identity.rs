use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

use super::WorkspaceRecord;

pub(super) fn resolve(
    root: &Path,
    workspaces: &BTreeMap<String, WorkspaceRecord>,
) -> (String, String) {
    let display_name = root
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("workspace")
        .to_string();
    let root_key = canonical_root_key(root);

    if let Some(existing) = workspaces
        .values()
        .find(|workspace| canonical_root_key(Path::new(&workspace.root_path)) == root_key)
    {
        return (existing.workspace_id.clone(), existing.display_name.clone());
    }

    let readable_id = format!("workspace.{display_name}");
    if !workspaces.contains_key(&readable_id) {
        return (readable_id, display_name);
    }

    let digest = format!("{:x}", Sha256::digest(root_key.as_bytes()));
    let short_id = format!("workspace.{display_name}.{}", &digest[..12]);
    if !workspaces.contains_key(&short_id) {
        return (short_id, display_name);
    }

    let full_id = format!("workspace.{display_name}.{digest}");
    if !workspaces.contains_key(&full_id) {
        return (full_id, display_name);
    }

    let mut sequence = 2;
    loop {
        let candidate = format!("{full_id}.{sequence}");
        if !workspaces.contains_key(&candidate) {
            return (candidate, display_name);
        }
        sequence += 1;
    }
}

fn canonical_root_key(root: &Path) -> String {
    let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let key = canonical.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        key.to_lowercase()
    } else {
        key
    }
}
