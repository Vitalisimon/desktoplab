use super::LocalApiRouter;

impl LocalApiRouter {
    pub(crate) fn publish_runtime_install_phases(
        &mut self,
        job_id: &str,
        result: &desktoplab_runtime::RuntimeInstallExecutionResult,
    ) {
        self.events.publish_runtime_install_phase(
            job_id,
            "detect",
            "completed",
            15,
            "retryable",
            "",
            "Runtime detection finished.",
        );
        match result.verification_state() {
            "verified" => {
                for (phase, percent) in [
                    ("download", 35),
                    ("verify", 55),
                    ("install", 75),
                    ("start", 90),
                    ("health", 100),
                ] {
                    self.events.publish_runtime_install_phase(
                        job_id,
                        phase,
                        "completed",
                        percent,
                        "retryable",
                        "",
                        "Step completed.",
                    );
                }
            }
            "download_failed_retryable" => {
                self.events.publish_runtime_install_phase(
                    job_id,
                    "download",
                    "failed",
                    25,
                    "retryable",
                    "network_unavailable",
                    result.remediation(),
                );
                self.events.publish_runtime_install_phase(
                    job_id,
                    "verify",
                    "queued",
                    25,
                    "retryable",
                    "",
                    "Waiting for the installer download to finish.",
                );
            }
            "requires_admin_action" => {
                for (phase, percent) in [("download", 35), ("verify", 55)] {
                    self.events.publish_runtime_install_phase(
                        job_id,
                        phase,
                        "completed",
                        percent,
                        "retryable",
                        "",
                        "Step completed.",
                    );
                }
                self.events.publish_runtime_install_phase(
                    job_id,
                    "install",
                    "blocked",
                    65,
                    "non_retryable",
                    "requires_admin_action",
                    result.remediation(),
                );
            }
            other => self.events.publish_runtime_install_phase(
                job_id,
                "verify",
                "blocked",
                45,
                "non_retryable",
                other,
                result.remediation(),
            ),
        }
    }
}
