use crate::{SecretRef, SecretValue, Vault, VaultError};
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeychainCommandOutput {
    stdout: String,
}

impl KeychainCommandOutput {
    #[must_use]
    pub fn stdout(stdout: String) -> Self {
        Self { stdout }
    }

    #[must_use]
    pub fn stdout_text(&self) -> &str {
        &self.stdout
    }
}

pub trait KeychainCommandRunner {
    fn run_security(&self, args: &[String]) -> Result<KeychainCommandOutput, VaultError>;
}

#[derive(Clone, Debug, Default)]
pub struct SystemKeychainCommandRunner;

impl KeychainCommandRunner for SystemKeychainCommandRunner {
    fn run_security(&self, args: &[String]) -> Result<KeychainCommandOutput, VaultError> {
        let output = Command::new("/usr/bin/security")
            .args(args)
            .output()
            .map_err(|error| {
                VaultError::Unavailable(format!("macOS Keychain unavailable: {error}"))
            })?;
        if !output.status.success() {
            return Err(VaultError::Unavailable(
                String::from_utf8_lossy(&output.stderr).trim().to_string(),
            ));
        }
        Ok(KeychainCommandOutput::stdout(
            String::from_utf8_lossy(&output.stdout)
                .trim_end()
                .to_string(),
        ))
    }
}

pub struct MacOsKeychainVault<R = SystemKeychainCommandRunner> {
    service: String,
    runner: R,
}

impl MacOsKeychainVault<SystemKeychainCommandRunner> {
    #[must_use]
    pub fn desktoplab() -> Self {
        Self::with_runner("DesktopLab", SystemKeychainCommandRunner)
    }
}

impl<R: KeychainCommandRunner> MacOsKeychainVault<R> {
    #[must_use]
    pub fn with_runner(service: impl Into<String>, runner: R) -> Self {
        Self {
            service: service.into(),
            runner,
        }
    }

    fn account(&self, secret_ref: &SecretRef) -> String {
        format!(
            "desktoplab:{}:{}",
            secret_ref.scope().as_uri_segment(),
            secret_ref.id()
        )
    }
}

impl<R: KeychainCommandRunner> Vault for MacOsKeychainVault<R> {
    fn put(&mut self, secret_ref: SecretRef, secret: SecretValue) -> Result<(), VaultError> {
        let _ = self.delete(&secret_ref);
        self.runner
            .run_security(&[
                "add-generic-password".to_string(),
                "-a".to_string(),
                self.account(&secret_ref),
                "-s".to_string(),
                self.service.clone(),
                "-w".to_string(),
                secret.expose_for_adapter().to_string(),
                "-U".to_string(),
            ])
            .map(|_| ())
    }

    fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, VaultError> {
        self.runner
            .run_security(&[
                "find-generic-password".to_string(),
                "-a".to_string(),
                self.account(secret_ref),
                "-s".to_string(),
                self.service.clone(),
                "-w".to_string(),
            ])
            .map(|output| SecretValue::new(output.stdout_text()))
    }

    fn delete(&mut self, secret_ref: &SecretRef) -> Result<(), VaultError> {
        self.runner
            .run_security(&[
                "delete-generic-password".to_string(),
                "-a".to_string(),
                self.account(secret_ref),
                "-s".to_string(),
                self.service.clone(),
            ])
            .map(|_| ())
    }
}
