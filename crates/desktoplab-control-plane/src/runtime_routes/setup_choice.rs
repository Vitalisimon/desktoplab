use super::helpers::string_body_field;
use desktoplab_runtime::RuntimeInstallError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSetupChoice {
    UseExisting,
    Replace,
}

impl RuntimeSetupChoice {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UseExisting => "use_existing",
            Self::Replace => "replace",
        }
    }
}

pub fn runtime_setup_choice(body: &str) -> Result<RuntimeSetupChoice, RuntimeInstallError> {
    match string_body_field(body, "setupChoice").as_deref() {
        None | Some("") | Some("use_existing") | Some("install") => {
            Ok(RuntimeSetupChoice::UseExisting)
        }
        Some("replace") => Ok(RuntimeSetupChoice::Replace),
        Some(_) => Err(RuntimeInstallError::UnknownSetupChoice),
    }
}
