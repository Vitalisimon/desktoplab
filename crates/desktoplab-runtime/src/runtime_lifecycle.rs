use std::{fs, io, path::Path};

use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeLifecyclePhase {
    Detect,
    Plan,
    Accept,
    Acquire,
    Verify,
    InstallOrConnect,
    Start,
    Health,
    ModelReady,
    ProtocolCanary,
    Available,
}

impl RuntimeLifecyclePhase {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Detect => "detect",
            Self::Plan => "plan",
            Self::Accept => "accept",
            Self::Acquire => "acquire",
            Self::Verify => "verify",
            Self::InstallOrConnect => "install_or_connect",
            Self::Start => "start",
            Self::Health => "health",
            Self::ModelReady => "model_ready",
            Self::ProtocolCanary => "protocol_canary",
            Self::Available => "available",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "detect" => Self::Detect,
            "plan" => Self::Plan,
            "accept" => Self::Accept,
            "acquire" => Self::Acquire,
            "verify" => Self::Verify,
            "install_or_connect" => Self::InstallOrConnect,
            "start" => Self::Start,
            "health" => Self::Health,
            "model_ready" => Self::ModelReady,
            "protocol_canary" => Self::ProtocolCanary,
            "available" => Self::Available,
            _ => return None,
        })
    }

    fn ordinal(self) -> u8 {
        match self {
            Self::Detect => 0,
            Self::Plan => 1,
            Self::Accept => 2,
            Self::Acquire => 3,
            Self::Verify => 4,
            Self::InstallOrConnect => 5,
            Self::Start => 6,
            Self::Health => 7,
            Self::ModelReady => 8,
            Self::ProtocolCanary => 9,
            Self::Available => 10,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeLifecycleOwnership {
    DesktopLabManaged,
    UserOwned,
}

impl RuntimeLifecycleOwnership {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DesktopLabManaged => "desktoplab_managed",
            Self::UserOwned => "user_owned",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "desktoplab_managed" => Some(Self::DesktopLabManaged),
            "user_owned" => Some(Self::UserOwned),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeLifecycleFailureClass {
    Retryable,
    Terminal,
    Infrastructure,
}

impl RuntimeLifecycleFailureClass {
    fn as_str(self) -> &'static str {
        match self {
            Self::Retryable => "retryable",
            Self::Terminal => "terminal",
            Self::Infrastructure => "infrastructure",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "retryable" => Some(Self::Retryable),
            "terminal" => Some(Self::Terminal),
            "infrastructure" => Some(Self::Infrastructure),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeLifecycleCheckpoint {
    runtime_id: String,
    ownership: RuntimeLifecycleOwnership,
    phase: RuntimeLifecyclePhase,
    failure: Option<(RuntimeLifecycleFailureClass, String)>,
}

impl RuntimeLifecycleCheckpoint {
    #[must_use]
    pub fn new(runtime_id: impl Into<String>, ownership: RuntimeLifecycleOwnership) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            ownership,
            phase: RuntimeLifecyclePhase::Detect,
            failure: None,
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn ownership(&self) -> RuntimeLifecycleOwnership {
        self.ownership
    }

    #[must_use]
    pub fn phase(&self) -> RuntimeLifecyclePhase {
        self.phase
    }

    pub fn advance(&mut self, phase: RuntimeLifecyclePhase) -> Result<(), &'static str> {
        if phase == self.phase {
            return Ok(());
        }
        if phase.ordinal() != self.phase.ordinal() + 1 {
            return Err("runtime lifecycle phase must advance exactly once");
        }
        self.phase = phase;
        self.failure = None;
        Ok(())
    }

    pub fn fail(&mut self, class: RuntimeLifecycleFailureClass, reason: impl Into<String>) {
        self.failure = Some((class, redact_and_bound(reason.into())));
    }

    #[must_use]
    pub fn recovery(&self, selected_runtime_id: &str) -> RuntimeLifecycleRecovery {
        if selected_runtime_id != self.runtime_id {
            return RuntimeLifecycleRecovery::Blocked("selected_runtime_mismatch".to_string());
        }
        if self.phase == RuntimeLifecyclePhase::Available {
            return RuntimeLifecycleRecovery::Complete;
        }
        match self.failure.as_ref() {
            Some((RuntimeLifecycleFailureClass::Terminal, reason)) => {
                RuntimeLifecycleRecovery::Blocked(reason.clone())
            }
            Some((RuntimeLifecycleFailureClass::Infrastructure, _)) => {
                RuntimeLifecycleRecovery::AwaitOperator(self.phase)
            }
            _ => RuntimeLifecycleRecovery::Resume(self.phase),
        }
    }

    fn to_json(&self) -> Value {
        json!({
            "schemaVersion":1,
            "runtimeId":self.runtime_id,
            "ownership":self.ownership.as_str(),
            "phase":self.phase.as_str(),
            "failure":self.failure.as_ref().map(|(class, reason)| json!({
                "class":class.as_str(),
                "reason":reason
            }))
        })
    }

    fn from_json(value: &Value) -> Option<Self> {
        if value.get("schemaVersion")?.as_u64()? != 1 {
            return None;
        }
        let failure = match value.get("failure") {
            Some(Value::Object(failure)) => Some((
                RuntimeLifecycleFailureClass::from_str(failure.get("class")?.as_str()?)?,
                redact_and_bound(failure.get("reason")?.as_str()?.to_string()),
            )),
            Some(Value::Null) | None => None,
            _ => return None,
        };
        Some(Self {
            runtime_id: safe_runtime_id(value.get("runtimeId")?.as_str()?)?,
            ownership: RuntimeLifecycleOwnership::from_str(value.get("ownership")?.as_str()?)?,
            phase: RuntimeLifecyclePhase::from_str(value.get("phase")?.as_str()?)?,
            failure,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RuntimeLifecycleRecovery {
    Resume(RuntimeLifecyclePhase),
    AwaitOperator(RuntimeLifecyclePhase),
    Blocked(String),
    Complete,
}

pub fn write_runtime_lifecycle(
    path: &Path,
    checkpoint: &RuntimeLifecycleCheckpoint,
) -> Result<(), io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "checkpoint parent missing"))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join("lifecycle.json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&checkpoint.to_json()).map_err(io::Error::other)?,
    )?;
    fs::rename(temporary, path)
}

pub fn read_runtime_lifecycle(
    path: &Path,
) -> Result<Option<RuntimeLifecycleCheckpoint>, io::Error> {
    match fs::read(path) {
        Ok(bytes) => {
            let value = serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            RuntimeLifecycleCheckpoint::from_json(&value)
                .map(Some)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid checkpoint"))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn safe_runtime_id(value: &str) -> Option<String> {
    (!value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-')))
    .then(|| value.to_string())
}

fn redact_and_bound(value: String) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let lower = part.to_ascii_lowercase();
            if lower.contains("token=")
                || lower.contains("secret=")
                || lower.contains("password=")
                || lower.contains("api_key=")
                || part.contains("sk-")
            {
                "[REDACTED]".to_string()
            } else {
                part.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect()
}
