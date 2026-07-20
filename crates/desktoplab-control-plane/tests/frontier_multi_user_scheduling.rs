use desktoplab_control_plane::{
    FrontierPartitionKind, FrontierResourcePartition, FrontierResourceScheduler,
    FrontierScheduleDecision, FrontierScheduleRequest,
};
use desktoplab_policy::{FrontierDeploymentMode, WorkspaceAccess};
use xtask::check_logical_line_limit;

#[test]
fn scheduler_assigns_distinct_mig_slices_to_distinct_user_sessions() {
    let mut scheduler = scheduler(FrontierDeploymentMode::SharedWorkstation);
    let first = scheduler.schedule(request("user.a", "session.a", "workspace.a", "user.a"));
    let second = scheduler.schedule(request("user.b", "session.b", "workspace.b", "user.b"));

    let FrontierScheduleDecision::Assigned(first) = first else {
        panic!("first session should be assigned")
    };
    let FrontierScheduleDecision::Assigned(second) = second else {
        panic!("second session should be assigned")
    };
    assert_ne!(first.partition_id(), second.partition_id());
    assert_eq!(first.user_id(), "user.a");
    assert_eq!(second.user_id(), "user.b");
}

#[test]
fn busy_compatible_partitions_queue_without_stealing_a_lease() {
    let mut scheduler = scheduler(FrontierDeploymentMode::SharedWorkstation);
    scheduler.schedule(request("user.a", "session.a", "workspace.a", "user.a"));
    scheduler.schedule(request("user.b", "session.b", "workspace.b", "user.b"));
    let third = scheduler.schedule(request("user.c", "session.c", "workspace.c", "user.c"));

    assert_eq!(
        third,
        FrontierScheduleDecision::Queued {
            session_id: "session.c".into()
        }
    );
    assert_eq!(scheduler.queued_sessions(), &["session.c"]);
    assert!(!scheduler.release("user.c", "session.a"));
    assert!(scheduler.release("user.a", "session.a"));
}

#[test]
fn scheduler_enforces_workspace_cache_and_approval_ownership() {
    let mut scheduler = scheduler(FrontierDeploymentMode::SharedWorkstation);
    let foreign_workspace =
        scheduler.schedule(request("user.b", "session.b", "workspace.a", "user.a"));
    assert_eq!(
        foreign_workspace,
        FrontierScheduleDecision::Rejected {
            reason: "workspace_or_session_isolation_denied"
        }
    );
    assert!(scheduler.can_reuse_model_cache("user.b", "user.b", "user.a", true, false));
    assert!(!scheduler.can_reuse_model_cache("user.b", "user.b", "user.a", false, false));
    assert!(!scheduler.can_resolve_approval("user.admin", "user.b"));
    assert!(scheduler.can_resolve_approval("user.b", "user.b"));
}

#[test]
fn team_node_accepts_explicit_member_workspace_grant() {
    let mut scheduler = scheduler(FrontierDeploymentMode::TeamNode);
    let mut granted = request("user.b", "session.b", "workspace.a", "user.a");
    granted.workspace_access = WorkspaceAccess::GrantedTeamMember;
    assert!(matches!(
        scheduler.schedule(granted),
        FrontierScheduleDecision::Assigned(_)
    ));
}

#[test]
fn scheduler_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/frontier_scheduler.rs",
        include_str!("../src/frontier_scheduler.rs"),
        260,
    )
    .expect("frontier scheduler should remain focused");
}

fn scheduler(mode: FrontierDeploymentMode) -> FrontierResourceScheduler {
    FrontierResourceScheduler::new(
        mode,
        vec![
            mig_partition("mig.0", "MIG-GPU-0/1/0"),
            mig_partition("mig.1", "MIG-GPU-0/2/0"),
        ],
    )
}

fn mig_partition(id: &str, gpu_uuid: &str) -> FrontierResourcePartition {
    FrontierResourcePartition::new(
        id,
        FrontierPartitionKind::MigSlice {
            gpu_uuid: gpu_uuid.into(),
            profile: "7g.80gb".into(),
        },
        80,
        &["runtime.nim"],
    )
}

fn request(
    actor: &str,
    session: &str,
    workspace: &str,
    workspace_owner: &str,
) -> FrontierScheduleRequest {
    FrontierScheduleRequest {
        actor_user_id: actor.into(),
        session_id: session.into(),
        session_owner_user_id: actor.into(),
        workspace_id: workspace.into(),
        workspace_owner_user_id: workspace_owner.into(),
        workspace_access: WorkspaceAccess::Owner,
        model_id: "model.frontier".into(),
        runtime_id: "runtime.nim".into(),
        required_memory_gb: 70,
    }
}
