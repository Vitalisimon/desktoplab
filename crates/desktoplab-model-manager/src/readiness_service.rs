use std::sync::{Arc, Mutex};

use desktoplab_runtime::RuntimeId;

use crate::{ModelReadiness, ModelVerification};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelVerificationReport {
    runtime_id: RuntimeId,
    model_id: String,
    verification: ModelVerification,
}

impl ModelVerificationReport {
    #[must_use]
    pub fn passed(runtime_id: RuntimeId, model_id: impl Into<String>) -> Self {
        Self {
            runtime_id,
            model_id: model_id.into(),
            verification: ModelVerification::passed(),
        }
    }

    #[must_use]
    pub fn failed(
        runtime_id: RuntimeId,
        model_id: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            runtime_id,
            model_id: model_id.into(),
            verification: ModelVerification::failed(reason),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelRouteStatus {
    Ready,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRouteReadiness {
    status: ModelRouteStatus,
    reason: Option<String>,
}

impl ModelRouteReadiness {
    fn ready() -> Self {
        Self {
            status: ModelRouteStatus::Ready,
            reason: None,
        }
    }

    fn blocked(reason: impl Into<String>) -> Self {
        Self {
            status: ModelRouteStatus::Blocked,
            reason: Some(reason.into()),
        }
    }

    #[must_use]
    pub fn status(&self) -> ModelRouteStatus {
        self.status
    }

    #[must_use]
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ModelReadinessRecord {
    runtime_id: RuntimeId,
    model_id: String,
    readiness: ModelReadiness,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryModelReadinessStore {
    records: Arc<Mutex<Vec<ModelReadinessRecord>>>,
}

impl InMemoryModelReadinessStore {
    fn upsert(&self, record: ModelReadinessRecord) {
        let mut records = self
            .records
            .lock()
            .expect("model readiness store lock should not be poisoned");
        records.retain(|existing| {
            existing.runtime_id != record.runtime_id || existing.model_id != record.model_id
        });
        records.push(record);
    }

    fn get(&self, runtime_id: &str, model_id: &str) -> Option<ModelReadinessRecord> {
        self.records
            .lock()
            .expect("model readiness store lock should not be poisoned")
            .iter()
            .find(|record| record.runtime_id.as_str() == runtime_id && record.model_id == model_id)
            .cloned()
    }
}

#[derive(Clone, Debug)]
pub struct ModelReadinessService {
    store: InMemoryModelReadinessStore,
}

impl ModelReadinessService {
    #[must_use]
    pub fn new(store: InMemoryModelReadinessStore) -> Self {
        Self { store }
    }

    pub fn apply_verification(&mut self, report: ModelVerificationReport) -> ModelReadiness {
        let readiness = ModelReadiness::from_verification(report.verification);
        self.store.upsert(ModelReadinessRecord {
            runtime_id: report.runtime_id,
            model_id: report.model_id,
            readiness: readiness.clone(),
        });
        readiness
    }

    #[must_use]
    pub fn route_readiness(&self, runtime_id: &str, model_id: &str) -> ModelRouteReadiness {
        let Some(record) = self.store.get(runtime_id, model_id) else {
            return ModelRouteReadiness::blocked("model_unavailable");
        };

        if record.readiness.is_ready() {
            ModelRouteReadiness::ready()
        } else {
            ModelRouteReadiness::blocked(
                record
                    .readiness
                    .reason()
                    .unwrap_or("model_verification_failed"),
            )
        }
    }
}
