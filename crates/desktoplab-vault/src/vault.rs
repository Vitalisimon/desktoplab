use crate::{SecretRef, SecretValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VaultError {
    SecretNotFound(SecretRef),
    Unavailable(String),
}

pub trait Vault {
    fn put(&mut self, secret_ref: SecretRef, secret: SecretValue) -> Result<(), VaultError>;

    fn get(&self, secret_ref: &SecretRef) -> Result<SecretValue, VaultError>;

    fn delete(&mut self, secret_ref: &SecretRef) -> Result<(), VaultError>;
}
