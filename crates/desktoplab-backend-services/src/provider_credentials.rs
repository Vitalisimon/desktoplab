use std::sync::{Arc, Mutex};

use desktoplab_vault::{SecretRef, SecretScope, SecretValue, Vault, VaultError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderCredentialError {
    VaultUnavailable(String),
    ProviderMissing(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderAccount {
    provider_id: String,
    display_name: String,
    secret_ref: SecretRef,
}

impl ProviderAccount {
    #[must_use]
    pub fn secret_ref(&self) -> &SecretRef {
        &self.secret_ref
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderReadiness {
    connected: bool,
    workspace_egress_allowed: bool,
}

impl ProviderReadiness {
    #[must_use]
    pub fn connected(&self) -> bool {
        self.connected
    }

    #[must_use]
    pub fn workspace_egress_allowed(&self) -> bool {
        self.workspace_egress_allowed
    }
}

#[derive(Clone, Debug, Default)]
pub struct ProviderCredentialStore {
    inner: Arc<Mutex<ProviderCredentialData>>,
}

#[derive(Clone, Debug, Default)]
struct ProviderCredentialData {
    accounts: Vec<ProviderAccount>,
    events: Vec<String>,
}

#[derive(Debug)]
pub struct ProviderCredentialService<V> {
    store: ProviderCredentialStore,
    vault: V,
}

impl<V> ProviderCredentialService<V>
where
    V: Vault,
{
    #[must_use]
    pub fn new(store: ProviderCredentialStore, vault: V) -> Self {
        Self { store, vault }
    }

    pub fn register_provider(
        &mut self,
        provider_id: impl Into<String>,
        display_name: impl Into<String>,
        secret: SecretValue,
    ) -> Result<ProviderAccount, ProviderCredentialError> {
        let provider_id = provider_id.into();
        let secret_ref = SecretRef::new(SecretScope::Provider, format!("{provider_id}:api-key"));
        self.vault
            .put(secret_ref.clone(), secret)
            .map_err(provider_error)?;
        let account = ProviderAccount {
            provider_id: provider_id.clone(),
            display_name: display_name.into(),
            secret_ref,
        };
        let mut data = self
            .store
            .inner
            .lock()
            .expect("provider credential store lock should not be poisoned");
        data.accounts.push(account.clone());
        data.events
            .push(format!("provider_registered:{provider_id}"));
        Ok(account)
    }

    #[must_use]
    pub fn readiness(&self, provider_id: &str) -> Option<ProviderReadiness> {
        self.store
            .inner
            .lock()
            .expect("provider credential store lock should not be poisoned")
            .accounts
            .iter()
            .any(|account| account.provider_id == provider_id)
            .then_some(ProviderReadiness {
                connected: true,
                workspace_egress_allowed: false,
            })
    }

    pub fn delete_provider(&mut self, provider_id: &str) -> Result<(), ProviderCredentialError> {
        let mut data = self
            .store
            .inner
            .lock()
            .expect("provider credential store lock should not be poisoned");
        let Some(index) = data
            .accounts
            .iter()
            .position(|account| account.provider_id == provider_id)
        else {
            return Err(ProviderCredentialError::ProviderMissing(
                provider_id.to_string(),
            ));
        };
        let account = data.accounts.remove(index);
        self.vault
            .delete(&account.secret_ref)
            .map_err(provider_error)?;
        data.events.push(format!("provider_deleted:{provider_id}"));
        Ok(())
    }

    #[must_use]
    pub fn has_secret(&self, secret_ref: &SecretRef) -> bool {
        self.vault.get(secret_ref).is_ok()
    }

    #[must_use]
    pub fn event_log(&self) -> Vec<String> {
        self.store
            .inner
            .lock()
            .expect("provider credential store lock should not be poisoned")
            .events
            .clone()
    }
}

fn provider_error(error: VaultError) -> ProviderCredentialError {
    match error {
        VaultError::Unavailable(reason) => ProviderCredentialError::VaultUnavailable(reason),
        VaultError::SecretNotFound(secret_ref) => {
            ProviderCredentialError::ProviderMissing(secret_ref.as_uri())
        }
    }
}
