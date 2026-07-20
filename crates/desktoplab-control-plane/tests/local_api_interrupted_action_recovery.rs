use desktoplab_backend_services::{SessionService, SessionServiceStore};
use desktoplab_control_plane::LocalApiRouter;
use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn restart_never_reconciles_actions_without_canonical_identity() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("desktoplab.sqlite");
    let workspace = fixture.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(workspace.join("done.md"), "verified\n").unwrap();
    let store = SqliteStore::open(&database).unwrap();
    store.apply_migrations().unwrap();
    let session_store =
        SessionServiceStore::with_storage(SqliteStore::open(&database).unwrap()).unwrap();
    let mut sessions = SessionService::new(session_store);
    let write_session = sessions.create_session("workspace.recovery", "backend.ollama");
    let terminal_session = sessions.create_session("workspace.recovery", "backend.ollama");
    store
        .put_productization_states(&[
            ProductizationStateRecord::new(
                ProductizationRecordKind::CurrentWorkspace,
                "current",
                json!({
                    "workspaceId":"workspace.recovery",
                    "displayName":"recovery",
                    "rootPath":workspace
                })
                .to_string(),
            ),
            ProductizationStateRecord::new(
                ProductizationRecordKind::AgentPendingAction,
                "local",
                json!({"actions":[
                    applying_action(
                        "approval.write",
                        write_session.session_id(),
                        json!({"kind":"filesystem.write","path":"done.md"}),
                        Some("verified\n")
                    ),
                    applying_action(
                        "approval.terminal",
                        terminal_session.session_id(),
                        json!({
                            "kind":"terminal.command",
                            "workspaceId":"workspace.recovery",
                            "workingDirectory":"",
                            "command":"unknown-effect",
                            "riskClass":"medium"
                        }),
                        None
                    )
                ]})
                .to_string(),
            ),
        ])
        .unwrap();

    drop(LocalApiRouter::with_storage_path(&database).unwrap());
    let record = store
        .get_productization_state(ProductizationRecordKind::AgentPendingAction, "local")
        .unwrap()
        .unwrap();
    let payload: Value = serde_json::from_str(record.payload()).unwrap();

    assert_eq!(action_state(&payload, "approval.write"), "interrupted");
    assert_eq!(action_state(&payload, "approval.terminal"), "interrupted");
}

fn applying_action(id: &str, session_id: &str, tool: Value, content: Option<&str>) -> Value {
    json!({
        "approvalId":id,
        "sessionId":session_id,
        "tool":tool,
        "content":content,
        "payloadHash":format!("hash.{id}"),
        "state":"applying",
        "readbackAfterWrite":true,
        "checkpointId":null,
        "checkpointStatus":null,
        "approvedChangeFingerprint":null
    })
}

fn action_state<'a>(payload: &'a Value, id: &str) -> &'a str {
    payload["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["approvalId"] == id)
        .unwrap()["state"]
        .as_str()
        .unwrap()
}

#[test]
fn restart_recovers_a_native_write_from_verified_postcondition() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("native.sqlite");
    let workspace = fixture.path().join("native-workspace");
    std::fs::create_dir(&workspace).unwrap();
    let initialized = std::process::Command::new("git")
        .args(["init", "-b", "main"])
        .current_dir(&workspace)
        .output()
        .unwrap();
    assert!(initialized.status.success());
    let mut router = LocalApiRouter::with_storage_path(&database).unwrap();
    mark_setup_ready(&mut router);
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"write-1","tool":"desktoplab.write_file","arguments":{"path":"done.md","content":"verified\n"}}"#,
    ]);
    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &json!({
            "workspaceId":opened["workspaceId"],
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Create done.md"
        })
        .to_string(),
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap()
        .to_string();
    drop(router);

    let store = SqliteStore::open(&database).unwrap();
    let record = store
        .get_productization_state(ProductizationRecordKind::AgentPendingAction, "local")
        .unwrap()
        .unwrap();
    let mut payload: Value = serde_json::from_str(record.payload()).unwrap();
    payload["actions"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|action| action["approvalId"] == approval_id)
        .unwrap()["state"] = json!("applying");
    store
        .put_productization_states(&[ProductizationStateRecord::new(
            ProductizationRecordKind::AgentPendingAction,
            "local",
            payload.to_string(),
        )])
        .unwrap();
    std::fs::write(workspace.join("done.md"), "verified\n").unwrap();

    let mut restarted = LocalApiRouter::with_storage_path(&database).unwrap();
    let state = route_json(&mut restarted, "GET", "/v1/agent/workspace", "");
    assert_eq!(state["session"]["state"], "running", "{state}");
    let record = store
        .get_productization_state(ProductizationRecordKind::AgentPendingAction, "local")
        .unwrap()
        .unwrap();
    let payload: Value = serde_json::from_str(record.payload()).unwrap();
    assert_eq!(action_state(&payload, &approval_id), "applied");
}

#[test]
fn interrupted_action_recovery_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_interrupted_action_recovery.rs",
        include_str!("local_api_interrupted_action_recovery.rs"),
        230,
    )
    .expect("interrupted action recovery tests should stay focused");
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

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
