use keyring::v1::{Entry, Error as KeyringError};

use crate::{SecretRef, SecretValue, Vault, VaultError};

pub trait NativeCredentialStore {
    fn put(&self, service: &str, account: &str, secret: &str) -> Result<(), NativeStoreError>;

    fn get(&self, service: &str, account: &str) -> Result<String, NativeStoreError>;

    fn delete(&self, service: &str, account: &str) -> Result<(), NativeStoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NativeStoreError {
    Missing,
    Unavailable(String),
}

#[derive(Clone, Debug, Default)]
pub struct SystemNativeCredentialStore;

impl NativeCredentialStore for SystemNativeCredentialStore {
    fn put(&self, service: &str, account: &str, secret: &str) -> Result<(), NativeStoreError> {
        entry(service, account)?
            .set_password(secret)
            .map_err(map_error)
    }

    fn get(&self, service: &str, account: &str) -> Result<String, NativeStoreError> {
        entry(service, account)?.get_password().map_err(map_error)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), NativeStoreError> {
        entry(service, account)?
            .delete_credential()
            .map_err(map_error)
    }
}

fn entry(service: &str, account: &str) -> Result<Entry, NativeStoreError> {
    Entry::new(service, account).map_err(map_error)
}

fn map_error(error: KeyringError) -> NativeStoreError {
    match error {
        KeyringError::NoEntry => NativeStoreError::Missing,
        other => NativeStoreError::Unavailable(other.to_string()),
    }
}

pub struct NativeKeyringVault<S = SystemNativeCredentialStore> {
    service: String,
    store: S,
}

impl NativeKeyringVault<SystemNativeCredentialStore> {
    #[must_use]
    pub fn desktoplab() -> Self {
        Self::with_store("DesktopLab", SystemNativeCredentialStore)
    }
}

impl<S: NativeCredentialStore> NativeKeyringVault<S> {
    #[must_use]
    pub fn with_store(service: impl Into<String>, store: S) -> Self {
        Self {
            service: service.into(),
            store,
        }
    }

    fn account(secret_ref: &SecretRef) -> String {
        format!(
            "desktoplab:{}:{}",
            secret_ref.scope().as_uri_segment(),
            secret_ref.id()
        )
    }
}

impl<S: NativeCredentialStore> Vault for NativeKeyringVault<S> {
    fn put(&mut self, secret_ref: SecretRef, secret: SecretValue) -> Result<(), VaultError> {
        self.store
            .put(
                &self.service,
                &Self::account(&secret_ref),
                secret.expose_for_adapter(),
            )
            .map_err(|error| vault_error(error, secret_ref))
    }

    fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, VaultError> {
        self.store
            .get(&self.service, &Self::account(secret_ref))
            .map(SecretValue::new)
            .map_err(|error| vault_error(error, secret_ref.clone()))
    }

    fn delete(&mut self, secret_ref: &SecretRef) -> Result<(), VaultError> {
        self.store
            .delete(&self.service, &Self::account(secret_ref))
            .map_err(|error| vault_error(error, secret_ref.clone()))
    }
}

fn vault_error(error: NativeStoreError, secret_ref: SecretRef) -> VaultError {
    match error {
        NativeStoreError::Missing => VaultError::SecretNotFound(secret_ref),
        NativeStoreError::Unavailable(reason) => VaultError::Unavailable(reason),
    }
}
