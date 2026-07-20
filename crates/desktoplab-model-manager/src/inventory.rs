use std::sync::{Arc, Mutex};

use desktoplab_compatibility::{CompatibilityCatalog, CompatibilityEngine, MatchRequest};
use desktoplab_runtime::RuntimeId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelInventorySource {
    Registry,
    LocalRuntime,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelInstallState {
    Installed,
    NotInstalled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelInventoryEntry {
    model_id: String,
    source: ModelInventorySource,
    install_state: ModelInstallState,
    runtime_id: Option<String>,
    recommended: bool,
    compatibility_reason: String,
    provenance: ModelProvenance,
}

impl ModelInventoryEntry {
    fn registry(
        model_id: impl Into<String>,
        runtime_id: impl Into<String>,
        recommended: bool,
        compatibility_reason: impl Into<String>,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            source: ModelInventorySource::Registry,
            install_state: ModelInstallState::NotInstalled,
            runtime_id: Some(runtime_id.into()),
            recommended,
            compatibility_reason: compatibility_reason.into(),
            provenance: ModelProvenance::registry("compatibility_catalog", "not_verified_locally"),
        }
    }

    fn local(model_id: impl Into<String>, runtime_id: RuntimeId) -> Self {
        let runtime_id = runtime_id.as_str().to_string();
        Self {
            model_id: model_id.into(),
            source: ModelInventorySource::LocalRuntime,
            install_state: ModelInstallState::Installed,
            runtime_id: Some(runtime_id.clone()),
            recommended: false,
            compatibility_reason: "local runtime inventory".to_string(),
            provenance: ModelProvenance::local(&runtime_id, "local_runtime_inventory"),
        }
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn source(&self) -> ModelInventorySource {
        self.source
    }

    #[must_use]
    pub fn install_state(&self) -> ModelInstallState {
        self.install_state
    }

    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    #[must_use]
    pub fn is_recommended(&self) -> bool {
        self.recommended
    }

    #[must_use]
    pub fn compatibility_reason(&self) -> &str {
        &self.compatibility_reason
    }

    #[must_use]
    pub fn provenance(&self) -> &ModelProvenance {
        &self.provenance
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelProvenance {
    catalog_source: String,
    runtime_id: Option<String>,
    pull_ref: Option<String>,
    verification_state: String,
}

impl ModelProvenance {
    #[must_use]
    pub fn registry(catalog_source: &str, verification_state: &str) -> Self {
        Self {
            catalog_source: catalog_source.to_string(),
            runtime_id: None,
            pull_ref: None,
            verification_state: verification_state.to_string(),
        }
    }

    #[must_use]
    pub fn local(runtime_id: &str, verification_state: &str) -> Self {
        Self {
            catalog_source: "local_runtime_inventory".to_string(),
            runtime_id: Some(runtime_id.to_string()),
            pull_ref: None,
            verification_state: verification_state.to_string(),
        }
    }

    #[must_use]
    pub fn with_pull_ref(mut self, pull_ref: &str) -> Self {
        self.pull_ref = Some(pull_ref.to_string());
        self
    }

    #[must_use]
    pub fn catalog_source(&self) -> &str {
        &self.catalog_source
    }

    #[must_use]
    pub fn runtime_id(&self) -> Option<&str> {
        self.runtime_id.as_deref()
    }

    #[must_use]
    pub fn pull_ref(&self) -> Option<&str> {
        self.pull_ref.as_deref()
    }

    #[must_use]
    pub fn verification_state(&self) -> &str {
        &self.verification_state
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelInventorySnapshot {
    registry_models: Vec<ModelInventoryEntry>,
    local_models: Vec<ModelInventoryEntry>,
}

impl ModelInventorySnapshot {
    #[must_use]
    pub fn registry_models(&self) -> &[ModelInventoryEntry] {
        &self.registry_models
    }

    #[must_use]
    pub fn local_models(&self) -> &[ModelInventoryEntry] {
        &self.local_models
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryModelInventoryStore {
    local_models: Arc<Mutex<Vec<ModelInventoryEntry>>>,
}

impl InMemoryModelInventoryStore {
    fn record_local_model(&self, runtime_id: RuntimeId, model_id: impl Into<String>) {
        self.local_models
            .lock()
            .expect("model inventory store lock should not be poisoned")
            .push(ModelInventoryEntry::local(model_id, runtime_id));
    }

    fn local_inventory(&self) -> Vec<ModelInventoryEntry> {
        self.local_models
            .lock()
            .expect("model inventory store lock should not be poisoned")
            .clone()
    }
}

#[derive(Clone, Debug)]
pub struct ModelInventoryService {
    store: InMemoryModelInventoryStore,
}

impl ModelInventoryService {
    #[must_use]
    pub fn new(store: InMemoryModelInventoryStore) -> Self {
        Self { store }
    }

    pub fn record_local_model(&mut self, runtime_id: RuntimeId, model_id: impl Into<String>) {
        self.store.record_local_model(runtime_id, model_id);
    }

    #[must_use]
    pub fn local_inventory(&self) -> Vec<ModelInventoryEntry> {
        self.store.local_inventory()
    }

    #[must_use]
    pub fn inventory_for_runtime(
        &self,
        catalog: &CompatibilityCatalog,
        engine: &CompatibilityEngine,
        runtime_id: &str,
    ) -> ModelInventorySnapshot {
        let registry_models = catalog
            .models()
            .iter()
            .map(|model| {
                let decision = engine.evaluate(MatchRequest::new(runtime_id, model.id()));
                let mut entry = ModelInventoryEntry::registry(
                    model.id(),
                    runtime_id,
                    decision.is_recommended(),
                    decision.reason(),
                );
                entry.provenance =
                    ModelProvenance::registry("compatibility_catalog", "not_verified_locally")
                        .with_pull_ref(model.id());
                entry
            })
            .collect();

        ModelInventorySnapshot {
            registry_models,
            local_models: self.store.local_inventory(),
        }
    }
}
