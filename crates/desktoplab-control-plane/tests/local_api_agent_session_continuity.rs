use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn rapid_new_prompt_reuses_active_session_waiting_for_approval() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo note.md.","desktoplabAction":{"kind":"create_file","path":"note.md","content":"# Note\n"}}"##,
    );

    let blocked = create_session(&mut router, "crea note.md");
    let blocked_session_id = blocked["sessionId"].as_str().unwrap();
    assert_eq!(blocked["state"], "blocked");
    assert_eq!(blocked_session_id, "session.1");

    router.complete_agent_backend_for_test("This response must not start a new thread.");
    let rapid = create_session(&mut router, "leggi il file appena creato");

    assert_eq!(rapid["sessionId"], blocked_session_id);
    assert_eq!(rapid["state"], "blocked");
    assert_timeline_contains(&rapid, "session continuity pending user choice");
    assert_eq!(rapid["pendingApprovals"].as_array().unwrap().len(), 1);
    let sessions = route_json(&mut router, "GET", "/v1/sessions", "");
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 1);
}

#[test]
fn followup_prompt_without_approval_is_rejected_until_pending_action_resolves() {
    let (_fixture, workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo guide.md.","desktoplabAction":{"kind":"create_file","path":"guide.md","content":"# Guide\n"}}"##,
    );
    let blocked = create_session(&mut router, "crea guide.md");
    let session_id = blocked["sessionId"].as_str().unwrap().to_string();

    router.complete_agent_backend_for_test("Do not run while approval is pending.");
    let rejected = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"aggiungi una sezione"}"#,
    );

    assert_eq!(rejected["sessionId"], session_id);
    assert_eq!(rejected["state"], "blocked");
    assert_timeline_contains(&rejected, "session continuity pending user choice");
    assert!(!workspace_root.join("guide.md").exists());

    let approval_id =
        route_json(&mut router, "GET", "/v1/approvals", "")["approvals"][0]["approvalId"]
            .as_str()
            .unwrap()
            .to_string();
    router.complete_agent_backend_for_test("Approved action completed from observation.");
    let approval = route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    assert_eq!(approval["consumed"], true);
    let auto_resumed = route_json(&mut router, "GET", "/v1/agent/workspace", "");
    assert_eq!(auto_resumed["session"]["sessionId"], session_id);
    assert_eq!(auto_resumed["session"]["state"], "completed");
    assert_timeline_contains(
        &auto_resumed["session"],
        "Approved action completed from observation.",
    );
    assert_eq!(
        auto_resumed["session"]["timeline"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["message"] == "waiting for approval")
            .count(),
        1
    );
    let completed = route_json(
        &mut router,
        "POST",
        &format!("/v1/sessions/{session_id}/messages"),
        &format!(
            r#"{{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"Continue approved action","approvalId":"{approval_id}"}}"#
        ),
    );

    assert_eq!(completed["sessionId"], session_id);
    assert_eq!(completed["state"], "completed");
    assert_eq!(
        std::fs::read_to_string(workspace_root.join("guide.md")).unwrap(),
        "# Guide\n"
    );
}

#[test]
fn active_pending_session_survives_router_restart_and_workspace_reload() {
    let fixture = TempDir::new().expect("temp dir should exist");
    let db_path = fixture.path().join("desktoplab.sqlite");
    let workspace_root = fixture.path().join("workspace");
    create_repo(&workspace_root);

    let mut first_router =
        LocalApiRouter::with_storage_path(&db_path).expect("router should open storage");
    mark_setup_ready(&mut first_router);
    open_workspace(&mut first_router, &workspace_root);
    first_router.complete_agent_backend_for_test(
        r##"{"assistantMessage":"Creo restart.md.","desktoplabAction":{"kind":"create_file","path":"restart.md","content":"# Restart\n"}}"##,
    );

    let blocked = create_session(&mut first_router, "crea restart.md");
    assert_eq!(blocked["sessionId"], "session.1");
    assert_eq!(blocked["state"], "blocked");

    let mut restarted =
        LocalApiRouter::with_storage_path(&db_path).expect("router should reopen storage");
    let workspace = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");
    assert_eq!(workspace["session"]["sessionId"], "session.1");
    assert_eq!(workspace["session"]["state"], "blocked");
    assert_eq!(
        workspace["session"]["pendingApprovals"][0]["sessionId"],
        "session.1"
    );

    restarted.complete_agent_backend_for_test("This response must not create session.2.");
    let rapid = create_session(&mut restarted, "continua sullo stesso thread");
    assert_eq!(rapid["sessionId"], "session.1");
    assert_eq!(rapid["state"], "blocked");
    let sessions = route_json(&mut restarted, "GET", "/v1/sessions", "");
    assert_eq!(sessions["sessions"].as_array().unwrap().len(), 1);
}

#[test]
fn transcript_preserves_chronological_turn_interleaving() {
    let (_fixture, _workspace_root, mut router) = router_with_workspace();
    router.complete_agent_backend_for_test("Prima risposta.");
    let first = create_session(&mut router, "prima richiesta");
    let isolated_context = router
        .workspace_context_for_prompt_for_test(
            "workspace.workspace",
            "nuovo task nello stesso repository",
            &[],
        )
        .expect("new session context should exist");
    assert!(!isolated_context.contains("historical_completed_session"));
    assert!(!isolated_context.contains("prima richiesta"));
    assert!(!isolated_context.contains("Prima risposta."));
    let context = router
        .workspace_context_for_session_prompt_for_test(
            "workspace.workspace",
            first["sessionId"].as_str().unwrap(),
            "seconda richiesta",
        )
        .expect("follow-up context should exist");
    assert!(context.contains("user: prima richiesta"));
    assert!(context.contains("assistant: Prima risposta."));
    router.complete_agent_backend_for_test("Seconda risposta.");
    let continued = route_json(
        &mut router,
        "POST",
        &format!(
            "/v1/sessions/{}/messages",
            first["sessionId"].as_str().unwrap()
        ),
        r#"{"workspaceId":"workspace.workspace","executionBackendId":"backend.ollama","prompt":"seconda richiesta"}"#,
    );

    let turns = continued["transcript"].as_array().unwrap();
    let simplified = turns
        .iter()
        .map(|turn| {
            (
                turn["role"].as_str().unwrap().to_string(),
                turn["content"].as_str().unwrap().to_string(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        simplified,
        vec![
            ("user".to_string(), "prima richiesta".to_string()),
            ("assistant".to_string(), "Prima risposta.".to_string()),
            ("user".to_string(), "seconda richiesta".to_string()),
            ("assistant".to_string(), "Seconda risposta.".to_string()),
        ]
    );
}

#[test]
fn local_api_agent_session_continuity_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_session_continuity.rs",
        include_str!("local_api_agent_session_continuity.rs"),
        280,
    )
    .expect("agent session continuity tests should stay focused");
}

#[test]
fn approval_continuation_source_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/agent_continuation.rs",
        include_str!("../src/router/agent_continuation.rs"),
        250,
    )
    .expect("approval continuation source grew too large");
}

fn router_with_workspace() -> (TempDir, std::path::PathBuf, LocalApiRouter) {
    let fixture = TempDir::new().expect("temp workspace should exist");
    let workspace_root = fixture.path().join("workspace");
    create_repo(&workspace_root);
    let mut router = LocalApiRouter::default();
    router.enable_test_controls_for_dev_server();
    mark_setup_ready(&mut router);
    open_workspace(&mut router, &workspace_root);
    (fixture, workspace_root, router)
}

fn create_repo(path: &std::path::Path) {
    std::fs::create_dir_all(path).expect("workspace should exist");
    run_git(path, &["init", "-b", "main"]);
}

fn open_workspace(router: &mut LocalApiRouter, workspace_root: &std::path::Path) {
    route_json(
        router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace_root),
    );
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

fn assert_timeline_contains(session: &Value, expected: &str) {
    let timeline = session["timeline"].as_array().unwrap();
    assert!(
        timeline.iter().any(|event| event["message"]
            .as_str()
            .is_some_and(|message| message.contains(expected))),
        "{session}"
    );
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    route_json(
        router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.5.0");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    route_json(router, "POST", "/v1/setup/complete", "{}");
}

fn run_git(root: &std::path::Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("git command should run");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
