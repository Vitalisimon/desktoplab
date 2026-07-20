#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Confidence {
    Confirmed,
    Probable,
    Unknown,
    Conflicting,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareObservation<T> {
    value: T,
    confidence: Confidence,
}

impl<T> HardwareObservation<T> {
    #[must_use]
    pub fn new(value: T, confidence: Confidence) -> Self {
        Self { value, confidence }
    }

    #[must_use]
    pub fn confirmed(value: T) -> Self {
        Self::new(value, Confidence::Confirmed)
    }

    #[must_use]
    pub fn unknown(value: T) -> Self {
        Self::new(value, Confidence::Unknown)
    }

    #[must_use]
    pub fn unsupported(value: T) -> Self {
        Self::new(value, Confidence::Unsupported)
    }

    #[must_use]
    pub fn confidence(&self) -> Confidence {
        self.confidence.clone()
    }
}

impl<T: Clone> HardwareObservation<T> {
    #[must_use]
    pub fn value(&self) -> T {
        self.value.clone()
    }
}
