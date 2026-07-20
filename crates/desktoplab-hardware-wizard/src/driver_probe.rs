#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DriverProbeState {
    Confirmed,
    Unsupported,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DriverProbeObservation {
    state: DriverProbeState,
    version: Option<String>,
    reason: Option<String>,
}

impl DriverProbeObservation {
    #[must_use]
    pub fn confirmed(version: impl Into<String>) -> Self {
        Self {
            state: DriverProbeState::Confirmed,
            version: Some(version.into()),
            reason: None,
        }
    }

    #[must_use]
    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self {
            state: DriverProbeState::Unsupported,
            version: None,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn state(&self) -> DriverProbeState {
        self.state
    }

    #[must_use]
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    #[must_use]
    pub fn is_confirmed(&self) -> bool {
        self.state == DriverProbeState::Confirmed
    }
}

pub trait DriverProbeSource {
    fn cuda(&self) -> DriverProbeObservation;
    fn rocm(&self) -> DriverProbeObservation;
    fn metal(&self) -> DriverProbeObservation;
}

#[derive(Clone, Debug)]
pub struct HardwareDriverProbeAdapter<S> {
    source: S,
}

impl<S> HardwareDriverProbeAdapter<S>
where
    S: DriverProbeSource,
{
    #[must_use]
    pub fn new(source: S) -> Self {
        Self { source }
    }

    #[must_use]
    pub fn report(&self) -> DriverProbeReport {
        DriverProbeReport {
            cuda: self.source.cuda(),
            rocm: self.source.rocm(),
            metal: self.source.metal(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverProbeReport {
    cuda: DriverProbeObservation,
    rocm: DriverProbeObservation,
    metal: DriverProbeObservation,
}

impl DriverProbeReport {
    #[must_use]
    pub fn cuda(&self) -> &DriverProbeObservation {
        &self.cuda
    }

    #[must_use]
    pub fn rocm(&self) -> &DriverProbeObservation {
        &self.rocm
    }

    #[must_use]
    pub fn metal(&self) -> &DriverProbeObservation {
        &self.metal
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DriverProbePlan {
    requires_elevated_permissions: bool,
}

impl DriverProbePlan {
    #[must_use]
    pub fn v2() -> Self {
        Self {
            requires_elevated_permissions: false,
        }
    }

    #[must_use]
    pub fn requires_elevated_permissions(&self) -> bool {
        self.requires_elevated_permissions
    }
}
