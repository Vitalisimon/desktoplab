use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

use desktoplab_redaction::redact_sensitive;

use crate::path_security::{contained_existing_path, relative_workspace_path};
use crate::process_platform::{safe_inherited_env, shell_command, terminate_process_tree};

const PROCESS_OUTPUT_LIMIT: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedProcessState {
    Running,
    Exited(i32),
    Killed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedProcessSnapshot {
    process_id: String,
    state: ManagedProcessState,
    stdout: String,
    stderr: String,
    output_truncated: bool,
}

impl ManagedProcessSnapshot {
    pub fn process_id(&self) -> &str {
        &self.process_id
    }

    pub fn state(&self) -> &ManagedProcessState {
        &self.state
    }

    pub fn stdout(&self) -> &str {
        &self.stdout
    }

    pub fn stderr(&self) -> &str {
        &self.stderr
    }

    pub fn output_truncated(&self) -> bool {
        self.output_truncated
    }
}

#[derive(Clone, Default)]
pub struct SharedProcessRegistry {
    inner: Arc<Mutex<ManagedProcessRegistry>>,
}

impl SharedProcessRegistry {
    pub fn start(
        &self,
        root: &Path,
        workspace_id: &str,
        session_id: &str,
        command: &str,
        cwd: &str,
    ) -> Result<ManagedProcessSnapshot, String> {
        let candidate =
            relative_workspace_path(root, Path::new(cwd)).map_err(|_| "path_escape".to_string())?;
        let cwd =
            contained_existing_path(root, &candidate).map_err(|_| "path_escape".to_string())?;
        self.inner
            .lock()
            .map_err(|_| "process_registry_poisoned".to_string())?
            .start(workspace_id, session_id, command, &cwd)
    }

    pub fn poll(
        &self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
    ) -> Result<ManagedProcessSnapshot, String> {
        self.inner
            .lock()
            .map_err(|_| "process_registry_poisoned".to_string())?
            .poll(workspace_id, session_id, process_id)
    }

    pub fn write_stdin(
        &self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
        input: &str,
    ) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|_| "process_registry_poisoned".to_string())?
            .write_stdin(workspace_id, session_id, process_id, input)
    }

    pub fn kill(
        &self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
    ) -> Result<ManagedProcessSnapshot, String> {
        self.inner
            .lock()
            .map_err(|_| "process_registry_poisoned".to_string())?
            .kill(workspace_id, session_id, process_id)
    }

    pub fn kill_session(&self, workspace_id: &str, session_id: &str) -> Result<usize, String> {
        self.inner
            .lock()
            .map_err(|_| "process_registry_poisoned".to_string())?
            .kill_session(workspace_id, session_id)
    }
}

#[derive(Default)]
struct ManagedProcessRegistry {
    next_id: u64,
    processes: BTreeMap<String, ManagedProcess>,
}

impl ManagedProcessRegistry {
    fn start(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        command: &str,
        cwd: &Path,
    ) -> Result<ManagedProcessSnapshot, String> {
        let mut child = shell_command(command)
            .current_dir(cwd)
            .env_clear()
            .envs(safe_inherited_env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("process_spawn_failed:{error}"))?;
        let stdin = child.stdin.take().ok_or("process_stdin_unavailable")?;
        let stdout = child.stdout.take().ok_or("process_stdout_unavailable")?;
        let stderr = child.stderr.take().ok_or("process_stderr_unavailable")?;
        let (sender, receiver) = mpsc::channel();
        spawn_reader(stdout, ProcessStream::Stdout, sender.clone());
        spawn_reader(stderr, ProcessStream::Stderr, sender);
        self.next_id = self.next_id.saturating_add(1);
        let process_id = format!("process.{}", self.next_id);
        self.processes.insert(
            process_id.clone(),
            ManagedProcess {
                workspace_id: workspace_id.to_string(),
                session_id: session_id.to_string(),
                child,
                stdin: Some(stdin),
                receiver,
                state: ManagedProcessState::Running,
                stdout: Vec::new(),
                stderr: Vec::new(),
                output_bytes: 0,
                output_truncated: false,
            },
        );
        self.poll(workspace_id, session_id, &process_id)
    }

    fn poll(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
    ) -> Result<ManagedProcessSnapshot, String> {
        let process = self.owned(workspace_id, session_id, process_id)?;
        process.drain_output();
        if process.state == ManagedProcessState::Running
            && let Some(status) = process
                .child
                .try_wait()
                .map_err(|error| format!("process_poll_failed:{error}"))?
        {
            process.state = ManagedProcessState::Exited(status.code().unwrap_or_default());
            process.stdin = None;
            process.drain_output();
        }
        Ok(process.snapshot(process_id))
    }

    fn write_stdin(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
        input: &str,
    ) -> Result<(), String> {
        let process = self.owned(workspace_id, session_id, process_id)?;
        if process.state != ManagedProcessState::Running {
            return Err("process_not_running".to_string());
        }
        let stdin = process.stdin.as_mut().ok_or("process_stdin_closed")?;
        stdin
            .write_all(input.as_bytes())
            .and_then(|()| stdin.flush())
            .map_err(|error| format!("process_stdin_failed:{error}"))
    }

    fn kill(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
    ) -> Result<ManagedProcessSnapshot, String> {
        let process = self.owned(workspace_id, session_id, process_id)?;
        if process.state == ManagedProcessState::Running {
            terminate_process_tree(&mut process.child)
                .map_err(|error| format!("process_kill_failed:{error}"))?;
            let _ = process.child.wait();
            process.state = ManagedProcessState::Killed;
            process.stdin = None;
            process.drain_output();
        }
        Ok(process.snapshot(process_id))
    }

    fn kill_session(&mut self, workspace_id: &str, session_id: &str) -> Result<usize, String> {
        let mut killed = 0;
        for process in self.processes.values_mut().filter(|process| {
            process.workspace_id == workspace_id && process.session_id == session_id
        }) {
            if process.state != ManagedProcessState::Running {
                continue;
            }
            terminate_process_tree(&mut process.child)
                .map_err(|error| format!("process_kill_failed:{error}"))?;
            let _ = process.child.wait();
            process.state = ManagedProcessState::Killed;
            process.stdin = None;
            process.drain_output();
            killed += 1;
        }
        Ok(killed)
    }

    fn owned(
        &mut self,
        workspace_id: &str,
        session_id: &str,
        process_id: &str,
    ) -> Result<&mut ManagedProcess, String> {
        let process = self
            .processes
            .get_mut(process_id)
            .ok_or_else(|| "process_not_found".to_string())?;
        if process.workspace_id != workspace_id || process.session_id != session_id {
            return Err("process_ownership_denied".to_string());
        }
        Ok(process)
    }
}

impl Drop for ManagedProcessRegistry {
    fn drop(&mut self) {
        for process in self.processes.values_mut() {
            if process.state == ManagedProcessState::Running {
                let _ = terminate_process_tree(&mut process.child);
                let _ = process.child.wait();
            }
        }
    }
}

struct ManagedProcess {
    workspace_id: String,
    session_id: String,
    child: Child,
    stdin: Option<ChildStdin>,
    receiver: mpsc::Receiver<ProcessChunk>,
    state: ManagedProcessState,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    output_bytes: usize,
    output_truncated: bool,
}

impl ManagedProcess {
    fn drain_output(&mut self) {
        while let Ok(chunk) = self.receiver.try_recv() {
            let remaining = PROCESS_OUTPUT_LIMIT.saturating_sub(self.output_bytes);
            let accepted = chunk.bytes.len().min(remaining);
            self.output_bytes = self.output_bytes.saturating_add(chunk.bytes.len());
            match chunk.stream {
                ProcessStream::Stdout => self.stdout.extend_from_slice(&chunk.bytes[..accepted]),
                ProcessStream::Stderr => self.stderr.extend_from_slice(&chunk.bytes[..accepted]),
            }
            self.output_truncated |= accepted < chunk.bytes.len();
        }
    }

    fn snapshot(&mut self, process_id: &str) -> ManagedProcessSnapshot {
        let stdout = std::mem::take(&mut self.stdout);
        let stderr = std::mem::take(&mut self.stderr);
        ManagedProcessSnapshot {
            process_id: process_id.to_string(),
            state: self.state.clone(),
            stdout: redact_sensitive(&String::from_utf8_lossy(&stdout)),
            stderr: redact_sensitive(&String::from_utf8_lossy(&stderr)),
            output_truncated: self.output_truncated,
        }
    }
}

#[derive(Clone, Copy)]
enum ProcessStream {
    Stdout,
    Stderr,
}

struct ProcessChunk {
    stream: ProcessStream,
    bytes: Vec<u8>,
}

fn spawn_reader(
    mut pipe: impl Read + Send + 'static,
    stream: ProcessStream,
    sender: mpsc::Sender<ProcessChunk>,
) {
    thread::spawn(move || {
        let mut buffer = [0_u8; 8 * 1024];
        while let Ok(read) = pipe.read(&mut buffer) {
            if read == 0 {
                break;
            }
            if sender
                .send(ProcessChunk {
                    stream,
                    bytes: buffer[..read].to_vec(),
                })
                .is_err()
            {
                break;
            }
        }
    });
}
