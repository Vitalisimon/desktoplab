use crate::{TerminalRiskClass, ToolIntent};

impl ToolIntent {
    #[must_use]
    pub fn terminal(command: impl Into<String>) -> Self {
        Self::terminal_scoped("", command, TerminalRiskClass::Medium)
    }

    #[must_use]
    pub fn terminal_scoped(
        working_directory: impl Into<String>,
        command: impl Into<String>,
        risk_class: TerminalRiskClass,
    ) -> Self {
        Self::Terminal {
            workspace_id: None,
            working_directory: working_directory.into(),
            command: command.into(),
            risk_class,
        }
    }

    #[must_use]
    pub fn terminal_workspace(
        workspace_id: impl Into<String>,
        working_directory: impl Into<String>,
        command: impl Into<String>,
        risk_class: TerminalRiskClass,
    ) -> Self {
        Self::Terminal {
            workspace_id: Some(workspace_id.into()),
            working_directory: working_directory.into(),
            command: command.into(),
            risk_class,
        }
    }

    #[must_use]
    pub fn process_start(
        workspace_id: impl Into<String>,
        session_id: impl Into<String>,
        working_directory: impl Into<String>,
        command: impl Into<String>,
    ) -> Self {
        Self::ProcessStart {
            workspace_id: workspace_id.into(),
            session_id: session_id.into(),
            working_directory: working_directory.into(),
            command: command.into(),
        }
    }

    #[must_use]
    pub fn process_poll(process_id: impl Into<String>) -> Self {
        Self::ProcessPoll {
            process_id: process_id.into(),
        }
    }

    #[must_use]
    pub fn process_stdin(process_id: impl Into<String>) -> Self {
        Self::ProcessStdin {
            process_id: process_id.into(),
        }
    }

    #[must_use]
    pub fn process_kill(process_id: impl Into<String>) -> Self {
        Self::ProcessKill {
            process_id: process_id.into(),
        }
    }

    #[must_use]
    pub fn terminal_workspace_id(&self) -> Option<&str> {
        match self {
            Self::Terminal { workspace_id, .. } | Self::TestRun { workspace_id, .. } => {
                workspace_id.as_deref()
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn terminal_working_directory(&self) -> Option<&str> {
        match self {
            Self::Terminal {
                working_directory, ..
            }
            | Self::TestRun {
                working_directory, ..
            } => Some(working_directory),
            _ => None,
        }
    }

    #[must_use]
    pub fn terminal_risk_class(&self) -> Option<TerminalRiskClass> {
        match self {
            Self::Terminal { risk_class, .. } => Some(*risk_class),
            _ => None,
        }
    }
}
