use crate::{SecretRef, SecretValue, Vault, VaultError};
use std::collections::HashMap;

#[derive(Default)]
pub struct FakeVault {
    secrets: HashMap<SecretRef, SecretValue>,
}

impl Vault for FakeVault {
    fn put(&mut self, secret_ref: SecretRef, secret: SecretValue) -> Result<(), VaultError> {
        self.secrets.insert(secret_ref, secret);
        Ok(())
    }

    fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, VaultError> {
        self.secrets
            .get(secret_ref)
            .cloned()
            .ok_or_else(|| VaultError::SecretNotFound(secret_ref.clone()))
    }

    fn delete(&mut self, secret_ref: &SecretRef) -> Result<(), VaultError> {
        self.secrets.remove(secret_ref);
        Ok(())
    }
}
