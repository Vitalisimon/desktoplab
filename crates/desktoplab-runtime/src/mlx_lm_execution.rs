use crate::{ProcessCommand, ProcessOutput, ProcessRunner, RuntimeInstallExecutionResult};

pub(crate) fn verify_or_install<R>(runner: &R) -> RuntimeInstallExecutionResult
where
    R: ProcessRunner,
{
    let verification = verify_existing(runner);
    if verification.state() == crate::RuntimeExecutionState::Completed {
        return verification;
    }
    install(runner, verification.evidence())
}

pub(crate) fn verify_existing<R>(runner: &R) -> RuntimeInstallExecutionResult
where
    R: ProcessRunner,
{
    let import = verify_import(runner);
    let evidence = import.evidence().evidence();
    if import.succeeded() {
        return RuntimeInstallExecutionResult::completed(format!(
            "existing MLX-LM runtime detected; {evidence}"
        ));
    }
    RuntimeInstallExecutionResult::blocked(
        evidence,
        "MLX-LM is not installed in the local Python environment.",
    )
}

pub(crate) fn install<R>(runner: &R, prior_evidence: &str) -> RuntimeInstallExecutionResult
where
    R: ProcessRunner,
{
    let install = runner.run(
        ProcessCommand::new("python3")
            .arg("-m")
            .arg("pip")
            .arg("install")
            .arg("--upgrade")
            .arg("mlx-lm"),
    );
    let evidence = format!("{prior_evidence}; {}", install.evidence().evidence());
    if !install.succeeded() {
        return RuntimeInstallExecutionResult::failed(
            "python_environment_failed_retryable",
            evidence,
            "Python could not install mlx-lm. Check network access and Python tooling, then retry.",
        );
    }
    let import = verify_import(runner);
    let evidence = format!("{evidence}; {}", import.evidence().evidence());
    if import.succeeded() {
        RuntimeInstallExecutionResult::completed(evidence)
    } else {
        RuntimeInstallExecutionResult::failed(
            "python_environment_verify_failed",
            evidence,
            "mlx-lm installed but could not be imported by Python.",
        )
    }
}

fn verify_import<R>(runner: &R) -> ProcessOutput
where
    R: ProcessRunner,
{
    runner.run(
        ProcessCommand::new("python3")
            .arg("-c")
            .arg("import mlx_lm; print('mlx-lm import ok')"),
    )
}
