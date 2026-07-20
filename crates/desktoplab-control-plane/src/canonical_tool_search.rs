use std::path::{Component, Path, PathBuf};

use desktoplab_workspace::{WorkspaceSearch, WorkspaceSearchLimits};
use serde_json::{Value, json};

use crate::canonical_tool_executor::CanonicalAgentToolExecutor;

const SEARCH_FILE_BYTE_LIMIT: usize = 64 * 1024;

pub(crate) fn list_files(
    executor: &CanonicalAgentToolExecutor,
    path: Option<&str>,
) -> Result<Value, String> {
    let root = scoped_root(executor.root(), path)?;
    let entries = search_engine()
        .list_files(&root)
        .map_err(|_| "workspace_list_failed".to_string())?
        .into_iter()
        .map(|entry| json!({"path":entry.path(),"sizeBytes":entry.size_bytes()}))
        .collect::<Vec<_>>();
    Ok(json!({"entries":entries,"scope":path.unwrap_or("")}))
}

pub(crate) fn search(
    executor: &CanonicalAgentToolExecutor,
    query: &str,
    path: Option<&str>,
    regex: bool,
    case_sensitive: bool,
) -> Result<Value, String> {
    let root = scoped_root(executor.root(), path)?;
    let report = search_engine()
        .search_with_options(&root, query, regex, case_sensitive)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::InvalidInput {
                "workspace_search_invalid_pattern".to_string()
            } else {
                "workspace_search_failed".to_string()
            }
        })?;
    let matches = report
        .matches()
        .iter()
        .map(|hit| {
            json!({
                "path":hit.path(),
                "preview":hit.preview(),
                "lineNumber":hit.line_number()
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "matches":matches,
        "truncated":report.truncated(),
        "mode":if regex { "regex" } else { "literal" },
        "caseSensitive":case_sensitive
    }))
}

fn scoped_root(root: &Path, relative: Option<&str>) -> Result<PathBuf, String> {
    let Some(relative) = relative else {
        return root
            .canonicalize()
            .map_err(|_| "workspace_path_invalid".to_string());
    };
    let path = Path::new(relative);
    if path.is_absolute()
        || !path
            .components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
    {
        return Err("path_escape".to_string());
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|_| "workspace_path_invalid".to_string())?;
    let candidate = root
        .join(path)
        .canonicalize()
        .map_err(|_| "workspace_path_invalid".to_string())?;
    candidate
        .starts_with(&canonical_root)
        .then_some(candidate)
        .ok_or_else(|| "path_escape".to_string())
}

fn search_engine() -> WorkspaceSearch {
    WorkspaceSearch::new(WorkspaceSearchLimits::new(
        2_000,
        200,
        SEARCH_FILE_BYTE_LIMIT,
    ))
}
