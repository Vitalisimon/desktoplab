#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelDownloadCapacity {
    pub(crate) disk_available_mb: u64,
    pub(crate) network_available: bool,
}

impl ModelDownloadCapacity {
    #[must_use]
    pub fn new(disk_available_mb: u64) -> Self {
        Self {
            disk_available_mb,
            network_available: true,
        }
    }

    #[must_use]
    pub fn with_network_available(mut self, network_available: bool) -> Self {
        self.network_available = network_available;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelDownloadExecutionPolicy {
    pub(crate) resume_supported: bool,
}

impl ModelDownloadExecutionPolicy {
    #[must_use]
    pub fn resumable() -> Self {
        Self {
            resume_supported: true,
        }
    }
}
