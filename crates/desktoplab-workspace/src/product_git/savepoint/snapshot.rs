use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use super::ProductGitError;

pub(super) fn capture_worktree(
    root: &Path,
    git_dir: &Path,
    head: Option<&str>,
    session_id: &str,
) -> Result<String, ProductGitError> {
    let index_path = temporary_index_path(git_dir, session_id);
    let _cleanup = TemporaryIndex::new(index_path.clone());
    if head.is_some() {
        git_with_index(root, &index_path, &["read-tree", "HEAD"])?;
    } else {
        git_with_index(root, &index_path, &["read-tree", "--empty"])?;
    }
    git_with_index(root, &index_path, &["add", "-A", "--", "."])?;
    let tree = git_stdout_with_index(root, &index_path, &["write-tree"])?;
    let mut args = vec!["commit-tree", tree.trim()];
    if let Some(head) = head {
        args.extend(["-p", head]);
    }
    args.extend(["-m", "DesktopLab worktree checkpoint"]);
    Ok(git_stdout_with_index(root, &index_path, &args)?
        .trim()
        .to_string())
}

fn temporary_index_path(git_dir: &Path, session_id: &str) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let fragment = session_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .take(80)
        .collect::<String>();
    git_dir.join(format!(
        "desktoplab-checkpoint-index-{}-{timestamp}-{fragment}",
        std::process::id()
    ))
}

fn git_with_index(root: &Path, index_path: &Path, args: &[&str]) -> Result<(), ProductGitError> {
    git_index_command(root, index_path, args).map(|_| ())
}

fn git_stdout_with_index(
    root: &Path,
    index_path: &Path,
    args: &[&str],
) -> Result<String, ProductGitError> {
    let output = git_index_command(root, index_path, args)?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

fn git_index_command(
    root: &Path,
    index_path: &Path,
    args: &[&str],
) -> Result<Vec<u8>, ProductGitError> {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=DesktopLab",
            "-c",
            "user.email=desktoplab@example.invalid",
        ])
        .args(args)
        .env("GIT_INDEX_FILE", index_path)
        .current_dir(root)
        .output()
        .map_err(ProductGitError::from_display)?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(ProductGitError::new(
            String::from_utf8_lossy(&output.stderr).trim(),
        ))
    }
}

struct TemporaryIndex {
    path: PathBuf,
}

impl TemporaryIndex {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TemporaryIndex {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let mut lock_path = self.path.as_os_str().to_os_string();
        lock_path.push(".lock");
        let _ = fs::remove_file(PathBuf::from(lock_path));
    }
}
