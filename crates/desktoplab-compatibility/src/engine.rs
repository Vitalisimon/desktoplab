use crate::{
    AcceleratorConfidence, Channel, ChannelPolicy, CompatibilityCatalog, CompatibilityDecision,
    CompatibilityStatus, HardwareProfile, LocalOverride, RecommendationDecision,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchRequest {
    runtime_id: String,
    model_id: String,
}

impl MatchRequest {
    #[must_use]
    pub fn new(runtime_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            model_id: model_id.into(),
        }
    }

    #[must_use]
    pub fn runtime_id(&self) -> &str {
        &self.runtime_id
    }

    #[must_use]
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

pub struct CompatibilityEngine {
    catalog: CompatibilityCatalog,
    hardware: Option<HardwareProfile>,
    channel_policy: ChannelPolicy,
    local_override: LocalOverride,
    minimum_evidence_score: u8,
}

impl CompatibilityEngine {
    #[must_use]
    pub fn new(catalog: CompatibilityCatalog) -> Self {
        Self {
            catalog,
            hardware: None,
            channel_policy: ChannelPolicy::StableAndBeta,
            local_override: LocalOverride::none(),
            minimum_evidence_score: 0,
        }
    }

    #[must_use]
    pub fn with_hardware(mut self, hardware: HardwareProfile) -> Self {
        self.hardware = Some(hardware);
        self
    }

    #[must_use]
    pub fn with_channel_policy(mut self, channel_policy: ChannelPolicy) -> Self {
        self.channel_policy = channel_policy;
        self
    }

    #[must_use]
    pub fn with_local_override(mut self, local_override: LocalOverride) -> Self {
        self.local_override = local_override;
        self
    }

    #[must_use]
    pub fn with_minimum_evidence_score(mut self, minimum_evidence_score: u8) -> Self {
        self.minimum_evidence_score = minimum_evidence_score;
        self
    }

    #[must_use]
    pub fn evaluate(&self, request: MatchRequest) -> CompatibilityDecision {
        if let Some(blocked) = self
            .catalog
            .blocked_combination(&request.runtime_id, &request.model_id)
        {
            return blocked_decision(blocked.reason());
        }

        let Some(runtime) = self.catalog.runtime(&request.runtime_id) else {
            return unsupported("runtime manifest is missing");
        };
        let Some(model) = self.catalog.model(&request.model_id) else {
            return unsupported("model manifest is missing");
        };

        if !self.channel_allowed(runtime.channel()) {
            return blocked_decision(format!(
                "channel {} is blocked by policy",
                runtime.channel().as_str()
            ));
        }
        if !self.channel_allowed(model.channel()) {
            return blocked_decision(format!(
                "channel {} is blocked by policy",
                model.channel().as_str()
            ));
        }

        if !runtime.supports_format(model.format()) {
            return unsupported(format!(
                "runtime does not support model format {}",
                model.format()
            ));
        }

        if let Some(hardware) = &self.hardware {
            if model.required_memory_gb() > hardware.memory_gb() {
                return unsupported("model requires more memory than current hardware");
            }
            if model_requires_accelerator_confidence(model.required_memory_gb())
                && hardware.accelerator().confidence() == AcceleratorConfidence::Unknown
            {
                return CompatibilityDecision::new(
                    CompatibilityStatus::Compatible,
                    RecommendationDecision::CompatibleNotRecommended,
                    "compatible but accelerator confidence is unknown",
                );
            }
        }

        let evidence_score = self.catalog.strongest_evidence_score().unwrap_or(0);
        if evidence_score < self.minimum_evidence_score {
            return CompatibilityDecision::new(
                CompatibilityStatus::Compatible,
                RecommendationDecision::CompatibleNotRecommended,
                "compatible but insufficient evidence for recommendation",
            );
        }

        CompatibilityDecision::new(
            CompatibilityStatus::Recommended,
            RecommendationDecision::Recommended,
            "compatible with current policy and evidence",
        )
    }

    fn channel_allowed(&self, channel: Channel) -> bool {
        match (self.channel_policy, channel) {
            (_, Channel::Stable) => true,
            (ChannelPolicy::StableAndBeta | ChannelPolicy::AllowExperimental, Channel::Beta) => {
                true
            }
            (ChannelPolicy::AllowExperimental, Channel::Experimental) => true,
            (ChannelPolicy::StableOnly, Channel::Experimental)
                if self.local_override.allows_experimental() =>
            {
                true
            }
            (_, _) => false,
        }
    }
}

fn model_requires_accelerator_confidence(required_memory_gb: u32) -> bool {
    required_memory_gb >= 32
}

fn blocked_decision(reason: impl Into<String>) -> CompatibilityDecision {
    CompatibilityDecision::new(
        CompatibilityStatus::Blocked,
        RecommendationDecision::NotRecommended,
        reason,
    )
}

fn unsupported(reason: impl Into<String>) -> CompatibilityDecision {
    CompatibilityDecision::new(
        CompatibilityStatus::Unsupported,
        RecommendationDecision::NotRecommended,
        reason,
    )
}
