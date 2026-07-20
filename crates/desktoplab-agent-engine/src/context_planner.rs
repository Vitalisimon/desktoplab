use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ContextSectionKind {
    SystemPolicy,
    UserGoal,
    ToolSchemas,
    ExactFile,
    RetrievedEvidence,
    RepositorySummary,
    WorkspaceMemory,
    PriorTranscript,
    ValidationOutput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextStrategy {
    Direct,
    Summarized,
    Retrieved,
    LongContextPacking,
    Mixed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextInclusionReason {
    SystemPolicy,
    UserGoal,
    ToolSchema,
    ExactMutationTarget,
    ExactRead,
    RetrievedEvidence,
    RepositorySummary,
    WorkspaceMemory,
    PriorTranscript,
    ValidationOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRouteContextCapabilities {
    max_context_tokens: usize,
    max_context_bytes: usize,
    repo_rag: bool,
    long_context: bool,
}

impl AgentRouteContextCapabilities {
    #[must_use]
    pub fn new(max_context_tokens: usize, max_context_bytes: usize) -> Self {
        Self {
            max_context_tokens,
            max_context_bytes,
            repo_rag: false,
            long_context: false,
        }
    }

    #[must_use]
    pub fn with_repo_rag(mut self, enabled: bool) -> Self {
        self.repo_rag = enabled;
        self
    }

    #[must_use]
    pub fn with_long_context(mut self, enabled: bool) -> Self {
        self.long_context = enabled;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextCandidate {
    kind: ContextSectionKind,
    text: String,
    provenance: String,
    mutation_target: bool,
}

impl ContextCandidate {
    #[must_use]
    pub fn new(
        kind: ContextSectionKind,
        text: impl Into<String>,
        provenance: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            text: text.into(),
            provenance: provenance.into(),
            mutation_target: false,
        }
    }

    #[must_use]
    pub fn exact_file(
        path: impl Into<String>,
        contents: impl Into<String>,
        mutation_target: bool,
    ) -> Self {
        let path = path.into();
        Self {
            kind: ContextSectionKind::ExactFile,
            text: format!("file:{path}\n{}", contents.into()),
            provenance: path,
            mutation_target,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextPlannedItem {
    kind: ContextSectionKind,
    text: String,
    provenance: String,
    reason: ContextInclusionReason,
    estimated_tokens: usize,
}

impl ContextPlannedItem {
    #[must_use]
    pub fn kind(&self) -> ContextSectionKind {
        self.kind
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn provenance(&self) -> &str {
        &self.provenance
    }

    #[must_use]
    pub fn reason(&self) -> ContextInclusionReason {
        self.reason
    }

    #[must_use]
    pub fn estimated_tokens(&self) -> usize {
        self.estimated_tokens
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBudgetReport {
    max_tokens: usize,
    max_bytes: usize,
    used_tokens: usize,
    used_bytes: usize,
    section_bytes: BTreeMap<ContextSectionKind, usize>,
}

impl ContextBudgetReport {
    #[must_use]
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    #[must_use]
    pub fn max_bytes(&self) -> usize {
        self.max_bytes
    }

    #[must_use]
    pub fn used_tokens(&self) -> usize {
        self.used_tokens
    }

    #[must_use]
    pub fn used_bytes(&self) -> usize {
        self.used_bytes
    }

    #[must_use]
    pub fn section_bytes(&self, kind: ContextSectionKind) -> usize {
        self.section_bytes.get(&kind).copied().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentContextPlan {
    strategy: ContextStrategy,
    items: Vec<ContextPlannedItem>,
    dropped_provenance: Vec<String>,
    budget: ContextBudgetReport,
}

impl AgentContextPlan {
    #[must_use]
    pub fn strategy(&self) -> ContextStrategy {
        self.strategy
    }

    #[must_use]
    pub fn items(&self) -> &[ContextPlannedItem] {
        &self.items
    }

    #[must_use]
    pub fn dropped_provenance(&self) -> &[String] {
        &self.dropped_provenance
    }

    #[must_use]
    pub fn budget(&self) -> &ContextBudgetReport {
        &self.budget
    }
}

pub struct AgentContextPlanner;

impl AgentContextPlanner {
    #[must_use]
    pub fn plan(
        capabilities: &AgentRouteContextCapabilities,
        mut candidates: Vec<ContextCandidate>,
    ) -> AgentContextPlan {
        let strategy = strategy(capabilities);
        candidates.retain(|candidate| eligible(strategy, candidate));
        candidates.sort_by(|left, right| {
            priority(left)
                .cmp(&priority(right))
                .then_with(|| left.provenance.cmp(&right.provenance))
        });
        let section_limits = section_limits(capabilities.max_context_bytes);
        let mut section_bytes = BTreeMap::new();
        let mut items = Vec::new();
        let mut dropped_provenance = Vec::new();
        let mut used_bytes = 0;
        let mut used_tokens = 0;

        for candidate in candidates {
            let section_used = section_bytes
                .get(&candidate.kind)
                .copied()
                .unwrap_or_default();
            let section_limit = section_limits
                .get(&candidate.kind)
                .copied()
                .unwrap_or_default();
            let total_remaining = capabilities.max_context_bytes.saturating_sub(used_bytes);
            let section_remaining = section_limit.saturating_sub(section_used);
            let token_remaining = capabilities.max_context_tokens.saturating_sub(used_tokens) * 4;
            let allowed = total_remaining.min(section_remaining).min(token_remaining);
            if allowed == 0 {
                dropped_provenance.push(candidate.provenance);
                continue;
            }
            let text = truncate_utf8(&candidate.text, allowed);
            if text.is_empty() {
                dropped_provenance.push(candidate.provenance);
                continue;
            }
            let bytes = text.len();
            let tokens = estimate_tokens(&text);
            used_bytes += bytes;
            used_tokens += tokens;
            section_bytes.insert(candidate.kind, section_used + bytes);
            let reason = inclusion_reason(&candidate);
            items.push(ContextPlannedItem {
                kind: candidate.kind,
                text,
                provenance: candidate.provenance,
                reason,
                estimated_tokens: tokens,
            });
        }

        AgentContextPlan {
            strategy,
            items,
            dropped_provenance,
            budget: ContextBudgetReport {
                max_tokens: capabilities.max_context_tokens,
                max_bytes: capabilities.max_context_bytes,
                used_tokens,
                used_bytes,
                section_bytes,
            },
        }
    }
}

fn strategy(capabilities: &AgentRouteContextCapabilities) -> ContextStrategy {
    match (capabilities.repo_rag, capabilities.long_context) {
        (true, true) => ContextStrategy::Mixed,
        (false, true) => ContextStrategy::LongContextPacking,
        (true, false) => ContextStrategy::Retrieved,
        (false, false) if capabilities.max_context_tokens >= 32_000 => ContextStrategy::Direct,
        (false, false) => ContextStrategy::Summarized,
    }
}

fn eligible(strategy: ContextStrategy, candidate: &ContextCandidate) -> bool {
    match strategy {
        ContextStrategy::Summarized => {
            candidate.kind != ContextSectionKind::RetrievedEvidence
                && (candidate.kind != ContextSectionKind::ExactFile || candidate.mutation_target)
        }
        ContextStrategy::Direct | ContextStrategy::LongContextPacking => {
            candidate.kind != ContextSectionKind::RetrievedEvidence
        }
        ContextStrategy::Retrieved | ContextStrategy::Mixed => true,
    }
}

fn priority(candidate: &ContextCandidate) -> u8 {
    match (candidate.kind, candidate.mutation_target) {
        (ContextSectionKind::SystemPolicy, _) => 0,
        (ContextSectionKind::UserGoal, _) => 1,
        (ContextSectionKind::ToolSchemas, _) => 2,
        (ContextSectionKind::ExactFile, true) => 3,
        (ContextSectionKind::ValidationOutput, _) => 4,
        (ContextSectionKind::ExactFile, false) => 5,
        (ContextSectionKind::RetrievedEvidence, _) => 6,
        (ContextSectionKind::RepositorySummary, _) => 7,
        (ContextSectionKind::WorkspaceMemory, _) => 8,
        (ContextSectionKind::PriorTranscript, _) => 9,
    }
}

fn inclusion_reason(candidate: &ContextCandidate) -> ContextInclusionReason {
    match (candidate.kind, candidate.mutation_target) {
        (ContextSectionKind::SystemPolicy, _) => ContextInclusionReason::SystemPolicy,
        (ContextSectionKind::UserGoal, _) => ContextInclusionReason::UserGoal,
        (ContextSectionKind::ToolSchemas, _) => ContextInclusionReason::ToolSchema,
        (ContextSectionKind::ExactFile, true) => ContextInclusionReason::ExactMutationTarget,
        (ContextSectionKind::ExactFile, false) => ContextInclusionReason::ExactRead,
        (ContextSectionKind::RetrievedEvidence, _) => ContextInclusionReason::RetrievedEvidence,
        (ContextSectionKind::RepositorySummary, _) => ContextInclusionReason::RepositorySummary,
        (ContextSectionKind::WorkspaceMemory, _) => ContextInclusionReason::WorkspaceMemory,
        (ContextSectionKind::PriorTranscript, _) => ContextInclusionReason::PriorTranscript,
        (ContextSectionKind::ValidationOutput, _) => ContextInclusionReason::ValidationOutput,
    }
}

fn section_limits(max_bytes: usize) -> BTreeMap<ContextSectionKind, usize> {
    [
        (ContextSectionKind::SystemPolicy, 8),
        (ContextSectionKind::UserGoal, 8),
        (ContextSectionKind::ToolSchemas, 8),
        (ContextSectionKind::ExactFile, 30),
        (ContextSectionKind::RetrievedEvidence, 18),
        (ContextSectionKind::RepositorySummary, 5),
        (ContextSectionKind::WorkspaceMemory, 10),
        (ContextSectionKind::PriorTranscript, 10),
        (ContextSectionKind::ValidationOutput, 3),
    ]
    .into_iter()
    .map(|(kind, percent)| (kind, max_bytes.saturating_mul(percent) / 100))
    .collect()
}

fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4).max(1)
}

fn truncate_utf8(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut boundary = max_bytes;
    while boundary > 0 && !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text[..boundary].to_string()
}
