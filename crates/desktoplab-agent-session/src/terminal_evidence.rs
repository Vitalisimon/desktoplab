#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalEvidence {
    command: String,
    output: String,
    exit_code: Option<i32>,
}

impl TerminalEvidence {
    #[must_use]
    pub fn new(
        command: impl Into<String>,
        output: impl Into<String>,
        exit_code: Option<i32>,
    ) -> Self {
        Self {
            command: command.into(),
            output: output.into(),
            exit_code,
        }
    }

    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    #[must_use]
    pub fn output(&self) -> &str {
        &self.output
    }
    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }
}
