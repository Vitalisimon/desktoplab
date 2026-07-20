use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelFootprintTier {
    Gb100,
    Gb500,
    Tb1,
    MultiTb,
}

impl ModelFootprintTier {
    #[must_use]
    pub fn for_bytes(bytes: u64) -> Self {
        match bytes {
            0..=100_000_000_000 => Self::Gb100,
            100_000_000_001..=500_000_000_000 => Self::Gb500,
            500_000_000_001..=1_000_000_000_000 => Self::Tb1,
            _ => Self::MultiTb,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStoreCapacity {
    available_bytes: u64,
    reserve_bytes: u64,
}

impl ModelStoreCapacity {
    #[must_use]
    pub fn new(available_bytes: u64, reserve_bytes: u64) -> Self {
        Self {
            available_bytes,
            reserve_bytes,
        }
    }

    #[must_use]
    pub fn usable_bytes(&self) -> u64 {
        self.available_bytes.saturating_sub(self.reserve_bytes)
    }

    #[must_use]
    pub fn forecast(&self, artifact_bytes: u64, existing_partial_bytes: u64) -> ModelStoreForecast {
        let remaining_bytes = artifact_bytes.saturating_sub(existing_partial_bytes);
        ModelStoreForecast {
            tier: ModelFootprintTier::for_bytes(artifact_bytes),
            artifact_bytes,
            existing_partial_bytes,
            remaining_bytes,
            available_bytes: self.available_bytes,
            reserve_bytes: self.reserve_bytes,
            fits: remaining_bytes <= self.usable_bytes(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStoreForecast {
    tier: ModelFootprintTier,
    artifact_bytes: u64,
    existing_partial_bytes: u64,
    remaining_bytes: u64,
    available_bytes: u64,
    reserve_bytes: u64,
    fits: bool,
}

impl ModelStoreForecast {
    #[must_use]
    pub fn tier(&self) -> ModelFootprintTier {
        self.tier
    }

    #[must_use]
    pub fn remaining_bytes(&self) -> u64 {
        self.remaining_bytes
    }

    #[must_use]
    pub fn available_bytes(&self) -> u64 {
        self.available_bytes
    }

    #[must_use]
    pub fn reserve_bytes(&self) -> u64 {
        self.reserve_bytes
    }

    #[must_use]
    pub fn fits(&self) -> bool {
        self.fits
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelStoreEntry {
    model_id: String,
    path: PathBuf,
    size_bytes: u64,
    pinned: bool,
    last_used_epoch_seconds: u64,
}

impl ModelStoreEntry {
    #[must_use]
    pub fn new(
        model_id: impl Into<String>,
        path: impl Into<PathBuf>,
        size_bytes: u64,
        last_used_epoch_seconds: u64,
    ) -> Self {
        Self {
            model_id: model_id.into(),
            path: path.into(),
            size_bytes,
            pinned: false,
            last_used_epoch_seconds,
        }
    }

    #[must_use]
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub fn is_pinned(&self) -> bool {
        self.pinned
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelStoreInventory {
    entries: Vec<ModelStoreEntry>,
}

impl ModelStoreInventory {
    #[must_use]
    pub fn new(entries: Vec<ModelStoreEntry>) -> Self {
        Self { entries }
    }

    #[must_use]
    pub fn entries(&self) -> &[ModelStoreEntry] {
        &self.entries
    }

    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.entries.iter().map(ModelStoreEntry::size_bytes).sum()
    }

    #[must_use]
    pub fn eviction_recommendation(&self, bytes_needed: u64) -> ModelEvictionRecommendation {
        let mut candidates: Vec<&ModelStoreEntry> =
            self.entries.iter().filter(|entry| !entry.pinned).collect();
        candidates.sort_by_key(|entry| entry.last_used_epoch_seconds);
        let mut reclaimed_bytes = 0;
        let mut model_ids = Vec::new();
        for entry in candidates {
            if reclaimed_bytes >= bytes_needed {
                break;
            }
            reclaimed_bytes = reclaimed_bytes.saturating_add(entry.size_bytes);
            model_ids.push(entry.model_id.clone());
        }
        ModelEvictionRecommendation {
            model_ids,
            reclaimed_bytes,
            sufficient: reclaimed_bytes >= bytes_needed,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelEvictionRecommendation {
    model_ids: Vec<String>,
    reclaimed_bytes: u64,
    sufficient: bool,
}

impl ModelEvictionRecommendation {
    #[must_use]
    pub fn model_ids(&self) -> &[String] {
        &self.model_ids
    }

    #[must_use]
    pub fn reclaimed_bytes(&self) -> u64 {
        self.reclaimed_bytes
    }

    #[must_use]
    pub fn is_sufficient(&self) -> bool {
        self.sufficient
    }
}
