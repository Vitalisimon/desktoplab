#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalRiskClass {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolIntent {
    FilesystemList {
        path: Option<String>,
    },
    FilesystemRead {
        path: String,
    },
    SearchText {
        query: String,
        path: Option<String>,
    },
    FilesystemWrite {
        path: String,
    },
    FilesystemPatch {
        path: String,
    },
    FilesystemCreateDirectory {
        path: String,
    },
    FilesystemMove {
        source: String,
        destination: String,
    },
    FilesystemDelete {
        path: String,
        recursive: bool,
    },
    Terminal {
        workspace_id: Option<String>,
        working_directory: String,
        command: String,
        risk_class: TerminalRiskClass,
    },
    ProcessStart {
        workspace_id: String,
        session_id: String,
        working_directory: String,
        command: String,
    },
    ProcessPoll {
        process_id: String,
    },
    ProcessStdin {
        process_id: String,
    },
    ProcessKill {
        process_id: String,
    },
    TestRun {
        workspace_id: Option<String>,
        working_directory: String,
        command: String,
        reason: String,
    },
    GitCommit {
        message: String,
        paths: Vec<String>,
    },
    GitStatus,
    GitDiff {
        path: Option<String>,
    },
    GitPush {
        remote: String,
        branch: String,
    },
    CreateCheckpoint {
        label: String,
    },
    McpInvoke {
        tool_id: String,
        arguments: serde_json::Value,
    },
    Clarify {
        question: String,
        blocked_action: Option<String>,
    },
    RuntimeInstall {
        runtime_id: String,
    },
}

impl ToolIntent {
    #[must_use]
    pub fn filesystem_list(path: Option<String>) -> Self {
        Self::FilesystemList { path }
    }

    #[must_use]
    pub fn filesystem_read(path: impl Into<String>) -> Self {
        Self::FilesystemRead { path: path.into() }
    }

    #[must_use]
    pub fn search_text(query: impl Into<String>, path: Option<String>) -> Self {
        Self::SearchText {
            query: query.into(),
            path,
        }
    }

    #[must_use]
    pub fn filesystem_write(path: impl Into<String>) -> Self {
        Self::FilesystemWrite { path: path.into() }
    }

    #[must_use]
    pub fn filesystem_patch(path: impl Into<String>) -> Self {
        Self::FilesystemPatch { path: path.into() }
    }

    #[must_use]
    pub fn filesystem_create_directory(path: impl Into<String>) -> Self {
        Self::FilesystemCreateDirectory { path: path.into() }
    }

    #[must_use]
    pub fn filesystem_move(source: impl Into<String>, destination: impl Into<String>) -> Self {
        Self::FilesystemMove {
            source: source.into(),
            destination: destination.into(),
        }
    }

    #[must_use]
    pub fn filesystem_delete(path: impl Into<String>, recursive: bool) -> Self {
        Self::FilesystemDelete {
            path: path.into(),
            recursive,
        }
    }

    #[must_use]
    pub fn test_run(command: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::TestRun {
            workspace_id: None,
            working_directory: String::new(),
            command: command.into(),
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn git_commit(message: impl Into<String>) -> Self {
        Self::GitCommit {
            message: message.into(),
            paths: Vec::new(),
        }
    }

    #[must_use]
    pub fn git_commit_selected(
        message: impl Into<String>,
        paths: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::GitCommit {
            message: message.into(),
            paths: paths.into_iter().map(Into::into).collect(),
        }
    }

    #[must_use]
    pub fn git_status() -> Self {
        Self::GitStatus
    }

    #[must_use]
    pub fn git_diff(path: Option<String>) -> Self {
        Self::GitDiff { path }
    }

    #[must_use]
    pub fn git_push(remote: impl Into<String>, branch: impl Into<String>) -> Self {
        Self::GitPush {
            remote: remote.into(),
            branch: branch.into(),
        }
    }

    #[must_use]
    pub fn create_checkpoint(label: impl Into<String>) -> Self {
        Self::CreateCheckpoint {
            label: label.into(),
        }
    }

    #[must_use]
    pub fn mcp_invoke(tool_id: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self::McpInvoke {
            tool_id: tool_id.into(),
            arguments,
        }
    }

    #[must_use]
    pub fn clarify(question: impl Into<String>) -> Self {
        Self::Clarify {
            question: question.into(),
            blocked_action: None,
        }
    }

    #[must_use]
    pub fn blocking_clarification(
        question: impl Into<String>,
        blocked_action: impl Into<String>,
    ) -> Self {
        Self::Clarify {
            question: question.into(),
            blocked_action: Some(blocked_action.into()),
        }
    }

    #[must_use]
    pub fn runtime_install(runtime_id: impl Into<String>) -> Self {
        Self::RuntimeInstall {
            runtime_id: runtime_id.into(),
        }
    }

    pub(crate) fn path(&self) -> Option<&str> {
        match self {
            Self::FilesystemRead { path }
            | Self::FilesystemWrite { path }
            | Self::FilesystemPatch { path }
            | Self::FilesystemCreateDirectory { path }
            | Self::FilesystemDelete { path, .. } => Some(path),
            Self::FilesystemMove { source, .. } => Some(source),
            Self::FilesystemList { path } | Self::SearchText { path, .. } => path.as_deref(),
            Self::Terminal { .. }
            | Self::ProcessStart { .. }
            | Self::ProcessPoll { .. }
            | Self::ProcessStdin { .. }
            | Self::ProcessKill { .. }
            | Self::TestRun { .. }
            | Self::GitCommit { .. }
            | Self::GitStatus
            | Self::GitDiff { .. }
            | Self::GitPush { .. }
            | Self::CreateCheckpoint { .. }
            | Self::McpInvoke { .. }
            | Self::Clarify { .. }
            | Self::RuntimeInstall { .. } => None,
        }
    }

    pub(crate) fn secondary_path(&self) -> Option<&str> {
        match self {
            Self::FilesystemMove { destination, .. } => Some(destination),
            _ => None,
        }
    }
}
