use std::{path::PathBuf, process::Command};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessCommand {
    program: String,
    args: Vec<String>,
}

impl ProcessCommand {
    #[must_use]
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
        }
    }

    #[must_use]
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    #[must_use]
    pub fn program(&self) -> &str {
        &self.program
    }

    #[must_use]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    #[must_use]
    pub fn evidence(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(redact_part)
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[must_use]
    pub fn program_candidates(&self) -> Vec<String> {
        if self.program == "ollama" {
            let mut candidates = vec![
                self.program.clone(),
                "/usr/local/bin/ollama".to_string(),
                "/opt/homebrew/bin/ollama".to_string(),
                "/usr/bin/ollama".to_string(),
                "/Applications/Ollama.app/Contents/Resources/ollama".to_string(),
            ];
            if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
                candidates.push(
                    PathBuf::from(local_app_data)
                        .join("Programs")
                        .join("Ollama")
                        .join("ollama.exe")
                        .to_string_lossy()
                        .to_string(),
                );
            }
            return candidates;
        }
        vec![self.program.clone()]
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessOutput {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    evidence: ProcessCommand,
}

impl ProcessOutput {
    #[must_use]
    pub fn new(
        exit_code: Option<i32>,
        stdout: impl Into<String>,
        stderr: impl Into<String>,
        evidence: ProcessCommand,
    ) -> Self {
        Self {
            exit_code,
            stdout: bound(&redact(&stdout.into())),
            stderr: bound(&redact(&stderr.into())),
            evidence,
        }
    }

    #[must_use]
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
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
    pub fn evidence(&self) -> &ProcessCommand {
        &self.evidence
    }

    #[must_use]
    pub fn succeeded(&self) -> bool {
        self.exit_code == Some(0)
    }
}

pub trait ProcessRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        let mut last_error = None;
        for program in command.program_candidates() {
            match Command::new(&program).args(command.args()).output() {
                Ok(output) => {
                    return ProcessOutput::new(
                        output.status.code(),
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr),
                        command,
                    );
                }
                Err(error) => last_error = Some(error),
            }
        }
        ProcessOutput::new(
            None,
            "",
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "command not found".to_string()),
            command,
        )
    }
}

fn redact(value: &str) -> String {
    value
        .split_whitespace()
        .map(redact_part)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_part(part: &str) -> String {
    let lower = part.to_ascii_lowercase();
    if lower.contains("token=")
        || lower.contains("api_key=")
        || lower.contains("apikey=")
        || lower.contains("secret=")
        || lower.contains("password=")
        || part.contains("sk-")
    {
        "[REDACTED]".to_string()
    } else {
        part.to_string()
    }
}

fn bound(value: &str) -> String {
    const MAX_LEN: usize = 512;
    if value.chars().count() <= MAX_LEN {
        return value.to_string();
    }
    format!("{}...", value.chars().take(MAX_LEN - 3).collect::<String>())
}
