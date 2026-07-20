use desktoplab_agent_engine::{AgentParallelDecision, AgentParallelIntent, AgentParallelPolicy};
use xtask::check_logical_line_limit;

#[test]
fn write_capable_parallel_agents_require_worktree_isolation() {
    let policy = AgentParallelPolicy::strict();

    assert_eq!(
        policy.decide(AgentParallelIntent::WriteCapable),
        AgentParallelDecision::RequiresWorktree
    );
    assert!(!policy.ui_allows_shared_parallel_mode(AgentParallelIntent::WriteCapable));
}

#[test]
fn read_only_parallel_agents_can_share_workspace() {
    let policy = AgentParallelPolicy::strict();

    assert_eq!(
        policy.decide(AgentParallelIntent::ReadOnly),
        AgentParallelDecision::CanShareWorkspace
    );
    assert!(policy.ui_allows_shared_parallel_mode(AgentParallelIntent::ReadOnly));
}

#[test]
fn worktree_write_policy_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/parallel_policy.rs",
        include_str!("../src/parallel_policy.rs"),
        90,
    )
    .expect("parallel agent policy should stay focused");
}
