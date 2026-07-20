use desktoplab_backend_services::{JobServiceStore, JobSnapshot, JobState};
use desktoplab_storage::{
    ProductizationRecordKind, ProductizationStateRecord, SqliteStore, StorageError,
};
use serde_json::Value;

use super::LocalApiRouter;
use super::helpers::string_field;

impl LocalApiRouter {
    pub(crate) fn persist_runtime_jobs(&mut self) {
        let result = self.persist_jobs_by_kind(
            ProductizationRecordKind::RuntimeJob,
            "runtime.install",
            |kind| kind == "runtime.install",
        );
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_model_jobs(&mut self) {
        let result = self.persist_jobs_by_kind(
            ProductizationRecordKind::ModelJob,
            "model.download",
            |kind| kind.starts_with("model.download"),
        );
        self.record_state_journal_result(result);
    }

    fn persist_jobs_by_kind(
        &self,
        record_kind: ProductizationRecordKind,
        subject_id: &str,
        matches_kind: impl Fn(&str) -> bool,
    ) -> Result<(), StorageError> {
        let Some(storage) = &self.storage else {
            return Ok(());
        };
        let jobs = self
            .jobs
            .snapshot_jobs()
            .into_iter()
            .filter(|job| matches_kind(job.kind()))
            .map(|job| {
                serde_json::json!({
                    "id":job.id(),
                    "kind":job.kind(),
                    "state":job_state_to_str(job.state())
                })
            })
            .collect::<Vec<_>>();
        storage.put_productization_state(ProductizationStateRecord::new(
            record_kind,
            subject_id,
            serde_json::json!({"jobs":jobs}).to_string(),
        ))
    }
}

pub(crate) fn load_background_job_store(
    storage: &SqliteStore,
) -> Result<JobServiceStore, StorageError> {
    let mut snapshots = load_job_snapshots(
        storage,
        ProductizationRecordKind::RuntimeJob,
        "runtime.install",
        "runtime.install",
    )?;
    snapshots.extend(load_job_snapshots(
        storage,
        ProductizationRecordKind::ModelJob,
        "model.download",
        "model.download",
    )?);
    Ok(JobServiceStore::from_snapshots(snapshots))
}

fn load_job_snapshots(
    storage: &SqliteStore,
    kind: ProductizationRecordKind,
    subject_id: &str,
    fallback_kind: &str,
) -> Result<Vec<JobSnapshot>, StorageError> {
    let Some(record) = storage.get_productization_state(kind, subject_id)? else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(record.payload())
        .map_err(|error| StorageError::Sqlite(error.to_string()))?;
    Ok(value
        .get("jobs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|job| {
            JobSnapshot::new(
                string_field(job, "id", "job.0"),
                string_field(job, "kind", fallback_kind),
                job_state_from_str(&string_field(job, "state", "queued")),
            )
        })
        .collect())
}

pub(crate) fn job_state_to_str(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::AwaitingApproval => "awaiting_approval",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::Cancelled => "cancelled",
        JobState::Blocked => "blocked",
    }
}

fn job_state_from_str(value: &str) -> JobState {
    match value {
        "running" => JobState::Running,
        "awaiting_approval" => JobState::AwaitingApproval,
        "succeeded" => JobState::Succeeded,
        "failed" => JobState::Failed,
        "cancelled" => JobState::Cancelled,
        "blocked" => JobState::Blocked,
        _ => JobState::Queued,
    }
}
