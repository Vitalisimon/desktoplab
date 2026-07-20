#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityStatus {
    Recommended,
    Compatible,
    Unsupported,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecommendationDecision {
    Recommended,
    CompatibleNotRecommended,
    NotRecommended,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompatibilityDecision {
    status: CompatibilityStatus,
    recommendation: RecommendationDecision,
    reason: String,
}

impl CompatibilityDecision {
    #[must_use]
    pub fn new(
        status: CompatibilityStatus,
        recommendation: RecommendationDecision,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            status,
            recommendation,
            reason: reason.into(),
        }
    }

    #[must_use]
    pub fn status(&self) -> CompatibilityStatus {
        self.status
    }

    #[must_use]
    pub fn recommendation(&self) -> RecommendationDecision {
        self.recommendation
    }

    #[must_use]
    pub fn reason(&self) -> &str {
        &self.reason
    }

    #[must_use]
    pub fn is_recommended(&self) -> bool {
        self.recommendation == RecommendationDecision::Recommended
    }
}
