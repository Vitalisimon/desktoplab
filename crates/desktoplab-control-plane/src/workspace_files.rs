use desktoplab_workspace::{
    FilePreview, FilePreviewLimits, FilePreviewState, FileTreeEntryKind, FileTreeProtection,
    WorkspaceFileTree, WorkspaceFileTreeLimits,
};
use serde_json::{Value, json};
use std::io;
use std::path::Path;

const FILE_TREE_MAX_ENTRIES: usize = 200;
const FILE_TREE_MAX_DEPTH: usize = 8;
const FILE_PREVIEW_MAX_BYTES: usize = 64 * 1024;
const FILE_PREVIEW_MAX_LINES: usize = 400;

pub(crate) fn file_tree_json(workspace_id: &str, root_path: &str) -> io::Result<Value> {
    let tree = WorkspaceFileTree::scan(
        workspace_id,
        Path::new(root_path),
        WorkspaceFileTreeLimits::new(FILE_TREE_MAX_ENTRIES, FILE_TREE_MAX_DEPTH),
    )?;
    let limits = tree.limits();
    let entries = tree
        .entries()
        .iter()
        .map(|entry| {
            json!({
                "path": entry.path(),
                "kind": entry_kind(entry.kind()),
                "protection": entry_protection(entry.protection()),
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "workspaceId": tree.workspace_id(),
        "entries": entries,
        "degraded": tree.is_degraded(),
        "degradedReasons": tree.degraded_reasons(),
        "limits": {
            "maxEntries": limits.max_entries(),
            "maxDepth": limits.max_depth(),
        },
    }))
}

pub(crate) fn file_preview_json(
    workspace_id: &str,
    root_path: &str,
    relative_path: &str,
) -> io::Result<Value> {
    let preview = FilePreview::read(
        Path::new(root_path),
        relative_path,
        FilePreviewLimits::new(FILE_PREVIEW_MAX_BYTES, FILE_PREVIEW_MAX_LINES),
    )?;
    let text = preview.text().map(desktoplab_redaction::redact_sensitive);
    Ok(json!({
        "workspaceId": workspace_id,
        "path": relative_path,
        "state": preview_state(preview.state()),
        "text": text,
        "deniedReason": preview.denied_reason(),
        "originalBytes": preview.original_bytes(),
        "originalLines": preview.original_lines(),
        "returnedLines": preview.returned_lines(),
        "truncated": preview.is_truncated(),
    }))
}

pub(crate) fn context_attachments_json(workspace_id: &str, root_path: &str) -> io::Result<Value> {
    let tree = WorkspaceFileTree::scan(
        workspace_id,
        Path::new(root_path),
        WorkspaceFileTreeLimits::new(FILE_TREE_MAX_ENTRIES, FILE_TREE_MAX_DEPTH),
    )?;
    let mut attachments = tree
        .entries()
        .iter()
        .filter(|entry| matches!(entry.kind(), FileTreeEntryKind::File | FileTreeEntryKind::HiddenFile))
        .map(|entry| {
            let available = entry.kind() == FileTreeEntryKind::File
                && entry.protection() == FileTreeProtection::Readable;
            json!({
                "path": entry.path(),
                "label": entry.path(),
                "state": if available { "available" } else { "unavailable" },
                "disabledReason": if available { Value::Null } else { json!("Protected local file.") },
            })
        })
        .collect::<Vec<_>>();
    attachments.sort_by_key(|entry| {
        (
            entry["state"].as_str() != Some("available"),
            entry["path"].as_str().unwrap_or_default().to_string(),
        )
    });
    Ok(json!({"workspaceId":workspace_id,"attachments":attachments}))
}

pub(crate) fn selected_context_files(root_path: &str, paths: &[String]) -> Vec<(String, String)> {
    paths
        .iter()
        .filter_map(|path| {
            let preview = FilePreview::read(
                Path::new(root_path),
                path,
                FilePreviewLimits::new(FILE_PREVIEW_MAX_BYTES, FILE_PREVIEW_MAX_LINES),
            )
            .ok()?;
            (preview.state() == FilePreviewState::Text)
                .then(|| preview.text().map(|text| (path.clone(), text.to_string())))
                .flatten()
        })
        .collect()
}

pub(crate) fn preview_query_path(request_path: &str) -> Option<String> {
    let query = request_path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (candidate, value) = pair.split_once('=')?;
        (candidate == "path").then(|| decode_query_value(value))
    })
}

fn entry_kind(kind: FileTreeEntryKind) -> &'static str {
    match kind {
        FileTreeEntryKind::Directory => "directory",
        FileTreeEntryKind::File => "file",
        FileTreeEntryKind::HiddenFile => "hidden_file",
        FileTreeEntryKind::Symlink => "symlink",
    }
}

fn entry_protection(protection: FileTreeProtection) -> &'static str {
    match protection {
        FileTreeProtection::Readable => "readable",
        FileTreeProtection::Protected => "protected",
    }
}

fn preview_state(state: FilePreviewState) -> &'static str {
    match state {
        FilePreviewState::Text => "text",
        FilePreviewState::Binary => "binary",
        FilePreviewState::Denied => "denied",
    }
}

fn decode_query_value(value: &str) -> String {
    value.replace("%2F", "/").replace("%20", " ")
}
