#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiPayloadGuard {
    max_bytes: usize,
}

impl ApiPayloadGuard {
    #[must_use]
    pub fn new(max_bytes: usize) -> Self {
        Self { max_bytes }
    }

    pub fn check(&self, payload: &str) -> Result<(), &'static str> {
        if payload.len() > self.max_bytes {
            Err("api_response_too_large")
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceScanGuard {
    max_files: usize,
}

impl WorkspaceScanGuard {
    #[must_use]
    pub fn new(max_files: usize) -> Self {
        Self { max_files }
    }

    #[must_use]
    pub fn classify(&self, observed_files: usize) -> WorkspaceScanDecision {
        if observed_files > self.max_files {
            WorkspaceScanDecision::degraded("workspace_scan_file_limit_exceeded")
        } else {
            WorkspaceScanDecision::complete()
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceScanDecision {
    degraded: bool,
    reason: Option<&'static str>,
}

impl WorkspaceScanDecision {
    #[must_use]
    pub fn complete() -> Self {
        Self {
            degraded: false,
            reason: None,
        }
    }

    #[must_use]
    pub fn degraded(reason: &'static str) -> Self {
        Self {
            degraded: true,
            reason: Some(reason),
        }
    }

    #[must_use]
    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    #[must_use]
    pub fn reason(&self) -> Option<&'static str> {
        self.reason
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownloadDiskGuard {
    available_bytes: u64,
}

impl DownloadDiskGuard {
    #[must_use]
    pub fn new(available_bytes: u64) -> Self {
        Self { available_bytes }
    }

    pub fn check(&self, required_bytes: u64) -> Result<(), &'static str> {
        if required_bytes > self.available_bytes {
            Err("download_disk_limit_exceeded")
        } else {
            Ok(())
        }
    }
}
