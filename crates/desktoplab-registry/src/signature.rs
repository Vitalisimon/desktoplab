use crate::RegistryError;

pub trait SignatureVerifier {
    fn verify(&self, payload: &str, signature: &str) -> Result<(), RegistryError>;
}
