use std::process::{Command, Stdio};

use crate::ProcessCommand;

pub trait ProcessSpawner {
    fn spawn(&self, command: ProcessCommand) -> Result<u32, String>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SystemProcessSpawner;

impl ProcessSpawner for SystemProcessSpawner {
    fn spawn(&self, command: ProcessCommand) -> Result<u32, String> {
        let mut last_error = None;
        for program in command.program_candidates() {
            match Command::new(&program)
                .args(command.args())
                .envs(
                    command
                        .environment()
                        .iter()
                        .map(|(key, value)| (key, value)),
                )
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => return Ok(child.id()),
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "command not found".to_string()))
    }
}
