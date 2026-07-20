use crate::{
    InstallPlan, LmStudioGuidedSetupPlan, ProcessCommand, ProcessRunner,
    RuntimeInstallExecutionResult,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeInstallPhase {
    Detect,
    Download,
    VerifyInstaller,
    Install,
    Start,
    Health,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInstallExecutionDesign {
    runtime_id: String,
    phases: Vec<RuntimeInstallPhase>,
}

impl RuntimeInstallExecutionDesign {
    #[must_use]
    pub fn from_install_plan(plan: &InstallPlan) -> Self {
        Self {
            runtime_id: plan.runtime_id().as_str().to_string(),
            phases: vec![
                RuntimeInstallPhase::Detect,
                RuntimeInstallPhase::Download,
                RuntimeInstallPhase::VerifyInstaller,
                RuntimeInstallPhase::Install,
                RuntimeInstallPhase::Start,
                RuntimeInstallPhase::Health,
            ],
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn phases(&self) -> &[RuntimeInstallPhase] {
        &self.phases
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeInstallExecutor<R> {
    runner: R,
}

impl<R> RuntimeInstallExecutor<R>
where
    R: ProcessRunner,
{
    #[must_use]
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    #[must_use]
    pub fn execute_existing_or_install(&self, plan: &InstallPlan) -> RuntimeInstallExecutionResult {
        match plan.runtime_id().as_str() {
            "runtime.ollama" => self.verify_or_install_ollama(plan),
            "runtime.mlx-lm" => crate::mlx_lm_execution::verify_or_install(&self.runner),
            _ => RuntimeInstallExecutionResult::blocked(
                "unsupported runtime install adapter",
                "No executable adapter is registered for this runtime.",
            ),
        }
    }

    #[must_use]
    pub fn execute_install(&self, plan: &InstallPlan) -> RuntimeInstallExecutionResult {
        match plan.runtime_id().as_str() {
            "runtime.ollama" => self.install_ollama(plan, "replace requested"),
            "runtime.mlx-lm" => crate::mlx_lm_execution::install(&self.runner, "replace requested"),
            _ => RuntimeInstallExecutionResult::blocked(
                "unsupported runtime install adapter",
                "No executable adapter is registered for this runtime.",
            ),
        }
    }

    #[must_use]
    pub fn verify_existing(&self, runtime_id: &str) -> RuntimeInstallExecutionResult {
        match runtime_id {
            "runtime.ollama" => self.verify_existing_ollama(),
            "runtime.mlx-lm" => crate::mlx_lm_execution::verify_existing(&self.runner),
            _ => RuntimeInstallExecutionResult::blocked(
                "unsupported runtime verification adapter",
                "No executable verification adapter is registered for this runtime.",
            ),
        }
    }

    fn verify_or_install_ollama(&self, plan: &InstallPlan) -> RuntimeInstallExecutionResult {
        let verification = self.verify_existing_ollama();
        if verification.state() == crate::RuntimeExecutionState::Completed
            || verification.verification_state() == "health_failed_retryable"
        {
            return verification;
        }
        self.install_ollama(plan, verification.evidence())
    }

    fn verify_existing_ollama(&self) -> RuntimeInstallExecutionResult {
        let output = self
            .runner
            .run(ProcessCommand::new("ollama").arg("--version"));
        let evidence = output.evidence().evidence();
        if output.succeeded() {
            let health = self.runner.run(
                ProcessCommand::new("curl")
                    .arg("--fail")
                    .arg("http://127.0.0.1:11434/api/tags"),
            );
            let evidence = format!("{evidence}; {}", health.evidence().evidence());
            if !health.succeeded() {
                return RuntimeInstallExecutionResult::failed(
                    "health_failed_retryable",
                    evidence,
                    "Ollama is installed but its local API is not running. Start Ollama and retry.",
                );
            }
            return RuntimeInstallExecutionResult::completed(format!(
                "existing runtime detected; {evidence}"
            ));
        }
        RuntimeInstallExecutionResult::blocked(
            evidence,
            "Ollama is not installed or is not available on PATH.",
        )
    }

    fn install_ollama(
        &self,
        plan: &InstallPlan,
        prior_evidence: &str,
    ) -> RuntimeInstallExecutionResult {
        if plan.target_platform() == Some("darwin-arm64") {
            return crate::installer_flow::run_macos_ollama_install(
                &self.runner,
                plan,
                prior_evidence,
            );
        }
        if plan.target_platform() == Some("linux-x64") {
            return crate::installer_flow::run_linux_ollama_install(
                &self.runner,
                plan,
                prior_evidence,
            );
        }
        if plan.target_platform() == Some("windows-x64") {
            return crate::windows_ollama_install::run_windows_ollama_install(
                &self.runner,
                plan,
                prior_evidence,
            );
        }
        RuntimeInstallExecutionResult::blocked(
            prior_evidence,
            "Ollama was not detected. Install or allow DesktopLab to download the signed installer.",
        )
    }
}

impl RuntimeInstallExecutor<()> {
    #[must_use]
    pub fn external_guided(plan: LmStudioGuidedSetupPlan) -> RuntimeInstallExecutionResult {
        RuntimeInstallExecutionResult::external_guided(plan.explanation())
    }
}
