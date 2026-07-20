use crate::ToolIntent;

impl ToolIntent {
    #[must_use]
    pub fn canonical_tool_id(&self) -> &str {
        match self {
            Self::FilesystemList { .. } => "desktoplab.list_files",
            Self::FilesystemRead { .. } => "desktoplab.read_file",
            Self::SearchText { .. } => "desktoplab.search_text",
            Self::FilesystemWrite { .. } => "desktoplab.write_file",
            Self::FilesystemPatch { .. } => "desktoplab.patch_file",
            Self::FilesystemCreateDirectory { .. } => "desktoplab.create_directory",
            Self::FilesystemMove { .. } => "desktoplab.move_path",
            Self::FilesystemDelete { .. } => "desktoplab.delete_path",
            Self::Terminal { .. } => "desktoplab.run_terminal",
            Self::ProcessStart { .. } => "desktoplab.start_process",
            Self::ProcessPoll { .. } => "desktoplab.poll_process",
            Self::ProcessStdin { .. } => "desktoplab.write_process_stdin",
            Self::ProcessKill { .. } => "desktoplab.kill_process",
            Self::TestRun { .. } => "desktoplab.run_tests",
            Self::GitCommit { .. } => "desktoplab.commit_changes",
            Self::GitStatus => "desktoplab.git_status",
            Self::GitDiff { .. } => "desktoplab.git_diff",
            Self::GitPush { .. } => "desktoplab.push_changes",
            Self::CreateCheckpoint { .. } => "desktoplab.create_checkpoint",
            Self::McpInvoke { tool_id, .. } => tool_id,
            Self::Clarify { .. } => "desktoplab.clarify",
            Self::RuntimeInstall { .. } => "desktoplab.install_runtime",
        }
    }

    #[must_use]
    pub fn telemetry_source(&self) -> &'static str {
        match self {
            Self::FilesystemList { .. } => "filesystem.list",
            Self::FilesystemRead { .. } => "filesystem.read",
            Self::SearchText { .. } => "workspace.search",
            Self::FilesystemWrite { .. } => "filesystem.write",
            Self::FilesystemPatch { .. } => "filesystem.patch",
            Self::FilesystemCreateDirectory { .. } => "filesystem.create_directory",
            Self::FilesystemMove { .. } => "filesystem.move",
            Self::FilesystemDelete { .. } => "filesystem.delete",
            Self::Terminal { .. } => "terminal.agent_command",
            Self::ProcessStart { .. } => "process.start",
            Self::ProcessPoll { .. } => "process.poll",
            Self::ProcessStdin { .. } => "process.stdin",
            Self::ProcessKill { .. } => "process.kill",
            Self::TestRun { .. } => "test.runner",
            Self::GitStatus | Self::GitDiff { .. } => "git.read",
            Self::GitCommit { .. } | Self::GitPush { .. } => "git.operation",
            Self::CreateCheckpoint { .. } => "git.checkpoint",
            Self::McpInvoke { .. } => "mcp.tool.invoke",
            Self::Clarify { .. } => "agent.clarify",
            Self::RuntimeInstall { .. } => "runtime.operation",
        }
    }

    #[must_use]
    pub fn telemetry_evidence(&self) -> String {
        match self {
            Self::FilesystemList { path } => path.as_ref().map_or_else(
                || "filesystem.list".to_string(),
                |path| format!("filesystem.list:{path}"),
            ),
            Self::FilesystemRead { path } => format!("filesystem.read:{path}"),
            Self::SearchText { query, path } => path.as_ref().map_or_else(
                || format!("search.text:{query}"),
                |path| format!("search.text:{path}:{query}"),
            ),
            Self::FilesystemWrite { path } => format!("filesystem.write:{path}"),
            Self::FilesystemPatch { path } => format!("filesystem.patch:{path}"),
            Self::FilesystemCreateDirectory { path } => {
                format!("filesystem.create_directory:{path}")
            }
            Self::FilesystemMove {
                source,
                destination,
            } => {
                format!("filesystem.move:{source}:{destination}")
            }
            Self::FilesystemDelete { path, recursive } => {
                format!("filesystem.delete:{path}:recursive={recursive}")
            }
            Self::Terminal { command, .. } => format!("terminal:{command}"),
            Self::ProcessStart { command, .. } => format!("process.start:{command}"),
            Self::ProcessPoll { process_id } => format!("process.poll:{process_id}"),
            Self::ProcessStdin { process_id } => format!("process.stdin:{process_id}"),
            Self::ProcessKill { process_id } => format!("process.kill:{process_id}"),
            Self::TestRun { command, .. } => format!("test.run:{command}"),
            Self::GitStatus => "git.status".to_string(),
            Self::GitDiff { path } => path
                .as_ref()
                .map_or_else(|| "git.diff".to_string(), |path| format!("git.diff:{path}")),
            Self::GitCommit { message, paths } => {
                let mut paths = paths.clone();
                paths.sort();
                paths.dedup();
                format!("git.commit:{message}:paths={}", paths.join(","))
            }
            Self::GitPush { remote, branch } => format!("git.push:{remote}/{branch}"),
            Self::CreateCheckpoint { label } => format!("checkpoint.create:{label}"),
            Self::McpInvoke { tool_id, .. } => format!("mcp.invoke:{tool_id}"),
            Self::Clarify { question, .. } => format!("clarify:{question}"),
            Self::RuntimeInstall { runtime_id } => format!("runtime.install:{runtime_id}"),
        }
    }

    #[must_use]
    pub fn has_mutating_effect(&self) -> bool {
        canonical_tool_mutates(self.canonical_tool_id())
    }
}

#[must_use]
pub fn canonical_tool_from_record(source: &str, evidence: &str) -> Option<String> {
    if is_canonical_tool_id(evidence) {
        return Some(evidence.to_string());
    }
    let evidence_kind = evidence.split(':').next().unwrap_or_default();
    let id = match (source, evidence_kind) {
        ("filesystem.list", _) => "desktoplab.list_files",
        ("filesystem.read", _) => "desktoplab.read_file",
        ("workspace.search", _) => "desktoplab.search_text",
        ("filesystem.write", _) => "desktoplab.write_file",
        ("filesystem.patch", _) => "desktoplab.patch_file",
        ("filesystem.create_directory", _) => "desktoplab.create_directory",
        ("filesystem.move", _) => "desktoplab.move_path",
        ("filesystem.delete", _) => "desktoplab.delete_path",
        ("terminal.agent_command", _) => "desktoplab.run_terminal",
        ("process.start", _) => "desktoplab.start_process",
        ("process.poll", _) => "desktoplab.poll_process",
        ("process.stdin", _) => "desktoplab.write_process_stdin",
        ("process.kill", _) => "desktoplab.kill_process",
        ("test.runner", _) => "desktoplab.run_tests",
        ("git.read", "git.diff") => "desktoplab.git_diff",
        ("git.read", _) => "desktoplab.git_status",
        ("git.operation", "git.commit") => "desktoplab.commit_changes",
        ("git.operation", "git.push") => "desktoplab.push_changes",
        ("git.checkpoint", _) => "desktoplab.create_checkpoint",
        ("agent.clarify", _) => "desktoplab.clarify",
        ("runtime.operation", _) => "desktoplab.install_runtime",
        ("mcp.tool.invoke", "mcp.invoke") => {
            let id = evidence.strip_prefix("mcp.invoke:")?;
            return is_canonical_tool_id(id).then(|| id.to_string());
        }
        _ => return None,
    };
    Some(id.to_string())
}

#[must_use]
pub fn canonical_tool_mutates(id: &str) -> bool {
    matches!(
        id,
        "desktoplab.write_file"
            | "desktoplab.patch_file"
            | "desktoplab.create_directory"
            | "desktoplab.move_path"
            | "desktoplab.delete_path"
            | "desktoplab.run_terminal"
            | "desktoplab.start_process"
            | "desktoplab.write_process_stdin"
            | "desktoplab.kill_process"
            | "desktoplab.run_tests"
            | "desktoplab.create_checkpoint"
            | "desktoplab.commit_changes"
            | "desktoplab.push_changes"
            | "desktoplab.install_runtime"
            | "desktoplab.update_plan"
            | "desktoplab.spawn_subagent"
            | "desktoplab.send_subagent"
            | "desktoplab.cancel_subagent"
            | "desktoplab.close_subagent"
    ) || id.starts_with("mcp.")
}

fn is_canonical_tool_id(id: &str) -> bool {
    (id.starts_with("desktoplab.") || id.starts_with("mcp."))
        && id.len() <= 160
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
