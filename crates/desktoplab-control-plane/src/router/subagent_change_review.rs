use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use desktoplab_redaction::redact_sensitive_with_status;
use serde_json::{Value, json};

use super::worktree_bindings::WorktreeBinding;

const MAX_DIFF_PREVIEW_CHARS: usize = 32 * 1024;

pub(super) fn review(binding: &WorktreeBinding, terminal: bool) -> Value {
    match collect_review(binding, terminal) {
        Ok(review) => review,
        Err(reason) => json!({
            "status":"blocked",
            "readyToIntegrate":false,
            "reason":reason
        }),
    }
}

fn collect_review(binding: &WorktreeBinding, terminal: bool) -> Result<Value, String> {
    let root = Path::new(binding.worktree_root());
    if !root.is_dir() {
        return Err("managed_worktree_missing".to_string());
    }
    let base = binding.base_head().trim();
    if base.is_empty() {
        return Err("base_commit_unavailable".to_string());
    }
    let head = git_text(root, &["rev-parse", "HEAD"])?.trim().to_string();
    if !git_success(root, &["merge-base", "--is-ancestor", base, &head])? {
        return Err("base_commit_not_ancestor".to_string());
    }

    let status = git_bytes(root, &["status", "--porcelain=v1", "-z"])?;
    let working_tree_clean = status.is_empty();
    let commit_hashes = nul_strings(&git_bytes(
        root,
        &["rev-list", "--reverse", &format!("{base}..{head}")],
    )?);
    let commits = commit_hashes
        .iter()
        .map(|hash| {
            let subject = git_text(root, &["show", "-s", "--format=%s", hash])
                .unwrap_or_default()
                .trim()
                .to_string();
            json!({"commitHash":hash,"subject":subject})
        })
        .collect::<Vec<_>>();
    let changed_files = changed_files(root, base)?;
    let diff = git_text(root, &["diff", "--no-ext-diff", "--unified=3", base])?;
    let redacted = redact_sensitive_with_status(&diff);
    let preview = redacted
        .value()
        .chars()
        .take(MAX_DIFF_PREVIEW_CHARS)
        .collect::<String>();
    let diff_truncated = redacted.value().chars().count() > preview.chars().count();
    let reason = if !terminal {
        Some("subagent_not_terminal")
    } else if !working_tree_clean {
        Some("uncommitted_changes")
    } else if commit_hashes.is_empty() {
        Some("no_committed_changes")
    } else {
        None
    };
    let ready = reason.is_none();

    Ok(json!({
        "status":if ready { "reviewable" } else { "blocked" },
        "baseCommit":base,
        "headCommit":head,
        "commits":commits,
        "changedFiles":changed_files,
        "workingTreeClean":working_tree_clean,
        "diffPreview":preview,
        "diffTruncated":diff_truncated,
        "diffRedacted":redacted.redacted(),
        "readyToIntegrate":ready,
        "reason":reason,
        "integration":{
            "strategy":"cherry_pick",
            "commitHashes":commit_hashes,
            "requiresApproval":true
        }
    }))
}

fn changed_files(root: &Path, base: &str) -> Result<Vec<String>, String> {
    let mut paths = BTreeSet::new();
    for path in nul_strings(&git_bytes(root, &["diff", "--name-only", "-z", base])?) {
        paths.insert(path);
    }
    for path in nul_strings(&git_bytes(
        root,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?) {
        paths.insert(path);
    }
    Ok(paths.into_iter().collect())
}

fn nul_strings(bytes: &[u8]) -> Vec<String> {
    bytes
        .split(|byte| *byte == 0 || *byte == b'\n')
        .filter(|part| !part.is_empty())
        .map(|part| String::from_utf8_lossy(part).to_string())
        .collect()
}

fn git_text(root: &Path, args: &[&str]) -> Result<String, String> {
    Ok(String::from_utf8_lossy(&git_output(root, args)?.stdout).to_string())
}

fn git_bytes(root: &Path, args: &[&str]) -> Result<Vec<u8>, String> {
    Ok(git_output(root, args)?.stdout)
}

fn git_success(root: &Path, args: &[&str]) -> Result<bool, String> {
    Ok(Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("git_unavailable:{error}"))?
        .status
        .success())
}

fn git_output(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("git_unavailable:{error}"))?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(format!(
            "git_review_failed:{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn review_source_stays_focused() {
        xtask::check_logical_line_limit(
            "crates/desktoplab-control-plane/src/router/subagent_change_review.rs",
            include_str!("subagent_change_review.rs"),
            190,
        )
        .expect("subagent change review should stay focused");
    }
}
