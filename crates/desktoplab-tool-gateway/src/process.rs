use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::process_platform::{safe_inherited_env, shell_command, terminate_process_tree};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalProcessRequest {
    command: String,
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    allowed_env: BTreeSet<String>,
}

impl TerminalProcessRequest {
    #[must_use]
    pub fn new(command: impl Into<String>, cwd: impl AsRef<Path>) -> Self {
        Self {
            command: command.into(),
            cwd: cwd.as_ref().to_path_buf(),
            env: BTreeMap::new(),
            allowed_env: BTreeSet::new(),
        }
    }

    #[must_use]
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    #[must_use]
    pub fn allow_env(mut self, key: impl Into<String>) -> Self {
        self.allowed_env.insert(key.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalProcessStatus {
    Exited(i32),
    TimedOut,
    FailedToSpawn,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalProcessOutput {
    status: TerminalProcessStatus,
    stdout: String,
    stderr: String,
    stdout_original_bytes: usize,
    stderr_original_bytes: usize,
    stdout_truncated: bool,
    stderr_truncated: bool,
}

impl TerminalProcessOutput {
    #[must_use]
    pub fn status(&self) -> TerminalProcessStatus {
        self.status.clone()
    }

    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    #[must_use]
    pub fn stdout_original_bytes(&self) -> usize {
        self.stdout_original_bytes
    }

    #[must_use]
    pub fn stdout_truncated(&self) -> bool {
        self.stdout_truncated
    }

    #[must_use]
    pub fn stderr_truncated(&self) -> bool {
        self.stderr_truncated
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalProcessAdapter {
    timeout: Duration,
    output_limit: usize,
}

impl TerminalProcessAdapter {
    #[must_use]
    pub fn new(timeout: Duration, output_limit: usize) -> Self {
        Self {
            timeout,
            output_limit,
        }
    }

    pub fn run(&self, request: TerminalProcessRequest) -> TerminalProcessOutput {
        let Ok(mut child) = shell_command(&request.command)
            .current_dir(&request.cwd)
            .env_clear()
            .envs(allowed_env(&request))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        else {
            return empty_output(TerminalProcessStatus::FailedToSpawn);
        };

        let Some(stdout) = child.stdout.take() else {
            let _ = terminate_process_tree(&mut child);
            return empty_output(TerminalProcessStatus::FailedToSpawn);
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = terminate_process_tree(&mut child);
            return empty_output(TerminalProcessStatus::FailedToSpawn);
        };
        let output_limit = self.output_limit;
        let stdout_reader = thread::spawn(move || read_bounded(stdout, output_limit));
        let stderr_reader = thread::spawn(move || read_bounded(stderr, output_limit));

        let start = Instant::now();
        let (status, timed_out) = loop {
            match child.try_wait() {
                Ok(Some(status)) => break (Some(status), false),
                Ok(None) if start.elapsed() >= self.timeout => {
                    let _ = terminate_process_tree(&mut child);
                    break (child.wait().ok(), true);
                }
                Ok(None) => thread::sleep(Duration::from_millis(5)),
                Err(_) => {
                    let _ = terminate_process_tree(&mut child);
                    break (None, false);
                }
            }
        };
        collect_output(
            status,
            timed_out,
            stdout_reader.join().unwrap_or_default(),
            stderr_reader.join().unwrap_or_default(),
        )
    }
}

fn allowed_env(request: &TerminalProcessRequest) -> BTreeMap<String, String> {
    let mut env = safe_inherited_env();
    env.extend(
        request
            .env
            .iter()
            .filter(|(key, _)| request.allowed_env.contains(*key))
            .map(|(key, value)| (key.clone(), value.clone())),
    );
    env
}

fn collect_output(
    status: Option<ExitStatus>,
    timed_out: bool,
    stdout: BoundedBytes,
    stderr: BoundedBytes,
) -> TerminalProcessOutput {
    let status = if timed_out {
        TerminalProcessStatus::TimedOut
    } else {
        status.map_or(TerminalProcessStatus::FailedToSpawn, |status| {
            TerminalProcessStatus::Exited(status.code().unwrap_or_default())
        })
    };
    TerminalProcessOutput {
        status,
        stdout: String::from_utf8_lossy(&stdout.bytes).into_owned(),
        stderr: String::from_utf8_lossy(&stderr.bytes).into_owned(),
        stdout_original_bytes: stdout.original_bytes,
        stderr_original_bytes: stderr.original_bytes,
        stdout_truncated: stdout.truncated,
        stderr_truncated: stderr.truncated,
    }
}

fn empty_output(status: TerminalProcessStatus) -> TerminalProcessOutput {
    TerminalProcessOutput {
        status,
        stdout: String::new(),
        stderr: String::new(),
        stdout_original_bytes: 0,
        stderr_original_bytes: 0,
        stdout_truncated: false,
        stderr_truncated: false,
    }
}

#[derive(Default)]
struct BoundedBytes {
    bytes: Vec<u8>,
    original_bytes: usize,
    truncated: bool,
}

fn read_bounded(mut pipe: impl Read, limit: usize) -> BoundedBytes {
    let mut output = BoundedBytes::default();
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        let Ok(read) = pipe.read(&mut chunk) else {
            break;
        };
        if read == 0 {
            break;
        }
        output.original_bytes = output.original_bytes.saturating_add(read);
        let remaining = limit.saturating_sub(output.bytes.len());
        output
            .bytes
            .extend_from_slice(&chunk[..read.min(remaining)]);
        output.truncated |= read > remaining;
    }
    output
}
