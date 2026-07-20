use serde_json::json;

use super::{BackendEventScope, BackendEventStreamService};

impl BackendEventStreamService {
    pub fn publish_runtime_install_progress(
        &mut self,
        job_id: &str,
        state: &str,
        progress_percent: u8,
        retry_class: &str,
        failure_reason: &str,
    ) {
        self.publish_json(
            BackendEventScope::Job,
            json!({
                "jobId":job_id,"kind":"runtime.install","state":state,
                "progressPercent":progress_percent.min(100),"retryClass":retry_class,
                "failureReason":failure_reason
            }),
        );
    }

    pub fn publish_runtime_install_phase(
        &mut self,
        job_id: &str,
        phase: &str,
        state: &str,
        progress_percent: u8,
        retry_class: &str,
        failure_reason: &str,
        next_action: &str,
    ) {
        self.publish_json(
            BackendEventScope::Job,
            json!({
                "jobId":job_id,"kind":"runtime.install","phase":phase,"state":state,
                "progressPercent":progress_percent.min(100),"retryClass":retry_class,
                "failureReason":failure_reason,"nextAction":next_action
            }),
        );
    }

    pub fn publish_model_download_progress(
        &mut self,
        job_id: &str,
        model_id: &str,
        state: &str,
        progress_percent: u8,
        retry_class: &str,
        failure_reason: &str,
    ) {
        self.publish_json(
            BackendEventScope::Job,
            json!({
                "jobId":job_id,"kind":"model.download","modelId":model_id,"state":state,
                "progressPercent":progress_percent.min(100),"retryClass":retry_class,
                "failureReason":failure_reason
            }),
        );
    }
}
