use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::{ProcessCommand, ProcessOutput, ProcessRunner};

#[derive(Clone, Debug)]
pub struct DeterministicProcessRunner {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    sequence: Arc<Mutex<VecDeque<(Option<i32>, String, String)>>>,
}

impl DeterministicProcessRunner {
    #[must_use]
    pub fn succeeds(stdout: impl Into<String>, stderr: impl Into<String>) -> Self {
        Self {
            exit_code: Some(0),
            stdout: stdout.into(),
            stderr: stderr.into(),
            sequence: Arc::default(),
        }
    }

    #[must_use]
    pub fn fails(stderr: impl Into<String>) -> Self {
        Self {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: stderr.into(),
            sequence: Arc::default(),
        }
    }

    #[must_use]
    pub fn missing() -> Self {
        Self {
            exit_code: None,
            stdout: String::new(),
            stderr: "command not found".to_string(),
            sequence: Arc::default(),
        }
    }

    #[must_use]
    pub fn sequence(outputs: Vec<(Option<i32>, &str, &str)>) -> Self {
        Self {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            sequence: Arc::new(Mutex::new(
                outputs
                    .into_iter()
                    .map(|(exit_code, stdout, stderr)| {
                        (exit_code, stdout.to_string(), stderr.to_string())
                    })
                    .collect(),
            )),
        }
    }

    #[must_use]
    pub fn run(&self, command: ProcessCommand) -> ProcessOutput {
        <Self as ProcessRunner>::run(self, command)
    }
}

impl ProcessRunner for DeterministicProcessRunner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        if let Some((exit_code, stdout, stderr)) = self
            .sequence
            .lock()
            .expect("deterministic runner sequence should not be poisoned")
            .pop_front()
        {
            return ProcessOutput::new(exit_code, stdout, stderr, command);
        }
        ProcessOutput::new(
            self.exit_code,
            self.stdout.clone(),
            self.stderr.clone(),
            command,
        )
    }
}
