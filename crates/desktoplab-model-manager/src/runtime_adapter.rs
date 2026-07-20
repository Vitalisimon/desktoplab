use desktoplab_runtime::{
    HighEndRuntimeContract, MlxLmRuntime, ProcessCommand, ProcessRunner, RuntimeId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighEndModelRuntimeAdapter {
    contract: HighEndRuntimeContract,
}

impl HighEndModelRuntimeAdapter {
    #[must_use]
    pub fn new(contract: HighEndRuntimeContract) -> Self {
        Self { contract }
    }

    #[must_use]
    pub fn contract(&self) -> &HighEndRuntimeContract {
        &self.contract
    }

    #[must_use]
    pub fn supports_direct_pull(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeModelPullRequest {
    runtime_id: RuntimeId,
    pull_ref: String,
}

impl RuntimeModelPullRequest {
    #[must_use]
    pub fn new(runtime_id: RuntimeId, pull_ref: impl Into<String>) -> Self {
        Self {
            runtime_id,
            pull_ref: pull_ref.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRuntimeReadiness {
    runtime_id: RuntimeId,
    model_ref: String,
}

impl ModelRuntimeReadiness {
    #[must_use]
    pub fn new(runtime_id: RuntimeId, model_ref: impl Into<String>) -> Self {
        Self {
            runtime_id,
            model_ref: model_ref.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRuntimePullResult {
    state: String,
    command_evidence: String,
    stdout: String,
    stderr: String,
    reason: Option<String>,
}

impl ModelRuntimePullResult {
    #[must_use]
    pub fn blocked_for_unsupported_runtime() -> Self {
        blocked_result("unsupported runtime", "unsupported runtime")
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
pub struct ModelRuntimeReadinessResult {
    ready: bool,
    reason: Option<String>,
}

impl ModelRuntimeReadinessResult {
    #[must_use]
    pub fn ready() -> Self {
        Self {
            ready: true,
            reason: None,
        }
    }

    #[must_use]
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            ready: false,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

pub trait ModelRuntimeAdapter {
    fn pull(&self, request: RuntimeModelPullRequest) -> ModelRuntimePullResult;
    fn list(&self, runtime_id: RuntimeId) -> Vec<String>;
    fn verify(&self, readiness: ModelRuntimeReadiness) -> ModelRuntimeReadinessResult;
    fn cancel(&self, _runtime_id: RuntimeId, _model_ref: &str) -> ModelRuntimePullResult;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OllamaModelRuntimeAdapter<R> {
    runner: R,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MlxLmModelRuntimeAdapter<R> {
    runner: R,
}

impl<R> OllamaModelRuntimeAdapter<R>
where
    R: ProcessRunner,
{
    #[must_use]
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R> ModelRuntimeAdapter for OllamaModelRuntimeAdapter<R>
where
    R: ProcessRunner,
{
    fn pull(&self, request: RuntimeModelPullRequest) -> ModelRuntimePullResult {
        if request.runtime_id.as_str() != "runtime.ollama" {
            return blocked_result("unsupported runtime", "unsupported runtime");
        }
        let output = self.runner.run(
            ProcessCommand::new("ollama")
                .arg("pull")
                .arg(request.pull_ref),
        );
        let state = if output.succeeded() {
            "completed"
        } else {
            "blocked"
        };
        ModelRuntimePullResult {
            state: state.to_string(),
            command_evidence: output.evidence().evidence(),
            stdout: output.stdout().to_string(),
            stderr: output.stderr().to_string(),
            reason: (!output.succeeded()).then(|| "runtime pull failed".to_string()),
        }
    }

    fn list(&self, runtime_id: RuntimeId) -> Vec<String> {
        if runtime_id.as_str() != "runtime.ollama" {
            return Vec::new();
        }
        let output = self.runner.run(ProcessCommand::new("ollama").arg("list"));
        if !output.succeeded() {
            return Vec::new();
        }
        output
            .stdout()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect()
    }

    fn verify(&self, readiness: ModelRuntimeReadiness) -> ModelRuntimeReadinessResult {
        let models = self.list(readiness.runtime_id);
        if models
            .iter()
            .any(|model| model == &readiness.model_ref || model.starts_with(&readiness.model_ref))
        {
            return ModelRuntimeReadinessResult::ready();
        }
        ModelRuntimeReadinessResult::blocked("model_not_reported_by_runtime")
    }

    fn cancel(&self, runtime_id: RuntimeId, model_ref: &str) -> ModelRuntimePullResult {
        if runtime_id.as_str() != "runtime.ollama" {
            return blocked_result("unsupported runtime", "unsupported runtime");
        }
        blocked_result(
            format!("ollama pull {model_ref}"),
            "runtime_cancel_not_supported",
        )
    }
}

impl<R> MlxLmModelRuntimeAdapter<R>
where
    R: ProcessRunner,
{
    #[must_use]
    pub fn new(runner: R) -> Self {
        Self { runner }
    }
}

impl<R> ModelRuntimeAdapter for MlxLmModelRuntimeAdapter<R>
where
    R: ProcessRunner,
{
    fn pull(&self, request: RuntimeModelPullRequest) -> ModelRuntimePullResult {
        if request.runtime_id.as_str() != "runtime.mlx-lm" {
            return blocked_result("unsupported runtime", "unsupported runtime");
        }
        let Some(command) = MlxLmRuntime::new().download_command(request.pull_ref) else {
            return blocked_result("unsafe model reference", "unsafe model reference");
        };
        let output = self.runner.run(command);
        ModelRuntimePullResult {
            state: if output.succeeded() {
                "completed"
            } else {
                "blocked"
            }
            .to_string(),
            command_evidence: output.evidence().evidence(),
            stdout: output.stdout().to_string(),
            stderr: output.stderr().to_string(),
            reason: (!output.succeeded()).then(|| "runtime pull failed".to_string()),
        }
    }

    fn list(&self, _runtime_id: RuntimeId) -> Vec<String> {
        Vec::new()
    }

    fn verify(&self, readiness: ModelRuntimeReadiness) -> ModelRuntimeReadinessResult {
        if readiness.runtime_id.as_str() != "runtime.mlx-lm" {
            return ModelRuntimeReadinessResult::blocked("unsupported runtime");
        }
        let Some(command) = MlxLmRuntime::new().download_command(readiness.model_ref) else {
            return ModelRuntimeReadinessResult::blocked("unsafe model reference");
        };
        if self.runner.run(command).succeeded() {
            ModelRuntimeReadinessResult::ready()
        } else {
            ModelRuntimeReadinessResult::blocked("model_not_loadable_by_mlx_lm")
        }
    }

    fn cancel(&self, runtime_id: RuntimeId, model_ref: &str) -> ModelRuntimePullResult {
        if runtime_id.as_str() != "runtime.mlx-lm" {
            return blocked_result("unsupported runtime", "unsupported runtime");
        }
        blocked_result(
            format!("mlx_lm.generate --model {model_ref}"),
            "runtime_cancel_not_supported",
        )
    }
}

fn blocked_result(
    evidence: impl Into<String>,
    reason: impl Into<String>,
) -> ModelRuntimePullResult {
    ModelRuntimePullResult {
        state: "blocked".to_string(),
        command_evidence: evidence.into(),
        stdout: String::new(),
        stderr: String::new(),
        reason: Some(reason.into()),
    }
}
