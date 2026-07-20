use crate::{RepositoryInspection, TestCommandSet};

pub struct WorkspaceIntelligenceApi;

impl WorkspaceIntelligenceApi {
    #[must_use]
    pub fn snapshot(
        inspection: RepositoryInspection,
        test_commands: TestCommandSet,
    ) -> WorkspaceIntelligenceSnapshot {
        WorkspaceIntelligenceSnapshot {
            inspection,
            test_commands,
            stale: false,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIntelligenceSnapshot {
    inspection: RepositoryInspection,
    test_commands: TestCommandSet,
    stale: bool,
}

impl WorkspaceIntelligenceSnapshot {
    #[must_use]
    pub fn mark_stale(mut self) -> Self {
        self.stale = true;
        self
    }

    #[must_use]
    pub fn has_language(&self, language: &str) -> bool {
        self.inspection.has_language(language)
    }

    #[must_use]
    pub fn has_test_command(&self, command: &str) -> bool {
        self.test_commands
            .commands()
            .iter()
            .any(|candidate| format!("{candidate:?}").contains(command))
    }

    #[must_use]
    pub fn protected_file_summary(&self) -> String {
        self.inspection.summary_text()
    }

    #[must_use]
    pub fn is_stale(&self) -> bool {
        self.stale
    }

    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "languages={:?};tests={:?}",
            self.inspection.languages(),
            self.test_commands.commands()
        )
    }
}
