use crate::{HighEndRuntimeContract, RuntimeId, RuntimeLaunchSupport};
use std::process::{Child, Command, ExitStatus, Stdio};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighEndLaunchSpec {
    program: String,
    args: Vec<String>,
}

impl HighEndLaunchSpec {
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
}

#[derive(Debug)]
pub struct DesktopLabOwnedRuntimeProcess {
    runtime_id: RuntimeId,
    child: Child,
}

impl DesktopLabOwnedRuntimeProcess {
    pub fn launch_after_approval(
        contract: &HighEndRuntimeContract,
        spec: &HighEndLaunchSpec,
    ) -> Result<Self, HighEndProcessError> {
        if contract.launch_support() == RuntimeLaunchSupport::AttachOnly {
            return Err(HighEndProcessError::AttachOnly(
                contract.runtime_id().clone(),
            ));
        }
        if spec.program.trim().is_empty() {
            return Err(HighEndProcessError::InvalidProgram);
        }
        let child = Command::new(spec.program())
            .args(spec.args())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| HighEndProcessError::LaunchFailed(error.to_string()))?;
        Ok(Self {
            runtime_id: contract.runtime_id().clone(),
            child,
        })
    }

    #[must_use]
    pub fn runtime_id(&self) -> &RuntimeId {
        &self.runtime_id
    }

    pub fn try_status(&mut self) -> Result<Option<ExitStatus>, HighEndProcessError> {
        self.child
            .try_wait()
            .map_err(|error| HighEndProcessError::ProcessIo(error.to_string()))
    }

    pub fn stop_owned(mut self) -> Result<ExitStatus, HighEndProcessError> {
        if self
            .child
            .try_wait()
            .map_err(|error| HighEndProcessError::ProcessIo(error.to_string()))?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| HighEndProcessError::ProcessIo(error.to_string()))?;
        }
        self.child
            .wait()
            .map_err(|error| HighEndProcessError::ProcessIo(error.to_string()))
    }
}

impl Drop for DesktopLabOwnedRuntimeProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HighEndProcessError {
    AttachOnly(RuntimeId),
    InvalidProgram,
    LaunchFailed(String),
    ProcessIo(String),
}
