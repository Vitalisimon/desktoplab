#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ProviderCandidateKind {
    Local,
    Cloud,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderCandidate {
    id: String,
    capabilities: Vec<String>,
    kind: ProviderCandidateKind,
    cost_hint: Option<String>,
}

impl ProviderCandidate {
    #[must_use]
    pub(super) fn new(
        id: impl Into<String>,
        capabilities: &[&str],
        kind: ProviderCandidateKind,
        cost_hint: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            capabilities: capabilities.iter().map(ToString::to_string).collect(),
            kind,
            cost_hint,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProviderRoutePreference {
    LocalFirst,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRoutePlanner {
    preference: ProviderRoutePreference,
}

impl ProviderRoutePlanner {
    #[must_use]
    pub fn new(preference: ProviderRoutePreference) -> Self {
        Self { preference }
    }

    #[must_use]
    pub fn select(
        &self,
        required: &[&str],
        mut candidates: Vec<ProviderCandidate>,
    ) -> ProviderRouteDecision {
        let matches_required = |candidate: &ProviderCandidate| {
            required.iter().all(|required| {
                candidate
                    .capabilities
                    .iter()
                    .any(|capability| capability == required)
            })
        };
        if self.preference == ProviderRoutePreference::LocalFirst {
            candidates.sort_by_key(|candidate| candidate.kind != ProviderCandidateKind::Local);
        }
        let selected = candidates.into_iter().find(matches_required);
        ProviderRouteDecision { selected }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderRouteDecision {
    selected: Option<ProviderCandidate>,
}

impl ProviderRouteDecision {
    #[must_use]
    pub fn selected_id(&self) -> Option<&str> {
        self.selected
            .as_ref()
            .map(|candidate| candidate.id.as_str())
    }

    #[must_use]
    pub fn requires_provider_egress_approval(&self) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|candidate| candidate.kind == ProviderCandidateKind::Cloud)
    }

    #[must_use]
    pub fn cost_hint(&self) -> Option<String> {
        self.selected
            .as_ref()
            .and_then(|candidate| candidate.cost_hint.as_ref())
            .map(|hint| format!("metadata:{hint}"))
    }
}
