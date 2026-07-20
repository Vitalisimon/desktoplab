#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelVerification {
    passed: bool,
    reason: Option<String>,
}

impl ModelVerification {
    #[must_use]
    pub fn passed() -> Self {
        Self {
            passed: true,
            reason: None,
        }
    }

    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            passed: false,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn from_runtime_inventory(pull_ref: &str, inventory_output: &str) -> Self {
        let found = inventory_output
            .lines()
            .any(|line| inventory_line_has_model(line, pull_ref));
        if found {
            Self::passed()
        } else {
            Self::failed("model_not_reported_by_runtime")
        }
    }
}

fn inventory_line_has_model(line: &str, pull_ref: &str) -> bool {
    let line = line.trim();
    line == pull_ref
        || line.starts_with(pull_ref)
        || line
            .split_whitespace()
            .any(|token| token == pull_ref || token.starts_with(pull_ref))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelReadiness {
    ready: bool,
    reason: Option<String>,
}

impl ModelReadiness {
    #[must_use]
    pub fn from_verification(verification: ModelVerification) -> Self {
        Self {
            ready: verification.passed,
            reason: verification.reason,
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
