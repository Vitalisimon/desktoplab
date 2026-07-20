use crate::{ManifestFamily, RegistryError};

pub trait RegistrySource {
    fn fetch_family(&self, family: ManifestFamily) -> Result<String, RegistryError>;
}
