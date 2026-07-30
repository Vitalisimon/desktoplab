use super::helpers::string_body_field;
use desktoplab_runtime::RuntimeInstallError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSetupChoice {
    Install,
    UseExisting,
    Replace,
}

impl RuntimeSetupChoice {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::UseExisting => "use_existing",
            Self::Replace => "replace",
        }
    }
}

pub fn runtime_setup_choice(body: &str) -> Result<RuntimeSetupChoice, RuntimeInstallError> {
    match string_body_field(body, "setupChoice").as_deref() {
        None | Some("") | Some("install") => Ok(RuntimeSetupChoice::Install),
        Some("use_existing") => Ok(RuntimeSetupChoice::UseExisting),
        Some("replace") => Ok(RuntimeSetupChoice::Replace),
        Some(_) => Err(RuntimeInstallError::UnknownSetupChoice),
    }
}

#[cfg(test)]
mod tests {
    use super::{RuntimeSetupChoice, runtime_setup_choice};

    #[test]
    fn setup_choice_keeps_managed_install_distinct_from_use_existing() {
        assert_eq!(
            runtime_setup_choice(r#"{"setupChoice":"install"}"#),
            Ok(RuntimeSetupChoice::Install)
        );
        assert_eq!(
            runtime_setup_choice(r#"{"setupChoice":"use_existing"}"#),
            Ok(RuntimeSetupChoice::UseExisting)
        );
    }

    #[test]
    fn missing_setup_choice_defaults_to_install() {
        assert_eq!(runtime_setup_choice("{}"), Ok(RuntimeSetupChoice::Install));
    }
}
