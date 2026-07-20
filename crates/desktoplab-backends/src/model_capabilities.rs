use std::collections::BTreeSet;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{ModelToolProtocolCertification, ModelToolProtocolKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelCapabilityState {
    Confirmed,
    Unsupported,
    ProbeRequired,
}

impl ModelCapabilityState {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::Unsupported => "unsupported",
            Self::ProbeRequired => "probe_required",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendModelCapabilities {
    backend_id: String,
    model_id: String,
    version: Option<String>,
    context_window: Option<u64>,
    capabilities: BTreeSet<String>,
    capabilities_reported: bool,
    fingerprint: String,
    tool_protocol_certification: Option<ModelToolProtocolCertification>,
}

impl BackendModelCapabilities {
    #[must_use]
    pub fn reported(
        backend_id: impl Into<String>,
        model_id: impl Into<String>,
        version: Option<String>,
        context_window: Option<u64>,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::build(
            backend_id,
            model_id,
            version,
            context_window,
            capabilities,
            true,
        )
    }

    #[must_use]
    pub fn unverified(
        backend_id: impl Into<String>,
        model_id: impl Into<String>,
        version: Option<String>,
        context_window: Option<u64>,
    ) -> Self {
        Self::build(
            backend_id,
            model_id,
            version,
            context_window,
            std::iter::empty::<String>(),
            false,
        )
    }

    fn build(
        backend_id: impl Into<String>,
        model_id: impl Into<String>,
        version: Option<String>,
        context_window: Option<u64>,
        capabilities: impl IntoIterator<Item = impl Into<String>>,
        capabilities_reported: bool,
    ) -> Self {
        let backend_id = backend_id.into();
        let model_id = model_id.into();
        let capabilities = capabilities
            .into_iter()
            .map(Into::into)
            .collect::<BTreeSet<_>>();
        let fingerprint = capability_fingerprint(
            &backend_id,
            &model_id,
            version.as_deref(),
            context_window,
            &capabilities,
            capabilities_reported,
        );
        Self {
            backend_id,
            model_id,
            version,
            context_window,
            capabilities,
            capabilities_reported,
            fingerprint,
            tool_protocol_certification: None,
        }
    }

    #[must_use]
    pub fn backend_id(&self) -> &str {
        &self.backend_id
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    #[must_use]
    pub fn context_window(&self) -> Option<u64> {
        self.context_window
    }

    #[must_use]
    pub fn with_tool_protocol_certification(
        mut self,
        certification: ModelToolProtocolCertification,
    ) -> Self {
        if certification.is_for(&self.fingerprint) {
            self.tool_protocol_certification = Some(certification);
        }
        self
    }

    #[must_use]
    pub fn tool_protocol_certification(&self) -> Option<&ModelToolProtocolCertification> {
        self.tool_protocol_certification.as_ref()
    }

    #[must_use]
    pub fn tool_protocol_certified(&self) -> bool {
        self.tool_protocol_certification
            .as_ref()
            .is_some_and(|certification| certification.is_certified_for(&self.fingerprint))
    }

    #[must_use]
    pub fn tool_protocol_kind(&self) -> Option<ModelToolProtocolKind> {
        self.tool_protocol_certification
            .as_ref()
            .filter(|certification| certification.is_certified_for(&self.fingerprint))
            .and_then(ModelToolProtocolCertification::protocol)
    }

    #[must_use]
    pub fn capability_state(&self, capability: &str) -> ModelCapabilityState {
        if !self.capabilities_reported {
            ModelCapabilityState::ProbeRequired
        } else if self.capabilities.contains(capability) {
            ModelCapabilityState::Confirmed
        } else {
            ModelCapabilityState::Unsupported
        }
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "backendId":self.backend_id,
            "modelId":self.model_id,
            "version":self.version,
            "contextWindow":self.context_window,
            "capabilities":self.capabilities,
            "capabilitiesReported":self.capabilities_reported,
            "fingerprint":self.fingerprint,
            "toolProtocolCertification":self.tool_protocol_certification.as_ref().map(ModelToolProtocolCertification::to_json)
        })
    }

    #[must_use]
    pub fn from_json(value: &Value) -> Option<Self> {
        let backend_id = value.get("backendId")?.as_str()?;
        let model_id = value.get("modelId")?.as_str()?;
        let version = value
            .get("version")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let context_window = value.get("contextWindow").and_then(Value::as_u64);
        let capabilities = value
            .get("capabilities")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string);
        let capabilities_reported = value
            .get("capabilitiesReported")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mut profile = Self::build(
            backend_id,
            model_id,
            version,
            context_window,
            capabilities,
            capabilities_reported,
        );
        if let Some(certification) = value
            .get("toolProtocolCertification")
            .and_then(ModelToolProtocolCertification::from_json)
            && certification.is_for(profile.fingerprint())
        {
            profile.tool_protocol_certification = Some(certification);
        }
        Some(profile)
    }
}

fn capability_fingerprint(
    backend_id: &str,
    model_id: &str,
    version: Option<&str>,
    context_window: Option<u64>,
    capabilities: &BTreeSet<String>,
    capabilities_reported: bool,
) -> String {
    let canonical = json!({
        "backendId":backend_id,
        "modelId":model_id,
        "version":version,
        "contextWindow":context_window,
        "capabilities":capabilities,
        "capabilitiesReported":capabilities_reported
    });
    format!("sha256:{:x}", Sha256::digest(canonical.to_string()))
}
