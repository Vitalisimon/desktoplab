use crate::{DownloadCapacity, DownloadError, DownloadMetadata};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResumableDownloadPlan {
    resume_from_bytes: u64,
    temp_path: String,
}

impl ResumableDownloadPlan {
    pub fn from_metadata(
        metadata: DownloadMetadata,
        temp_path: impl Into<String>,
    ) -> Result<Self, DownloadError> {
        Ok(Self {
            resume_from_bytes: metadata.progress_bytes,
            temp_path: temp_path.into(),
        })
    }

    #[must_use]
    pub fn resume_from_bytes(&self) -> u64 {
        self.resume_from_bytes
    }

    #[must_use]
    pub fn temp_path(&self) -> &str {
        &self.temp_path
    }

    #[must_use]
    pub fn uses_safe_temp_path(&self) -> bool {
        self.temp_path.starts_with(".desktoplab/tmp/") && !self.temp_path.contains("..")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProductDownloadReservation {
    required_bytes: u64,
    capacity: DownloadCapacity,
    side_effects: bool,
}

impl ProductDownloadReservation {
    #[must_use]
    pub fn new(required_bytes: u64, capacity: DownloadCapacity) -> Self {
        Self {
            required_bytes,
            capacity,
            side_effects: false,
        }
    }

    pub fn reserve(&self) -> Result<(), DownloadError> {
        if self.required_bytes > self.capacity.disk_available_bytes {
            return Err(DownloadError::InsufficientDisk {
                required: self.required_bytes,
                available: self.capacity.disk_available_bytes,
            });
        }
        Ok(())
    }

    #[must_use]
    pub fn has_side_effects(&self) -> bool {
        self.side_effects
    }
}
