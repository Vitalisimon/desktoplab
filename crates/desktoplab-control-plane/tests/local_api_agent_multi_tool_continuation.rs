use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn approval_continuation_executes_additional_read_before_final_response() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("README.md"),
        "# Workspace\nmodule: src/lib.rs\n",
    )
    .expect("read fixture should exist");
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo seed.md.","desktoplabAction":{"kind":"create_file","path":"seed.md","content":"seed\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea seed.md e poi leggi README.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        "README.md identifies src/lib.rs as a workspace module.",
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert_timeline_contains(&resumed["session"], "Read README.md:");
    assert_timeline_contains(&resumed["session"], "identifies src/lib.rs");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("seed.md")).unwrap(),
        "seed\n"
    );
}

#[test]
fn approval_continuation_pauses_again_for_an_additional_write() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"desktoplabAction":{"kind":"create_file","path":"first.md","content":"first\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea due file");
    let first_approval = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.write_file","arguments":{"path":"second.md","content":"second\n"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{first_approval}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "blocked");
    assert_eq!(
        resumed["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(workspace_root.join("first.md").exists());
    assert!(!workspace_root.join("second.md").exists());

    let second_approval = resumed["session"]["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"Both files were created."}}"#,
    );
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{second_approval}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("second.md")).unwrap(),
        "second\n"
    );
}

#[test]
fn approval_continuation_deduplicates_the_same_applied_write() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let action =
        r##"{"name":"desktoplab.write_file","arguments":{"path":"same.md","content":"same\n"}}"##;
    router.complete_agent_backend_for_test(action);
    let blocked = create_session(&mut router, "crea same.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        action,
        "same.md was created and verified from executor evidence.",
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert!(
        resumed["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("same.md")).unwrap(),
        "same\n"
    );
}

#[test]
fn unchanged_write_is_replanned_instead_of_reported_as_applied() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("same.md"), "current\n").unwrap();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"same.md","content":"current\n"}}"##,
    );
    let blocked = create_session(&mut router, "append a verification line to same.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r##"{"name":"desktoplab.write_file","arguments":{"path":"same.md","content":"current\nverified\n"}}"##,
    ]);

    resolve_approval(&mut router, approval_id);
    let replanned = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(replanned["session"]["state"], "blocked", "{replanned}");
    assert_timeline_contains(&replanned["session"], "write_no_change");
    assert_eq!(
        replanned["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("same.md")).unwrap(),
        "current\n"
    );
}

#[test]
fn blocked_initial_action_does_not_publish_completed_before_approval() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"pending.md","content":"pending\n"}}"##,
    );

    let blocked = create_session(&mut router, "create pending.md");

    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert!(
        blocked["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .all(|event| event["kind"] != "completed"),
        "{blocked}"
    );
}

#[test]
fn denied_filesystem_approval_is_saved_and_does_not_write() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"denied.md","content":"blocked\n"}}"##,
    );
    let blocked = create_session(&mut router, "create denied.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"deny"}"#,
    );
    let denied = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(denied["session"]["state"], "blocked", "{denied}");
    assert!(
        denied["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(!workspace_root.join("denied.md").exists());
    assert_timeline_contains(&denied["session"], "approval denied");
}

#[test]
fn approved_file_target_is_not_rewritten_by_a_different_followup_payload() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let first = r##"{"name":"desktoplab.write_file","arguments":{"path":"same.md","content":"correct\n"}}"##;
    router.complete_agent_backend_for_test(first);
    let blocked = create_session(&mut router, "write same.md once");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r##"{"name":"desktoplab.write_file","arguments":{"path":"same.md","content":"worse formatting"}}"##,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert!(
        resumed["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_timeline_contains(
        &resumed["session"],
        "provider_output_recovery:completed_target_not_rewritten",
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("same.md")).unwrap(),
        "correct\n"
    );
}

#[test]
fn approval_continuation_completes_after_a_repeated_read_only_action() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Repeated read proof\n").unwrap();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"seed.md","content":"seed\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea seed.md e verifica README.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    let read = r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#;
    router.complete_agent_backend_sequence_for_test([read, read]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert_timeline_contains(&resumed["session"], "Read README.md:");
    assert_timeline_contains(
        &resumed["session"],
        "provider_output_recovery:repeated_read_only_action",
    );
}

#[test]
fn repeated_read_only_action_cannot_hide_a_failed_validation() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("calculator.js"),
        "export function add(left, right) { return left - right; }\n",
    )
    .unwrap();
    router.complete_agent_backend_for_test(
        &serde_json::json!({
            "name": "desktoplab.run_tests",
            "arguments": { "command": calculator_validation_command() }
        })
        .to_string(),
    );
    let blocked = create_session(&mut router, "repair the failing calculator");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    let search = r#"{"name":"desktoplab.search_text","arguments":{"query":"left + right","path":"calculator.js"}}"#;
    router.complete_agent_backend_sequence_for_test([search, search, search]);

    resolve_approval(&mut router, approval_id);
    let failed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(failed["session"]["state"], "failed", "{failed}");
    assert_timeline_contains(
        &failed["session"],
        "agent_no_progress_repeated_read_only_action",
    );
}

#[test]
fn completed_read_is_not_blocked_by_redundant_read_clarification() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Grounded read proof\n").unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        r#"{"name":"desktoplab.clarify","arguments":{"question":"What next?","blockedOn":"desktoplab.read_file"}}"#,
        "README.md contains the heading Grounded read proof.",
    ]);

    let completed = create_session(&mut router, "inspect README.md and explain it");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "Read README.md:");
    assert_timeline_contains(
        &completed,
        "provider_output_recovery:observed_read_not_blocking",
    );
    assert_timeline_contains(&completed, "contains the heading Grounded read proof");
}

#[test]
fn structured_completion_finishes_after_read_observation() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Completion proof\n").unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        r#"{"name":"desktoplab.complete","arguments":{"message":"README.md contains the Completion proof heading."}}"#,
    ]);

    let completed = create_session(&mut router, "read README.md and explain it");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "Read README.md:");
    assert_timeline_contains(&completed, "contains the Completion proof heading");
}

#[test]
fn equivalent_root_lists_get_a_final_grounded_completion_chance() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.list_files","arguments":{"path":"/"}}"#,
        r#"{"name":"desktoplab.list_files","arguments":{}}"#,
        r#"{"name":"desktoplab.list_files","arguments":{}}"#,
        r#"{"name":"desktoplab.complete","arguments":{"message":"missing.md is not present in the workspace."}}"#,
    ]);

    let completed = create_session(
        &mut router,
        "read missing.md and report truthfully if it does not exist",
    );

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "provider_output_recovery:read_only_no_progress");
    assert_timeline_contains(
        &completed,
        "provider_output_recovery:final_read_only_synthesis",
    );
    assert_timeline_contains(&completed, "missing.md is not present in the workspace");
}

#[test]
fn repeated_read_only_action_before_any_mutation_cannot_claim_completion() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let read = r#"{"name":"desktoplab.read_file","arguments":{"path":"target.md"}}"#;
    router.complete_agent_backend_sequence_for_test([read, read, read, read]);

    let failed = create_session(&mut router, "update target.md");

    assert_eq!(failed["state"], "failed");
    assert_timeline_contains(&failed, "agent_no_progress_repeated_read_only_action");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: old\n"
    );
}

#[test]
fn repeated_read_only_action_gets_one_chance_to_select_a_mutation() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let read = r#"{"name":"desktoplab.read_file","arguments":{"path":"target.md"}}"#;
    let write = r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old","replacement":"status: new"}}"#;
    router.complete_agent_backend_sequence_for_test([read, read, read, write]);

    let blocked = create_session(&mut router, "update target.md");

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["pendingApprovals"].as_array().unwrap().len(), 1);
    assert_timeline_contains(&blocked, "provider_output_recovery:read_only_no_progress");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: old\n"
    );
}

#[test]
fn optional_clarification_after_read_only_evidence_recovers_to_mutation() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let read = r#"{"name":"desktoplab.read_file","arguments":{"path":"target.md"}}"#;
    let optional =
        r#"{"name":"desktoplab.clarify","arguments":{"question":"Anything else?","blockedOn":""}}"#;
    let write = r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old","replacement":"status: new"}}"#;
    router.complete_agent_backend_sequence_for_test([read, optional, write]);

    let blocked = create_session(&mut router, "update target.md");

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["pendingApprovals"].as_array().unwrap().len(), 1);
    assert_timeline_contains(&blocked, "provider_output_recovery:read_only_no_progress");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: old\n"
    );
}

#[test]
fn approval_gated_clarification_after_read_recovers_to_native_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let read = r#"{"name":"desktoplab.read_file","arguments":{"path":"target.md"}}"#;
    let confirmation = r#"{"name":"desktoplab.clarify","arguments":{"question":"Should I update it?","blockedOn":"desktoplab.patch_file"}}"#;
    let write = r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old","replacement":"status: new"}}"#;
    router.complete_agent_backend_sequence_for_test([read, confirmation, write]);

    let blocked = create_session(&mut router, "update target.md");

    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked["pendingApprovals"].as_array().unwrap().len(), 1);
    assert_timeline_contains(
        &blocked,
        "provider_output_recovery:approval_is_confirmation",
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: old\n"
    );
}

#[test]
fn repeated_canonical_clarification_after_read_remains_blocking() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let read = r#"{"name":"desktoplab.read_file","arguments":{"path":"target.md"}}"#;
    let clarification = r#"{"name":"desktoplab.clarify","arguments":{"question":"What replacement value should I use?","blockedOn":"desktoplab.patch_file"}}"#;
    router.complete_agent_backend_sequence_for_test([read, clarification, clarification]);

    let blocked = create_session(
        &mut router,
        "update target.md with the value I choose later",
    );

    assert_eq!(blocked["state"], "blocked");
    assert_timeline_contains(
        &blocked,
        "clarification_required:What replacement value should I use?",
    );
    assert!(blocked["pendingApprovals"].as_array().unwrap().is_empty());
}

#[test]
fn optional_post_action_clarification_without_blocked_action_completes() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"done.md","content":"done\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea done.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.clarify","arguments":{"question":"Anything else?"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert_timeline_contains(
        &resumed["session"],
        "provider_output_recovery:optional_post_action_clarification",
    );
    assert!(workspace_root.join("done.md").exists());
}

#[test]
fn post_action_clarification_with_blocked_action_remains_blocking() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"first.md","content":"first\n"}}"##,
    );
    let blocked = create_session(
        &mut router,
        "crea first.md e poi esegui il test scelto dall'utente",
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    let clarification = r#"{"name":"desktoplab.clarify","arguments":{"question":"What test command should I run?","blockedOn":"desktoplab.run_tests"}}"#;
    router.complete_agent_backend_sequence_for_test([clarification]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "blocked");
    assert_timeline_contains(
        &resumed["session"],
        "clarification_required:What test command should I run?",
    );
}

#[test]
fn arbitrary_post_action_blocked_on_value_cannot_block_completion() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"bounded.md","content":"done\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea bounded.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.clarify","arguments":{"question":"Anything else?","blockedOn":"bounded.md"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert!(workspace_root.join("bounded.md").exists());
}

#[test]
fn malformed_post_action_output_gets_one_bounded_recovery_attempt() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"recovered.md","content":"done\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea recovered.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":}}"#,
        r#"{"name":"desktoplab.clarify","arguments":{"question":"Anything else?"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert!(workspace_root.join("recovered.md").exists());
}

#[test]
fn repeated_malformed_post_action_output_still_fails_closed() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"failed.md","content":"done\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea failed.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    let malformed = r#"{"name":"desktoplab.read_file","arguments":{"path":}}"#;
    router.complete_agent_backend_sequence_for_test([malformed, malformed]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "failed");
    assert_timeline_contains(&resumed["session"], "malformed structured file action");
}

#[test]
fn verified_mutation_completes_from_executor_evidence_when_final_synthesis_stays_malformed() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("verified.md"), "before\n").unwrap();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.patch_file","arguments":{"path":"verified.md","expected":"before","replacement":"after"}}"##,
    );
    let blocked = create_session(
        &mut router,
        "update verified.md, read it, and report the result",
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"verified.md"}}"#,
        r#"{"name":"desktoplab.read_file","arguments":{"path":}}"#,
        r#"{"name":"desktoplab.read_file","arguments":{"path":}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed", "{resumed}");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("verified.md")).unwrap(),
        "after\n"
    );
    assert_timeline_contains(
        &resumed["session"],
        "provider_output_recovery:executor_grounded_completion",
    );
    assert_timeline_contains(&resumed["session"], "Read verified.md:\nafter");
}

#[test]
fn malformed_output_after_read_gets_one_bounded_synthesis_retry() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Read recovery proof\n").unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        r#"{"name":"desktoplab.read_file","arguments":{"path":}}"#,
        "README.md contains the heading Read recovery proof.",
    ]);

    let completed = create_session(&mut router, "inspect README.md and explain it");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "Read README.md:");
    assert_timeline_contains(&completed, "contains the heading Read recovery proof");
}

#[test]
fn empty_output_after_read_gets_one_bounded_synthesis_retry() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Empty recovery proof\n").unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        "",
        r#"{"name":"desktoplab.complete","arguments":{"message":"README.md contains the heading Empty recovery proof."}}"#,
    ]);

    let completed = create_session(&mut router, "inspect README.md and explain it");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_timeline_contains(&completed, "Read README.md:");
    assert_timeline_contains(&completed, "contains the heading Empty recovery proof");
}

#[test]
fn repeated_empty_output_after_read_fails_closed() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("README.md"), "# Fail closed proof\n").unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}"#,
        "",
        "",
    ]);

    let failed = create_session(&mut router, "inspect README.md and explain it");

    assert_eq!(failed["state"], "failed", "{failed}");
    assert_timeline_contains(&failed, "malformed structured file action");
    assert!(
        !failed["transcript"]
            .as_array()
            .unwrap()
            .iter()
            .any(|turn| { turn["role"] == "assistant" && turn["content"].as_str() == Some("") })
    );
}

#[test]
fn empty_required_tool_argument_recovers_without_creating_an_approval() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("calculator.test.js"),
        "assert.equal(add(2, 3), 5);\n",
    )
    .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.read_file","arguments":{"path":"calculator.test.js"}}"#,
        r#"{"name":"desktoplab.run_tests","arguments":{"command":""}}"#,
        r#"{"name":"desktoplab.complete","arguments":{"message":"The test expectation was inspected."}}"#,
    ]);

    let completed = create_session(&mut router, "inspect the failing test");

    assert_eq!(completed["state"], "completed", "{completed}");
    assert!(completed["pendingApprovals"].as_array().unwrap().is_empty());
    assert!(!completed.to_string().contains("test.run:"));
}

#[test]
fn failed_validation_requires_a_later_passing_rerun_before_completion() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("calculator.js"),
        "export function add(left, right) { return left - right; }\n",
    )
    .unwrap();
    router.complete_agent_backend_for_test(
        &serde_json::json!({
            "name": "desktoplab.run_tests",
            "arguments": {
                "command": calculator_validation_command(),
                "reason": "prove fail repair"
            }
        })
        .to_string(),
    );
    let first = create_session(&mut router, "repair the failing validation and rerun it");
    let test_approval = first["pendingApprovals"][0]["approvalId"].as_str().unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"calculator.js","expected":"left - right","replacement":"left + right"}}"#,
    ]);

    resolve_approval(&mut router, test_approval);
    let patch_pending = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    let patch_approval = patch_pending["session"]["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"calculator.js","expected":"left + right","replacement":"left + right"}}"#,
    ]);

    resolve_approval(&mut router, patch_approval);
    let rerun_pending = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    assert_eq!(
        rerun_pending["session"]["state"], "blocked",
        "{rerun_pending}"
    );
    assert_timeline_contains(
        &rerun_pending["session"],
        "provider_output_recovery:automatic_validation_rerun",
    );
    let rerun_approval = rerun_pending["session"]["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"The repair is verified by a passing rerun."}}"#,
    );

    resolve_approval(&mut router, rerun_approval);
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert!(
        std::fs::read_to_string(workspace_root.join("calculator.js"))
            .unwrap()
            .contains("left + right")
    );
    assert_timeline_contains(&completed["session"], "status Exited(1)");
    assert_timeline_contains(&completed["session"], "status Exited(0)");
}

#[test]
fn canonical_progress_refreshes_the_bounded_malformed_output_recovery() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(
        workspace_root.join("calculator.js"),
        "export function add(left, right) { return left - right; }\n",
    )
    .unwrap();
    router.complete_agent_backend_for_test(
        &serde_json::json!({
            "name": "desktoplab.run_tests",
            "arguments": { "command": calculator_validation_command() }
        })
        .to_string(),
    );
    let first = create_session(&mut router, "repair the failing calculator");
    let test_approval = first["pendingApprovals"][0]["approvalId"].as_str().unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"legacy_read_file","arguments":{"path":"calculator.js"}}"#,
        r#"{"name":"desktoplab.read_file","arguments":{"path":"calculator.js"}}"#,
        r#"{"name":"legacy_patch_file","arguments":{"path":"calculator.js","expected":"left - right","replacement":"left + right"}}"#,
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"calculator.js","expected":"left - right","replacement":"left + right"}}"#,
    ]);

    resolve_approval(&mut router, test_approval);
    let recovered = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(recovered["session"]["state"], "blocked", "{recovered}");
    assert_eq!(
        recovered["session"]["pendingApprovals"][0]["action"],
        "filesystem.write"
    );
    assert_timeline_contains(&recovered["session"], "Read calculator.js:");
    assert!(
        !recovered["session"]
            .to_string()
            .contains("malformed structured file action")
    );
}

#[test]
fn patch_conflict_returns_to_the_model_for_read_and_replan() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("source.js"), "return left - right;\n").unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"source.js","expected":"return a - b;","replacement":"return a + b;"}}"#,
    );
    let first = create_session(&mut router, "repair source.js");
    let first_approval = first["pendingApprovals"][0]["approvalId"].as_str().unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"source.js","expected":"left - right","replacement":"left + right"}}"#,
    ]);

    resolve_approval(&mut router, first_approval);
    let replanned = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(replanned["session"]["state"], "blocked", "{replanned}");
    assert_timeline_contains(&replanned["session"], "patch_conflict");
    assert_timeline_contains(&replanned["session"], "Read source.js:");
    let second_approval = replanned["session"]["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"The conflict was resolved from current source evidence."}}"#,
    );

    resolve_approval(&mut router, second_approval);
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("source.js")).unwrap(),
        "return left + right;\n"
    );
}

#[test]
fn repeated_failed_patch_is_not_offered_for_approval_again() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("source.js"), "return left - right;\n").unwrap();
    let stale_patch = r#"{"name":"desktoplab.patch_file","arguments":{"path":"source.js","expected":"return a - b;","replacement":"return a + b;"}}"#;
    router.complete_agent_backend_for_test(stale_patch);
    let first = create_session(&mut router, "repair source.js");
    let first_approval = first["pendingApprovals"][0]["approvalId"].as_str().unwrap();
    router.complete_agent_backend_sequence_for_test([
        stale_patch,
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"source.js","expected":"left - right","replacement":"left + right"}}"#,
    ]);

    resolve_approval(&mut router, first_approval);
    let replanned = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(replanned["session"]["state"], "blocked", "{replanned}");
    assert_eq!(
        replanned["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "{replanned}"
    );
    assert_timeline_contains(
        &replanned["session"],
        "provider_output_recovery:deduplicated_failed_action",
    );
}

#[test]
fn completed_commit_is_not_offered_for_approval_again() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    run_git(
        &workspace_root,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace_root, &["config", "user.name", "DesktopLab Test"]);
    std::fs::write(workspace_root.join("README.md"), "# Initial\n").unwrap();
    run_git(&workspace_root, &["add", "."]);
    run_git(&workspace_root, &["commit", "-m", "initial"]);
    std::fs::write(workspace_root.join("README.md"), "# Changed\n").unwrap();
    let commit =
        r#"{"name":"desktoplab.commit_changes","arguments":{"message":"docs: update readme"}}"#;
    router.complete_agent_backend_for_test(commit);
    let blocked = create_session(&mut router, "commit the current change");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([commit]);

    resolve_approval(&mut router, approval_id);
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert!(
        completed["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{completed}"
    );
    assert_timeline_contains(
        &completed["session"],
        "provider_output_recovery:completed_git_transition_not_repeated",
    );
    let commit_count = std::process::Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(&workspace_root)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&commit_count.stdout).trim(), "2");
}

#[test]
fn completed_commit_is_not_revived_by_the_next_user_turn() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    run_git(
        &workspace_root,
        &["config", "user.email", "desktoplab@example.test"],
    );
    run_git(&workspace_root, &["config", "user.name", "DesktopLab Test"]);
    std::fs::write(workspace_root.join("README.md"), "# Initial\n").unwrap();
    run_git(&workspace_root, &["add", "."]);
    run_git(&workspace_root, &["commit", "-m", "initial"]);
    std::fs::write(workspace_root.join("README.md"), "# Changed\n").unwrap();
    let commit =
        r#"{"name":"desktoplab.commit_changes","arguments":{"message":"docs: update readme"}}"#;
    router.complete_agent_backend_for_test(commit);
    let blocked = create_session(&mut router, "commit the current change");
    let session_id = blocked["sessionId"].as_str().unwrap();
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([commit]);
    resolve_approval(&mut router, approval_id);

    router.complete_agent_backend_for_test(commit);
    let next_turn = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"read missing.md"}"#,
    );

    assert_eq!(next_turn["state"], "completed", "{next_turn}");
    assert!(next_turn["pendingApprovals"].as_array().unwrap().is_empty());
    assert_timeline_contains(
        &next_turn,
        "state=skipped source=agent.approval canonical=desktoplab.commit_changes",
    );
}

#[test]
fn completed_patch_is_not_revived_by_the_next_user_turn() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let patch = r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old","replacement":"status: new"}}"#;
    router.complete_agent_backend_for_test(patch);
    let blocked = create_session(&mut router, "update target.md");
    let session_id = blocked["sessionId"].as_str().unwrap();
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"target.md was updated."}}"#,
    );
    resolve_approval(&mut router, approval_id);

    router.complete_agent_backend_sequence_for_test([
        patch,
        r#"{"name":"desktoplab.complete","arguments":{"message":"The next task continued without repeating the completed patch."}}"#,
    ]);
    let next_turn = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"run the next repository task"}"#,
    );

    assert_eq!(next_turn["state"], "completed", "{next_turn}");
    assert!(next_turn["pendingApprovals"].as_array().unwrap().is_empty());
    assert_timeline_contains(
        &next_turn,
        "provider_output_recovery:deduplicated_applied_action",
    );
    assert!(
        !next_turn.to_string().contains("patch_conflict"),
        "{next_turn}"
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: new\n"
    );
}

#[test]
fn semantically_completed_patch_is_not_revived_by_the_next_user_turn() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    std::fs::write(workspace_root.join("target.md"), "status: old\n").unwrap();
    let patch = r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old","replacement":"status: new"}}"#;
    router.complete_agent_backend_for_test(patch);
    let blocked = create_session(&mut router, "update target.md");
    let session_id = blocked["sessionId"].as_str().unwrap();
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"target.md was updated."}}"#,
    );
    resolve_approval(&mut router, approval_id);

    let equivalent_whole_file_patch = r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old\n","replacement":"status: new\n"}}"#;
    router.complete_agent_backend_sequence_for_test([
        equivalent_whole_file_patch,
        r#"{"name":"desktoplab.run_tests","arguments":{"command":"true","reason":"Verify the current task"}}"#,
    ]);
    let next_turn = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"run the next repository task"}"#,
    );

    assert_eq!(next_turn["state"], "blocked", "{next_turn}");
    assert_eq!(next_turn["pendingApprovals"].as_array().unwrap().len(), 1);
    assert_eq!(
        next_turn["pendingApprovals"][0]["operationId"],
        "test.run:true"
    );
    assert_timeline_contains(
        &next_turn,
        "provider_output_recovery:deduplicated_applied_action",
    );
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: new\n"
    );
}

#[test]
fn historical_write_dedup_does_not_block_a_new_turn_patch_to_the_same_file() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    let original_write = r##"{"name":"desktoplab.write_file","arguments":{"path":"target.md","content":"status: old\n"}}"##;
    router.complete_agent_backend_for_test(original_write);
    let blocked = create_session(&mut router, "create target.md");
    let session_id = blocked["sessionId"].as_str().unwrap();
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"target.md was created."}}"#,
    );
    resolve_approval(&mut router, approval_id);

    router.complete_agent_backend_sequence_for_test([
        original_write,
        r#"{"name":"desktoplab.read_file","arguments":{"path":"target.md"}}"#,
        r#"{"name":"desktoplab.patch_file","arguments":{"path":"target.md","expected":"status: old","replacement":"status: new"}}"#,
    ]);
    let next_turn = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"update target.md to status new"}"#,
    );

    assert_eq!(next_turn["state"], "blocked", "{next_turn}");
    assert_eq!(next_turn["pendingApprovals"].as_array().unwrap().len(), 1);
    assert_timeline_contains(
        &next_turn,
        "provider_output_recovery:deduplicated_applied_action",
    );
    assert_timeline_contains(&next_turn, "Read target.md:");
    assert!(
        !next_turn
            .to_string()
            .contains("provider_output_recovery:completed_target_not_rewritten"),
        "{next_turn}"
    );

    let patch_approval = next_turn["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_for_test(
        r#"{"name":"desktoplab.complete","arguments":{"message":"target.md was updated."}}"#,
    );
    resolve_approval(&mut router, patch_approval);

    assert_eq!(
        std::fs::read_to_string(workspace_root.join("target.md")).unwrap(),
        "status: new\n"
    );
}

#[test]
fn unsolicited_git_followup_after_mutation_does_not_keep_task_open() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"done.md","content":"done\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea done.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.clarify","arguments":{"question":"Should I commit?","blockedOn":"desktoplab.commit_changes"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert!(workspace_root.join("done.md").exists());
    assert!(
        resumed["session"]["pendingApprovals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn same_mutation_family_followup_does_not_keep_task_open() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"done.md","content":"done\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea done.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"desktoplab.clarify","arguments":{"question":"More edits?","blockedOn":"desktoplab.patch_file"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "completed");
    assert!(workspace_root.join("done.md").exists());
}

#[test]
fn approval_continuation_rejects_unknown_tool_aliases() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"name":"desktoplab.write_file","arguments":{"path":"first.md","content":"first\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea first.md e second.md");
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    router.complete_agent_backend_sequence_for_test([
        r#"{"name":"legacy_write_file","arguments":{"path":"second.md","content":"second\n"}}"#,
    ]);

    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    let resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(resumed["session"]["state"], "failed");
    assert!(workspace_root.join("first.md").exists());
    assert!(!workspace_root.join("second.md").exists());
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    std::fs::create_dir_all(&workspace_root).expect("workspace should exist");
    run_git(&workspace_root, &["init", "-b", "main"]);
    let mut router = LocalApiRouter::default();
    router.enable_test_controls_for_dev_server();
    router.set_host_memory_gb_for_test(32);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
    (fixture, workspace_root, router)
}

fn create_session(router: &mut LocalApiRouter, prompt: &str) -> Value {
    route_json(
        router,
        "POST",
        "/v1/sessions",
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","initialPrompt":{}}}"#,
            serde_json::to_string(prompt).unwrap()
        ),
    )
}

#[cfg(not(windows))]
fn calculator_validation_command() -> &'static str {
    "grep -q 'left + right' calculator.js"
}

#[cfg(windows)]
fn calculator_validation_command() -> &'static str {
    "if ((Get-Content -Raw -LiteralPath 'calculator.js') -match 'left \\+ right') { exit 0 } else { exit 1 }"
}

fn assert_timeline_contains(session: &Value, expected: &str) {
    assert!(
        session["timeline"].as_array().unwrap().iter().any(|event| {
            event["message"]
                .as_str()
                .is_some_and(|message| message.contains(expected))
        }),
        "timeline did not contain {expected:?}: {session}"
    );
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn resolve_approval(router: &mut LocalApiRouter, approval_id: &str) {
    route_json(
        router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
}
