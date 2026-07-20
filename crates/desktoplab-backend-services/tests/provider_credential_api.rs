use desktoplab_backend_services::{
    ProviderCredentialError, ProviderCredentialService, ProviderCredentialStore,
};
use desktoplab_vault::{FakeVault, SecretRef, SecretValue, Vault, VaultError};
use xtask::check_logical_line_limit;

#[test]
fn raw_provider_keys_never_enter_metadata_or_events() {
    let mut service =
        ProviderCredentialService::new(ProviderCredentialStore::default(), FakeVault::default());

    let account = service
        .register_provider(
            "provider.openai",
            "OpenAI",
            SecretValue::new("sk-live-secret"),
        )
        .expect("provider registration should store key in vault");

    assert!(account.secret_ref().as_uri().contains("provider.openai"));
    assert!(!format!("{account:?}").contains("sk-live-secret"));
    assert!(!service.event_log().join("\n").contains("sk-live-secret"));
}

#[test]
fn missing_vault_support_fails_closed() {
    let mut service =
        ProviderCredentialService::new(ProviderCredentialStore::default(), UnavailableVault);

    let error = service
        .register_provider(
            "provider.openai",
            "OpenAI",
            SecretValue::new("sk-live-secret"),
        )
        .expect_err("unavailable vault should fail closed");

    assert_eq!(
        error,
        ProviderCredentialError::VaultUnavailable("native vault unavailable".to_string())
    );
}

#[test]
fn connected_provider_does_not_approve_workspace_egress() {
    let mut service =
        ProviderCredentialService::new(ProviderCredentialStore::default(), FakeVault::default());

    service
        .register_provider(
            "provider.openai",
            "OpenAI",
            SecretValue::new("sk-live-secret"),
        )
        .unwrap();
    let readiness = service.readiness("provider.openai").unwrap();

    assert!(readiness.connected());
    assert!(!readiness.workspace_egress_allowed());
}

#[test]
fn credential_deletion_removes_vault_reference() {
    let mut service =
        ProviderCredentialService::new(ProviderCredentialStore::default(), FakeVault::default());
    let account = service
        .register_provider(
            "provider.openai",
            "OpenAI",
            SecretValue::new("sk-live-secret"),
        )
        .unwrap();
    let secret_ref = account.secret_ref().clone();

    service.delete_provider("provider.openai").unwrap();

    assert!(!service.has_secret(&secret_ref));
    assert!(service.readiness("provider.openai").is_none());
}

#[test]
fn provider_credential_api_source_stays_below_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/provider_credentials.rs",
        include_str!("../src/provider_credentials.rs"),
        300,
    )
    .expect("provider credential api source should stay below the line-count guard");
}

struct UnavailableVault;

impl Vault for UnavailableVault {
    fn put(&mut self, _secret_ref: SecretRef, _secret: SecretValue) -> Result<(), VaultError> {
        Err(VaultError::Unavailable(
            "native vault unavailable".to_string(),
        ))
    }

    fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, VaultError> {
        Err(VaultError::SecretNotFound(secret_ref.clone()))
    }

    fn delete(&mut self, _secret_ref: &SecretRef) -> Result<(), VaultError> {
        Ok(())
    }
}
