#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuntimeId(String);

impl RuntimeId {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeState {
    NotInstalled,
    Installed,
    Degraded,
    Starting,
    Running,
    Stopped,
    VerificationFailed,
    Ready,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeLifecycleState {
    Supported,
    Blocked,
    PackagingManaged,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeLifecycleBoundary {
    state: RuntimeLifecycleState,
    reason: String,
}

impl RuntimeLifecycleBoundary {
    #[must_use]
    pub fn supported(reason: impl Into<String>) -> Self {
        Self {
            state: RuntimeLifecycleState::Supported,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn blocked(reason: impl Into<String>) -> Self {
        Self {
            state: RuntimeLifecycleState::Blocked,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn packaging_managed(reason: impl Into<String>) -> Self {
        Self {
            state: RuntimeLifecycleState::PackagingManaged,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn state(&self) -> RuntimeLifecycleState {
        self.state
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationResult {
    passed: bool,
    reason: Option<String>,
}

impl VerificationResult {
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
    pub fn is_passed(&self) -> bool {
        self.passed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeStatus {
    id: RuntimeId,
    name: String,
    version: Option<String>,
    state: RuntimeState,
    verification_failure: Option<String>,
    exists: bool,
    update_lifecycle: RuntimeLifecycleBoundary,
    uninstall_lifecycle: RuntimeLifecycleBoundary,
}

impl RuntimeStatus {
    #[must_use]
    pub fn not_installed(id: RuntimeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            version: None,
            state: RuntimeState::NotInstalled,
            verification_failure: None,
            exists: true,
            update_lifecycle: default_update_lifecycle(),
            uninstall_lifecycle: default_uninstall_lifecycle(),
        }
    }

    #[must_use]
    pub fn installed(id: RuntimeId, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            version: Some(version.into()),
            state: RuntimeState::Installed,
            verification_failure: None,
            exists: true,
            update_lifecycle: default_update_lifecycle(),
            uninstall_lifecycle: default_uninstall_lifecycle(),
        }
    }

    #[must_use]
    pub fn degraded(id: RuntimeId, name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            version: None,
            state: RuntimeState::Degraded,
            verification_failure: Some(reason.into()),
            exists: true,
            update_lifecycle: default_update_lifecycle(),
            uninstall_lifecycle: default_uninstall_lifecycle(),
        }
    }

    #[must_use]
    pub fn missing(id: RuntimeId) -> Self {
        Self {
            id,
            name: String::new(),
            version: None,
            state: RuntimeState::Unknown,
            verification_failure: None,
            exists: false,
            update_lifecycle: RuntimeLifecycleBoundary::blocked("Runtime is not registered."),
            uninstall_lifecycle: RuntimeLifecycleBoundary::blocked("Runtime is not registered."),
        }
    }

    #[must_use]
    pub fn id(&self) -> &RuntimeId {
        &self.id
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.verification_failure.as_deref()
    }

    #[must_use]
    pub fn state(&self) -> RuntimeState {
        self.state
    }

    #[must_use]
    pub fn exists(&self) -> bool {
        self.exists
    }

    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.state == RuntimeState::Ready
    }

    #[must_use]
    pub fn update_lifecycle(&self) -> &RuntimeLifecycleBoundary {
        &self.update_lifecycle
    }

    #[must_use]
    pub fn uninstall_lifecycle(&self) -> &RuntimeLifecycleBoundary {
        &self.uninstall_lifecycle
    }

    pub fn set_lifecycle(
        &mut self,
        update_lifecycle: RuntimeLifecycleBoundary,
        uninstall_lifecycle: RuntimeLifecycleBoundary,
    ) {
        self.update_lifecycle = update_lifecycle;
        self.uninstall_lifecycle = uninstall_lifecycle;
    }

    pub fn set_state(&mut self, state: RuntimeState) {
        self.state = state;
    }

    pub fn set_installed(&mut self, version: impl Into<String>) {
        self.version = Some(version.into());
        self.state = RuntimeState::Installed;
    }

    pub fn apply_verification(&mut self, result: VerificationResult) {
        if result.is_passed() {
            self.state = RuntimeState::Ready;
            self.verification_failure = None;
        } else {
            self.state = RuntimeState::VerificationFailed;
            self.verification_failure = result.reason;
        }
    }
}

fn default_update_lifecycle() -> RuntimeLifecycleBoundary {
    RuntimeLifecycleBoundary::supported(
        "DesktopLab can manage runtime updates when the installer exposes them.",
    )
}

fn default_uninstall_lifecycle() -> RuntimeLifecycleBoundary {
    RuntimeLifecycleBoundary::supported(
        "DesktopLab can manage runtime removal when the installer exposes it.",
    )
}
