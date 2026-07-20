use desktoplab_agent_engine::{
    AgentToolExecutionOwner, AgentToolRisk, AgentToolScope, DesktopLabToolRegistry,
};

#[test]
fn registry_exposes_stable_desktoplab_tool_ids() {
    let registry = DesktopLabToolRegistry::default();
    let ids: Vec<&str> = registry.tools().iter().map(|tool| tool.id()).collect();

    assert_eq!(
        ids,
        vec![
            "desktoplab.list_files",
            "desktoplab.read_file",
            "desktoplab.search_text",
            "desktoplab.write_file",
            "desktoplab.patch_file",
            "desktoplab.create_directory",
            "desktoplab.move_path",
            "desktoplab.delete_path",
            "desktoplab.run_terminal",
            "desktoplab.start_process",
            "desktoplab.poll_process",
            "desktoplab.write_process_stdin",
            "desktoplab.kill_process",
            "desktoplab.git_status",
            "desktoplab.git_diff",
            "desktoplab.create_checkpoint",
            "desktoplab.run_tests",
            "desktoplab.commit_changes",
            "desktoplab.update_plan",
            "desktoplab.spawn_subagent",
            "desktoplab.send_subagent",
            "desktoplab.get_subagent",
            "desktoplab.cancel_subagent",
            "desktoplab.close_subagent",
            "desktoplab.complete",
            "desktoplab.clarify",
            "desktoplab.push_changes",
        ]
    );
}

#[test]
fn mutating_and_execution_tools_require_approval() {
    let registry = DesktopLabToolRegistry::default();

    for id in [
        "desktoplab.write_file",
        "desktoplab.patch_file",
        "desktoplab.create_directory",
        "desktoplab.move_path",
        "desktoplab.delete_path",
        "desktoplab.run_terminal",
        "desktoplab.start_process",
        "desktoplab.run_tests",
        "desktoplab.commit_changes",
        "desktoplab.push_changes",
    ] {
        let tool = registry.get(id).expect("tool should exist");
        assert!(tool.requires_approval(), "{id} must require approval");
        assert!(matches!(
            tool.risk(),
            AgentToolRisk::Medium | AgentToolRisk::High
        ));
    }

    for id in [
        "desktoplab.list_files",
        "desktoplab.read_file",
        "desktoplab.search_text",
        "desktoplab.git_status",
        "desktoplab.git_diff",
        "desktoplab.poll_process",
        "desktoplab.write_process_stdin",
        "desktoplab.kill_process",
        "desktoplab.create_checkpoint",
        "desktoplab.update_plan",
        "desktoplab.spawn_subagent",
        "desktoplab.send_subagent",
        "desktoplab.get_subagent",
        "desktoplab.cancel_subagent",
        "desktoplab.close_subagent",
        "desktoplab.clarify",
        "desktoplab.complete",
    ] {
        assert!(!registry.get(id).unwrap().requires_approval(), "{id}");
    }
}

#[test]
fn fallback_json_action_schema_uses_same_tool_vocabulary() {
    let registry = DesktopLabToolRegistry::default();
    let fallback = registry.strict_json_action_schema();
    let allowed = fallback["properties"]["tool"]["enum"].as_array().unwrap();

    for tool in registry.tools() {
        assert!(
            allowed
                .iter()
                .any(|value| value.as_str() == Some(tool.id())),
            "{} missing from fallback enum",
            tool.id()
        );
    }
}

#[test]
fn git_inspection_tools_explain_progress_without_repeating_observations() {
    let registry = DesktopLabToolRegistry::default();
    let status = registry.get("desktoplab.git_status").unwrap().description();
    let diff = registry.get("desktoplab.git_diff").unwrap().description();

    assert!(status.contains("changed and untracked paths"));
    assert!(status.contains("Do not repeat"));
    assert!(diff.contains("tracked file changes"));
    assert!(diff.contains("after status"));
}

#[test]
fn every_tool_declares_its_real_execution_owner_and_scope() {
    let registry = DesktopLabToolRegistry::default();
    let router_owned = [
        "desktoplab.update_plan",
        "desktoplab.spawn_subagent",
        "desktoplab.send_subagent",
        "desktoplab.get_subagent",
        "desktoplab.cancel_subagent",
        "desktoplab.close_subagent",
    ];

    for tool in registry.tools() {
        let expected_owner = if router_owned.contains(&tool.id()) {
            AgentToolExecutionOwner::RouterControl
        } else if matches!(tool.id(), "desktoplab.complete" | "desktoplab.clarify") {
            AgentToolExecutionOwner::LoopControl
        } else {
            AgentToolExecutionOwner::CanonicalGateway
        };
        assert_eq!(tool.execution_owner(), expected_owner, "{}", tool.id());
        let expected_scope = if matches!(
            tool.execution_owner(),
            AgentToolExecutionOwner::RouterControl | AgentToolExecutionOwner::LoopControl
        ) {
            AgentToolScope::Session
        } else {
            AgentToolScope::Workspace
        };
        assert_eq!(tool.scope(), expected_scope, "{}", tool.id());
    }
}
