use desktoplab_agent_engine::{
    AgentContextPlanner, AgentRouteContextCapabilities, ContextCandidate, ContextInclusionReason,
    ContextSectionKind, ContextStrategy,
};
use xtask::check_logical_line_limit;

#[test]
fn powerful_rag_route_uses_mixed_context_with_explicit_section_budgets() {
    let capabilities = AgentRouteContextCapabilities::new(32_000, 32_000)
        .with_repo_rag(true)
        .with_long_context(true);
    let plan = AgentContextPlanner::plan(
        &capabilities,
        vec![
            candidate(ContextSectionKind::SystemPolicy, "policy", "policy"),
            candidate(ContextSectionKind::UserGoal, "refactor checkout", "user"),
            candidate(ContextSectionKind::ToolSchemas, "filesystem patch", "tools"),
            candidate(
                ContextSectionKind::RetrievedEvidence,
                "related code",
                "rag:1",
            ),
            candidate(
                ContextSectionKind::PriorTranscript,
                "previous work",
                "session",
            ),
            candidate(
                ContextSectionKind::WorkspaceMemory,
                "remembered decision",
                "memory:1",
            ),
        ],
    );

    assert_eq!(plan.strategy(), ContextStrategy::Mixed);
    assert!(plan.budget().used_bytes() <= plan.budget().max_bytes());
    assert!(plan.budget().used_tokens() <= plan.budget().max_tokens());
    assert!(
        plan.budget()
            .section_bytes(ContextSectionKind::RetrievedEvidence)
            > 0
    );
    assert!(
        plan.budget()
            .section_bytes(ContextSectionKind::WorkspaceMemory)
            > 0
    );
}

#[test]
fn exact_mutation_target_precedes_and_survives_retrieved_background() {
    let capabilities = AgentRouteContextCapabilities::new(4_096, 4_096).with_repo_rag(true);
    let plan = AgentContextPlanner::plan(
        &capabilities,
        vec![
            ContextCandidate::exact_file("src/target.rs", "pub fn target() {}", true),
            ContextCandidate::exact_file("src/read.rs", "pub fn read() {}", false),
            candidate(
                ContextSectionKind::RetrievedEvidence,
                &"background ".repeat(1_000),
                "rag:large",
            ),
        ],
    );

    assert_eq!(plan.items()[0].provenance(), "src/target.rs");
    assert_eq!(
        plan.items()[0].reason(),
        ContextInclusionReason::ExactMutationTarget
    );
    assert!(plan.items()[0].text().contains("pub fn target"));
}

#[test]
fn small_route_uses_summary_and_excludes_non_target_full_files_and_rag() {
    let capabilities = AgentRouteContextCapabilities::new(1_000, 2_000);
    let plan = AgentContextPlanner::plan(
        &capabilities,
        vec![
            ContextCandidate::exact_file("src/background.rs", "large background", false),
            ContextCandidate::exact_file("src/target.rs", "mutation", true),
            candidate(ContextSectionKind::RetrievedEvidence, "retrieved", "rag"),
            candidate(ContextSectionKind::RepositorySummary, "summary", "repo"),
        ],
    );

    assert_eq!(plan.strategy(), ContextStrategy::Summarized);
    assert!(
        plan.items()
            .iter()
            .any(|item| item.provenance() == "src/target.rs")
    );
    assert!(plan.items().iter().any(|item| item.provenance() == "repo"));
    assert!(plan.items().iter().all(|item| item.provenance() != "rag"));
    assert!(
        plan.items()
            .iter()
            .all(|item| item.provenance() != "src/background.rs")
    );
}

#[test]
fn context_planner_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/context_planner.rs",
        include_str!("../src/context_planner.rs"),
        380,
    )
    .expect("context planner should stay focused");
}

fn candidate(kind: ContextSectionKind, text: &str, provenance: &str) -> ContextCandidate {
    ContextCandidate::new(kind, text, provenance)
}
