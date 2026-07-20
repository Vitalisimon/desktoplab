use std::process::Command;

use desktoplab_acp_plugin::AcpProtocolAdapter;
use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn external_json_rpc_client_reaches_real_desktoplab_session_service() {
    let repo = TempDir::new().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    let mut router = LocalApiRouter::default();
    mark_setup_ready(&mut router);
    router.complete_agent_backend_for_test("Repository inspected through ACP.");
    let mut client = AcpProtocolAdapter::new(router);

    let init = client.dispatch(&request(
        0,
        "initialize",
        json!({"protocolVersion":1,"clientCapabilities":{}}),
    ));
    assert_eq!(result(&init)["protocolVersion"], 1);
    let created = client.dispatch(&request(
        1,
        "session/new",
        json!({"cwd":repo.path(),"mcpServers":[]}),
    ));
    let session_id = result(&created)["sessionId"].as_str().unwrap().to_string();
    let prompt = client.dispatch(&request(
        2,
        "session/prompt",
        json!({
            "sessionId":session_id,"prompt":[{"type":"text","text":"Inspect this repository"}]
        }),
    ));
    assert_eq!(result(&prompt)["stopReason"], "end_turn");
    assert_eq!(
        prompt.notifications[0]["params"]["update"]["content"]["text"],
        "Repository inspected through ACP."
    );
    let listed = client.host_mut().route("GET", "/v1/sessions", "").unwrap();
    assert!(listed.body().contains(&session_id));
    let inspect = client
        .host_mut()
        .route("GET", "/v1/runtime/inspect", "")
        .unwrap();
    let inspect: Value = serde_json::from_str(inspect.body()).unwrap();
    assert_eq!(inspect["protocolAdapters"]["acp"]["protocolVersion"], 1);
    assert_eq!(
        inspect["protocolAdapters"]["acp"]["publicExecutionStatus"],
        "not_registered"
    );
}

#[test]
fn external_client_smoke_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/acp_external_client_smoke.rs",
        include_str!("acp_external_client_smoke.rs"),
        120,
    )
    .unwrap();
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
}

fn result(dispatch: &desktoplab_acp_plugin::AcpDispatch) -> &Value {
    &dispatch.response.as_ref().unwrap()["result"]
}

fn mark_setup_ready(router: &mut LocalApiRouter) {
    router.set_host_memory_gb_for_test(32);
    router.route(
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    router.mark_runtime_verified_for_test("runtime.ollama", "ollama 0.6.2");
    router.mark_model_verified_for_test("runtime.ollama", "model.gemma4-12b-q4", "gemma4:12b");
    router.route("POST", "/v1/setup/complete", "{}");
}
