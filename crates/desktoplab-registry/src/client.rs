use crate::manifest::{REGISTRY_SCHEMA, SignedManifestGroup};
use crate::{
    CachedRegistry, ManifestFamily, ManifestGroup, RegistryError, RegistrySource, SignatureVerifier,
};

pub struct RegistryClient<S, V> {
    source: S,
    verifier: V,
    cache: CachedRegistry,
}

impl<S, V> RegistryClient<S, V>
where
    S: RegistrySource,
    V: SignatureVerifier,
{
    #[must_use]
    pub fn new(source: S, verifier: V, cache: CachedRegistry) -> Self {
        Self {
            source,
            verifier,
            cache,
        }
    }

    pub fn refresh_family(
        &mut self,
        family: ManifestFamily,
    ) -> Result<ManifestGroup, RegistryError> {
        match self.source.fetch_family(family) {
            Ok(raw) => {
                let group = self.parse_verified_group(family, &raw)?;
                self.cache.store(group.clone())?;
                Ok(group)
            }
            Err(error) => self
                .cache
                .get(family)
                .cloned()
                .map(ManifestGroup::mark_last_known_good)
                .ok_or(error),
        }
    }

    #[must_use]
    pub fn cache(&self) -> &CachedRegistry {
        &self.cache
    }

    fn parse_verified_group(
        &self,
        family: ManifestFamily,
        raw: &str,
    ) -> Result<ManifestGroup, RegistryError> {
        let signed: SignedManifestGroup = serde_json::from_str(raw)?;

        if signed.schema != REGISTRY_SCHEMA {
            return Err(RegistryError::InvalidManifest(format!(
                "manifest group has unsupported schema {}",
                signed.schema
            )));
        }

        if signed.family != family {
            return Err(RegistryError::InvalidManifest(format!(
                "manifest group has family {}, expected {}",
                signed.family.as_str(),
                family.as_str()
            )));
        }

        self.verifier.verify(raw, &signed.signature)?;

        for manifest in &signed.payload.manifests {
            manifest.validate_for_family(family)?;
        }

        let group = ManifestGroup::new(family, signed.payload.manifests);
        Ok(group)
    }
}
