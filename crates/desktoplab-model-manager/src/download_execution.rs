use desktoplab_runtime::{
    MlxLmRuntime, OllamaRuntime, ProcessCommand, ProcessOutput, ProcessRunner, RuntimeId,
};

use crate::ModelDownloadPlan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelDownloadCapacity {
    disk_available_mb: u64,
    network_available: bool,
}

impl ModelDownloadCapacity {
    #[must_use]
    pub fn new(disk_available_mb: u64) -> Self {
        Self {
            disk_available_mb,
            network_available: true,
        }
    }

    #[must_use]
    pub fn with_network_available(mut self, network_available: bool) -> Self {
        self.network_available = network_available;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelDownloadExecutionPolicy {
    resume_supported: bool,
}

impl ModelDownloadExecutionPolicy {
    #[must_use]
    pub fn resumable() -> Self {
        Self {
            resume_supported: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelDownloadError {
    SetupPlanNotAccepted,
    InsufficientDisk { required_mb: u64, available_mb: u64 },
    NetworkUnavailable,
    ResumeUnsupported,
    UnsupportedRuntime(String),
    UnsafeModelReference(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelDownloadState {
    Running,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeModelDownloadCommand {
    program: String,
    args: Vec<String>,
}

impl RuntimeModelDownloadCommand {
    #[must_use]
    pub fn new(program: impl Into<String>, args: &[&str]) -> Self {
        Self {
            program: program.into(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
        }
    }

    fn to_process_command(&self) -> ProcessCommand {
        self.args
            .iter()
            .fold(ProcessCommand::new(self.program.clone()), |command, arg| {
                command.arg(arg)
            })
    }

    #[must_use]
    pub fn evidence(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn from_process_command(command: ProcessCommand) -> Self {
        Self {
            program: command.program().to_string(),
            args: command.args().to_vec(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDownloadProcessResult {
    state: String,
    command_evidence: String,
    stdout: String,
    stderr: String,
    reason: Option<String>,
}

impl ModelDownloadProcessResult {
    fn from_output(output: ProcessOutput) -> Self {
        let succeeded = output.succeeded();
        Self {
            state: if succeeded { "completed" } else { "blocked" }.to_string(),
            command_evidence: output.evidence().evidence(),
            stdout: output.stdout().to_string(),
            stderr: output.stderr().to_string(),
            reason: (!succeeded).then(|| "runtime pull failed".to_string()),
        }
    }

    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    #[must_use]
    pub fn command_evidence(&self) -> &str {
        &self.command_evidence
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
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDownloadEvent {
    name: &'static str,
}

impl ModelDownloadEvent {
    fn new(name: &'static str) -> Self {
        Self { name }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDownloadMetadata {
    command: RuntimeModelDownloadCommand,
    state: ModelDownloadState,
    progress_mb: u64,
    resume_supported: bool,
    events: Vec<ModelDownloadEvent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDownloadJob {
    command: RuntimeModelDownloadCommand,
    state: ModelDownloadState,
    progress_mb: u64,
    resume_supported: bool,
    events: Vec<ModelDownloadEvent>,
}

impl ModelDownloadJob {
    fn running(command: RuntimeModelDownloadCommand, resume_supported: bool) -> Self {
        Self {
            command,
            state: ModelDownloadState::Running,
            progress_mb: 0,
            resume_supported,
            events: vec![
                ModelDownloadEvent::new("queued"),
                ModelDownloadEvent::new("started"),
            ],
        }
    }

    #[must_use]
    pub fn state(&self) -> ModelDownloadState {
        self.state
    }

    #[must_use]
    pub fn command(&self) -> &RuntimeModelDownloadCommand {
        &self.command
    }

    #[must_use]
    pub fn progress_mb(&self) -> u64 {
        self.progress_mb
    }

    pub fn record_progress_mb(&mut self, progress_mb: u64) {
        self.progress_mb = progress_mb;
        self.events.push(ModelDownloadEvent::new("progress"));
    }

    pub fn cancel(&mut self) {
        self.state = ModelDownloadState::Cancelled;
        self.events.push(ModelDownloadEvent::new("cancelled"));
    }

    #[must_use]
    pub fn metadata(&self) -> ModelDownloadMetadata {
        ModelDownloadMetadata {
            command: self.command.clone(),
            state: self.state,
            progress_mb: self.progress_mb,
            resume_supported: self.resume_supported,
            events: self.events.clone(),
        }
    }

    #[must_use]
    pub fn event_names(&self) -> Vec<&'static str> {
        self.events.iter().map(|event| event.name).collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDownloadExecutor {
    capacity: ModelDownloadCapacity,
}

impl ModelDownloadExecutor {
    #[must_use]
    pub fn new(capacity: ModelDownloadCapacity) -> Self {
        Self { capacity }
    }

    pub fn start(
        &self,
        plan: ModelDownloadPlan,
        policy: ModelDownloadExecutionPolicy,
    ) -> Result<ModelDownloadJob, ModelDownloadError> {
        if !plan.starts_automatically() {
            return Err(ModelDownloadError::SetupPlanNotAccepted);
        }
        if plan.expected_disk_mb() > self.capacity.disk_available_mb {
            return Err(ModelDownloadError::InsufficientDisk {
                required_mb: plan.expected_disk_mb(),
                available_mb: self.capacity.disk_available_mb,
            });
        }
        if !self.capacity.network_available {
            return Err(ModelDownloadError::NetworkUnavailable);
        }

        let command = command_for_runtime(plan.runtime_id(), plan.pull_ref())?;
        Ok(ModelDownloadJob::running(command, policy.resume_supported))
    }

    pub fn execute<R>(
        &self,
        plan: ModelDownloadPlan,
        policy: ModelDownloadExecutionPolicy,
        runner: &R,
    ) -> Result<ModelDownloadProcessResult, ModelDownloadError>
    where
        R: ProcessRunner,
    {
        let job = self.start(plan, policy)?;
        let output = runner.run(job.command().to_process_command());
        Ok(ModelDownloadProcessResult::from_output(output))
    }

    pub fn resume(
        &self,
        metadata: ModelDownloadMetadata,
    ) -> Result<ModelDownloadJob, ModelDownloadError> {
        if !metadata.resume_supported {
            return Err(ModelDownloadError::ResumeUnsupported);
        }

        let mut events = metadata.events;
        events.push(ModelDownloadEvent::new("resumed"));
        Ok(ModelDownloadJob {
            command: metadata.command,
            state: ModelDownloadState::Running,
            progress_mb: metadata.progress_mb,
            resume_supported: true,
            events,
        })
    }
}

fn command_for_runtime(
    runtime_id: &RuntimeId,
    model_id: &str,
) -> Result<RuntimeModelDownloadCommand, ModelDownloadError> {
    match runtime_id.as_str() {
        "runtime.ollama" => {
            let pull_ref = OllamaRuntime::new()
                .validate_model_pull_ref(model_id)
                .map_err(|error| {
                    ModelDownloadError::UnsafeModelReference(error.pull_ref().to_string())
                })?;
            Ok(RuntimeModelDownloadCommand::new(
                "ollama",
                &["pull", pull_ref.as_str()],
            ))
        }
        "runtime.mlx-lm" => MlxLmRuntime::new()
            .download_command(model_id)
            .map(RuntimeModelDownloadCommand::from_process_command)
            .ok_or_else(|| ModelDownloadError::UnsafeModelReference(model_id.to_string())),
        other => Err(ModelDownloadError::UnsupportedRuntime(other.to_string())),
    }
}
