#![forbid(unsafe_code)]

use desktoplab_policy::EgressClassification;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactVerification {
    Verified,
    Unsigned,
    ChecksumMismatch,
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductizationSecurityGateInput {
    pub binary: ArtifactVerification,
    pub model: ArtifactVerification,
    pub plugin_verified: bool,
    pub provider_egress: EgressClassification,
    pub protected_path: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProductizationSecurityGate {
    _private: (),
}

impl ProductizationSecurityGate {
    #[must_use]
    pub fn evaluate(&self, input: ProductizationSecurityGateInput) -> ProductizationSecurityReport {
        let mut reasons = Vec::new();
        if input.binary != ArtifactVerification::Verified {
            reasons.push("unsigned_binary".to_string());
        }
        if input.model != ArtifactVerification::Verified {
            reasons.push("model_verification_failed".to_string());
        }
        if !input.plugin_verified {
            reasons.push("unverified_plugin".to_string());
        }
        if input.provider_egress == EgressClassification::LocalOnly {
            reasons.push("provider_egress_denied".to_string());
        }
        if is_protected_path(&input.protected_path) {
            reasons.push("protected_path_local_only".to_string());
        }
        ProductizationSecurityReport { reasons }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductizationSecurityReport {
    reasons: Vec<String>,
}

impl ProductizationSecurityReport {
    #[must_use]
    pub fn is_denied(&self) -> bool {
        !self.reasons.is_empty()
    }

    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
}

fn is_protected_path(path: &str) -> bool {
    path == ".env" || path.starts_with(".git/") || path.starts_with(".ssh/")
}
