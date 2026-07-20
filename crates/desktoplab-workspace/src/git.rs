use crate::CheckpointPlan;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const GIT_COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryIdentity {
    root_path: PathBuf,
    git_dir_path: PathBuf,
}

impl RepositoryIdentity {
    #[must_use]
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    #[must_use]
    pub fn git_dir_path(&self) -> &Path {
        &self.git_dir_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitStatus {
    entries: Vec<String>,
    files: Vec<GitStatusFile>,
}

impl GitStatus {
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        !self.entries.is_empty()
    }

    #[must_use]
    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    #[must_use]
    pub fn files(&self) -> &[GitStatusFile] {
        &self.files
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitStatusFile {
    code: String,
    path: String,
}

impl GitStatusFile {
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn is_untracked(&self) -> bool {
        self.code == "??"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitDiff {
    text: String,
}

impl GitDiff {
    #[must_use]
    pub fn as_text(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GitRepository {
    identity: RepositoryIdentity,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self, WorkspaceGitError> {
        let root_path = run_git(path, &["rev-parse", "--show-toplevel"])?;
        let git_dir_path = run_git(path, &["rev-parse", "--git-dir"])?;
        let root_path = PathBuf::from(root_path.trim());
        let git_dir_path = normalize_git_dir(&root_path, git_dir_path.trim());

        Ok(Self {
            identity: RepositoryIdentity {
                root_path,
                git_dir_path,
            },
        })
    }

    #[must_use]
    pub fn identity(&self) -> &RepositoryIdentity {
        &self.identity
    }

    pub fn status(&self) -> Result<GitStatus, WorkspaceGitError> {
        let output = run_git(
            self.identity.root_path(),
            &["status", "--porcelain=v1", "-z"],
        )?;
        let files = parse_porcelain_v1_z(&output);
        let entries = files
            .iter()
            .map(|file| format!("{} {}", file.code(), file.path()))
            .collect();

        Ok(GitStatus { entries, files })
    }

    pub fn diff(&self) -> Result<GitDiff, WorkspaceGitError> {
        let staged = run_git(self.identity.root_path(), &["diff", "--cached", "--", "."])?;
        let unstaged = run_git(self.identity.root_path(), &["diff", "--", "."])?;
        let untracked = untracked_file_preview(self.identity.root_path())?;
        Ok(GitDiff {
            text: join_diff_parts(&[staged, unstaged, untracked]),
        })
    }

    pub fn diff_path(&self, relative_path: &str) -> Result<GitDiff, WorkspaceGitError> {
        validate_relative_path(relative_path)?;
        let staged = run_git(
            self.identity.root_path(),
            &["diff", "--cached", "--", relative_path],
        )?;
        let unstaged = run_git(self.identity.root_path(), &["diff", "--", relative_path])?;
        let untracked = run_git(
            self.identity.root_path(),
            &[
                "ls-files",
                "--others",
                "--exclude-standard",
                "--",
                relative_path,
            ],
        )?;
        let untracked = if untracked.lines().any(|path| path == relative_path) {
            untracked_file_path_preview(self.identity.root_path(), relative_path)?
        } else {
            String::new()
        };
        Ok(GitDiff {
            text: join_diff_parts(&[staged, unstaged, untracked]),
        })
    }

    pub fn prepare_checkpoint(&self) -> Result<CheckpointPlan, WorkspaceGitError> {
        self.status()?;
        Ok(CheckpointPlan::ready())
    }
}

fn parse_porcelain_v1_z(output: &str) -> Vec<GitStatusFile> {
    let mut records = output.split('\0').filter(|record| !record.is_empty());
    let mut entries = Vec::new();
    while let Some(record) = records.next() {
        let status = record.get(..2).unwrap_or(record);
        let path = record.get(3..).unwrap_or_default();
        entries.push(GitStatusFile {
            code: status.to_string(),
            path: path.to_string(),
        });
        if status.contains('R') || status.contains('C') {
            records.next();
        }
    }
    entries
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceGitError {
    message: String,
}

impl WorkspaceGitError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for WorkspaceGitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for WorkspaceGitError {}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, WorkspaceGitError> {
    let mut command = Command::new("git");
    command.args(args).current_dir(cwd);
    let output = output_with_timeout(command, GIT_COMMAND_TIMEOUT)?;

    if !output.status.success() {
        return Err(WorkspaceGitError::new(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn output_with_timeout(
    mut command: Command,
    timeout: Duration,
) -> Result<Output, WorkspaceGitError> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| WorkspaceGitError::new(error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| WorkspaceGitError::new("git stdout pipe unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| WorkspaceGitError::new("git stderr pipe unavailable"))?;
    let stdout_reader = thread::spawn(move || read_pipe(stdout));
    let stderr_reader = thread::spawn(move || read_pipe(stderr));
    let deadline = Instant::now() + timeout;

    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| WorkspaceGitError::new(error.to_string()))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(WorkspaceGitError::new(format!(
                "git command timed out after {}ms",
                timeout.as_millis()
            )));
        }
        thread::sleep(Duration::from_millis(10));
    };

    Ok(Output {
        status,
        stdout: stdout_reader.join().unwrap_or_default(),
        stderr: stderr_reader.join().unwrap_or_default(),
    })
}

fn read_pipe(mut pipe: impl Read) -> Vec<u8> {
    let mut bytes = Vec::new();
    let _ = pipe.read_to_end(&mut bytes);
    bytes
}

fn normalize_git_dir(root_path: &Path, git_dir: &str) -> PathBuf {
    let git_dir_path = PathBuf::from(git_dir);

    if git_dir_path.is_absolute() {
        return git_dir_path;
    }

    root_path.join(git_dir_path)
}

fn untracked_file_preview(root_path: &Path) -> Result<String, WorkspaceGitError> {
    let files = run_git(root_path, &["ls-files", "--others", "--exclude-standard"])?;
    let mut preview = String::new();

    for file in files.lines().filter(|line| !line.trim().is_empty()) {
        preview.push_str("--- /dev/null\n");
        preview.push_str(&format!("+++ {file}\n"));
        preview.push_str(&untracked_contents(root_path, file)?);
    }

    Ok(preview)
}

fn join_diff_parts(parts: &[String]) -> String {
    parts
        .iter()
        .filter(|part| !part.is_empty())
        .map(|part| part.trim_end_matches('\n'))
        .collect::<Vec<_>>()
        .join("\n")
}

fn untracked_file_path_preview(
    root_path: &Path,
    relative_path: &str,
) -> Result<String, WorkspaceGitError> {
    let contents = untracked_contents(root_path, relative_path)?;
    Ok(format!("--- /dev/null\n+++ {relative_path}\n{contents}"))
}

fn untracked_contents(root_path: &Path, relative_path: &str) -> Result<String, WorkspaceGitError> {
    let bytes = fs::read(root_path.join(relative_path))
        .map_err(|error| WorkspaceGitError::new(error.to_string()))?;
    match String::from_utf8(bytes) {
        Ok(contents) => Ok(contents),
        Err(error) => {
            let bytes = error.into_bytes();
            let digest = format!("{:x}", Sha256::digest(&bytes));
            Ok(format!(
                "Binary file omitted from text diff ({} bytes, sha256:{digest})\n",
                bytes.len()
            ))
        }
    }
}

fn validate_relative_path(path: &str) -> Result<(), WorkspaceGitError> {
    let path = Path::new(path);
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(WorkspaceGitError::new("invalid_git_diff_path"));
    }
    path.components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        .then_some(())
        .ok_or_else(|| WorkspaceGitError::new("invalid_git_diff_path"))
}

#[cfg(test)]
mod tests {
    use super::output_with_timeout;
    use std::process::Command;
    use std::time::{Duration, Instant};

    #[test]
    #[cfg(unix)]
    fn command_timeout_returns_instead_of_hanging() {
        let mut command = Command::new("sh");
        command.args(["-c", "sleep 2"]);
        let started = Instant::now();

        let error = output_with_timeout(command, Duration::from_millis(50))
            .expect_err("slow command should time out");

        assert!(error.to_string().contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(750));
    }
}
