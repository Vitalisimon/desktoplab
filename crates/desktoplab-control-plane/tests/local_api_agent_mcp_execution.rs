use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::thread;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use tempfile::TempDir;

#[test]
fn native_agent_discovers_invokes_and_observes_a_connected_mcp_tool() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    let (endpoint, server) = serve_mcp();
    let (mut router, workspace_id) = ready_router(workspace.path());
    route_json(
        &mut router,
        "POST",
        "/v1/mcp/servers/import",
        &json!({
            "serverId":"server.docs",
            "reviewed":true,
            "permissionScopes":["repository.read"],
            "requiresApproval":false,
            "trustedServer":true,
            "transport":{"kind":"http","endpoint":endpoint}
        })
        .to_string(),
    );
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"mcp-1","tool":"mcp.server_docs.docs_search","arguments":{"query":"agent runtime"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"MCP docs search returned grounded evidence.","outcome":"executed","evidenceCallIds":["mcp-1"]}}"#,
    ]);

    let completed = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Use the connected docs search tool."
        })
        .to_string(),
    );
    server.join().unwrap();

    assert_eq!(completed["state"], "completed", "{completed}");
    assert_eq!(
        completed["summary"],
        "MCP docs search returned grounded evidence."
    );
    let timeline = completed["timeline"].as_array().unwrap();
    assert!(timeline.iter().any(|event| {
        event["message"]
            .as_str()
            .is_some_and(|message| message.contains("mcp.server_docs.docs_search"))
    }));
}

#[test]
fn untrusted_mcp_tool_pauses_executes_once_after_approval_and_resumes() {
    let workspace = TempDir::new().unwrap();
    create_repo(workspace.path());
    let (endpoint, server) = serve_mcp();
    let (mut router, workspace_id) = ready_router(workspace.path());
    route_json(
        &mut router,
        "POST",
        "/v1/mcp/servers/import",
        &json!({
            "serverId":"server.docs",
            "reviewed":true,
            "permissionScopes":["repository.read"],
            "requiresApproval":false,
            "trustedServer":false,
            "transport":{"kind":"http","endpoint":endpoint}
        })
        .to_string(),
    );
    router.complete_native_iterative_backend_sequence_for_test([
        r#"{"id":"mcp-1","tool":"mcp.server_docs.docs_search","arguments":{"query":"agent runtime"}}"#,
        r#"{"tool":"desktoplab.complete","arguments":{"message":"Approved MCP evidence received.","outcome":"executed","evidenceCallIds":["mcp-1"]}}"#,
    ]);

    let blocked = route_json(
        &mut router,
        "POST",
        "/v1/sessions",
        &json!({
            "workspaceId":workspace_id,
            "executionBackendId":"backend.ollama",
            "initialPrompt":"Use the connected docs search tool."
        })
        .to_string(),
    );
    assert_eq!(blocked["state"], "blocked", "{blocked}");
    assert_eq!(blocked["pendingApprovals"][0]["action"], "mcp.tool.invoke");
    assert_eq!(
        blocked["pendingApprovals"][0]["operationId"],
        "mcp.invoke:mcp.server_docs.docs_search"
    );
    let approval_id = blocked["pendingApprovals"][0]["approvalId"]
        .as_str()
        .unwrap();
    route_json(
        &mut router,
        "POST",
        &format!("/v1/approvals/{approval_id}/resolve"),
        r#"{"resolution":"approve"}"#,
    );
    server.join().unwrap();
    let completed = route_json(&mut router, "GET", "/v1/agent/workspace", "");

    assert_eq!(completed["session"]["state"], "completed", "{completed}");
    assert_eq!(
        completed["session"]["summary"],
        "Approved MCP evidence received."
    );
}

#[test]
fn native_agent_mcp_product_test_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_agent_mcp_execution.rs",
        include_str!("local_api_agent_mcp_execution.rs"),
        240,
    )
    .unwrap();
}

fn serve_mcp() -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        for ordinal in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let id = request["id"].as_u64().unwrap();
            let result = if ordinal == 0 {
                assert_eq!(request["method"], "tools/list");
                json!({"tools":[{
                    "name":"docs.search","description":"Search repository docs",
                    "inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}
                }]})
            } else {
                assert_eq!(request["method"], "tools/call");
                assert_eq!(request["params"]["arguments"]["query"], "agent runtime");
                json!({"content":[{"type":"text","text":"agent evidence"}]})
            };
            let body = json!({"jsonrpc":"2.0","id":id,"result":result}).to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    (endpoint, server)
}

fn read_request(stream: &mut std::net::TcpStream) -> Value {
    let mut bytes = [0_u8; 16 * 1024];
    let count = stream.read(&mut bytes).unwrap();
    let request = String::from_utf8_lossy(&bytes[..count]);
    serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}

fn ready_router(workspace: &std::path::Path) -> (LocalApiRouter, String) {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b    5.2 GB"]);
    route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );
    route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"gemma4:12b    5.2 GB"}"#,
    );
    router.mark_ollama_model_capabilities_for_test("gemma4:12b", &["completion", "tools"]);
    route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    let opened = route_json(
        &mut router,
        "POST",
        "/v1/workspaces/open",
        &xtask::test_http::workspace_open_body(&workspace),
    );
    (router, opened["workspaceId"].as_str().unwrap().to_string())
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .expect("route should exist");
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}

fn create_repo(path: &std::path::Path) {
    assert!(
        Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .status()
            .unwrap()
            .success()
    );
}
