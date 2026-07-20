#![forbid(unsafe_code)]

mod fake;
mod macos_keychain;
mod metadata;
mod native_keyring;
mod platform;
mod secret;
mod vault;

pub use fake::FakeVault;
pub use macos_keychain::{KeychainCommandOutput, KeychainCommandRunner, MacOsKeychainVault};
pub use metadata::{AuthModeMetadata, CredentialMetadata, CredentialMetadataError};
pub use native_keyring::{
    NativeCredentialStore, NativeKeyringVault, NativeStoreError, SystemNativeCredentialStore,
};
pub use platform::{DegradedVaultReason, NativeVaultKind, OperatingSystem, VaultAdapterSelection};
pub use secret::{SecretRef, SecretRefParseError, SecretScope, SecretValue};
pub use vault::{Vault, VaultError};

pub fn put_current_native_secret(
    secret_ref: SecretRef,
    secret: SecretValue,
) -> Result<(), VaultError> {
    match VaultAdapterSelection::current() {
        VaultAdapterSelection::Available(_) => {
            NativeKeyringVault::desktoplab().put(secret_ref, secret)
        }
        VaultAdapterSelection::Degraded(reason) => {
            Err(VaultError::Unavailable(format!("{reason:?}")))
        }
    }
}

pub fn get_current_native_secret(secret_ref: &SecretRef) -> Result<SecretValue, VaultError> {
    match VaultAdapterSelection::current() {
        VaultAdapterSelection::Available(_) => NativeKeyringVault::desktoplab().get(secret_ref),
        VaultAdapterSelection::Degraded(reason) => {
            Err(VaultError::Unavailable(format!("{reason:?}")))
        }
    }
}

pub fn delete_current_native_secret(secret_ref: &SecretRef) -> Result<(), VaultError> {
    match VaultAdapterSelection::current() {
        VaultAdapterSelection::Available(_) => NativeKeyringVault::desktoplab().delete(secret_ref),
        VaultAdapterSelection::Degraded(reason) => {
            Err(VaultError::Unavailable(format!("{reason:?}")))
        }
    }
}
