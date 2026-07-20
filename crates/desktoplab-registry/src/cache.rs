use crate::{ManifestFamily, ManifestGroup};
use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct CachedRegistry {
    groups: HashMap<ManifestFamily, ManifestGroup>,
    storage_path: Option<PathBuf>,
}

impl CachedRegistry {
    pub fn store(&mut self, group: ManifestGroup) -> Result<(), crate::RegistryError> {
        if let Some(path) = &self.storage_path {
            persist_group(path, &group)?;
        }
        self.groups.insert(group.family(), group);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, family: ManifestFamily) -> Option<&ManifestGroup> {
        self.groups.get(&family)
    }

    pub fn with_storage_path(path: impl AsRef<Path>) -> Result<Self, crate::RegistryError> {
        let path = path.as_ref().to_path_buf();
        let store = SqliteStore::open(&path)?;
        store.apply_migrations()?;
        let mut cache = Self {
            groups: HashMap::new(),
            storage_path: Some(path),
        };
        for family in [
            ManifestFamily::Runtime,
            ManifestFamily::Model,
            ManifestFamily::Backend,
            ManifestFamily::Plugin,
        ] {
            if let Some(group) = load_group(&store, family)? {
                cache.groups.insert(family, group);
            }
        }
        Ok(cache)
    }
}

fn persist_group(path: &Path, group: &ManifestGroup) -> Result<(), crate::RegistryError> {
    let store = SqliteStore::open(path)?;
    store.apply_migrations()?;
    store.put_productization_state(ProductizationStateRecord::new(
        ProductizationRecordKind::RegistryCache,
        group.family().as_str(),
        serde_json::to_string(group)?,
    ))?;
    Ok(())
}

fn load_group(
    store: &SqliteStore,
    family: ManifestFamily,
) -> Result<Option<ManifestGroup>, crate::RegistryError> {
    let Some(record) =
        store.get_productization_state(ProductizationRecordKind::RegistryCache, family.as_str())?
    else {
        return Ok(None);
    };
    let group: ManifestGroup = serde_json::from_str(record.payload())?;
    if group.family() != family {
        return Err(crate::RegistryError::NoSafeCatalog(format!(
            "persisted {} catalog has wrong family {}",
            family.as_str(),
            group.family().as_str()
        )));
    }
    Ok(Some(group))
}
