use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};
use xtask::check_logical_line_limit;

#[test]
fn mcp_tool_route_only_exposes_live_reviewed_connections() {
    let mut router = LocalApiRouter::default();
    let empty = route_json(&mut router, "GET", "/v1/mcp/tools", "");
    assert_eq!(empty["source"], "runtime_backed");
    assert_eq!(empty["tools"], json!([]));

    let endpoint = serve_tools_list();
    let imported = route_json(
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
    let tool = &imported["tools"][0];
    assert_eq!(tool["toolId"], "mcp.server_docs.docs_search");
    assert_eq!(tool["serverId"], "server.docs");
    assert_eq!(tool["status"], "connected");
    assert_eq!(tool["requiresApproval"], false);
    assert_eq!(tool["inputSchema"]["properties"]["query"]["type"], "string");

    let disconnected = route_json(
        &mut router,
        "POST",
        "/v1/mcp/servers/server.docs/disconnect",
        "",
    );
    assert_eq!(disconnected["tools"], json!([]));
}

#[test]
fn mcp_import_rejects_unreviewed_or_unscoped_servers() {
    let mut router = LocalApiRouter::default();
    let endpoint = serve_tools_list();
    let response = router
        .route(
            "POST",
            "/v1/mcp/servers/import",
            &json!({
                "serverId":"server.unreviewed",
                "permissionScopes":["repository.read"],
                "transport":{"kind":"http","endpoint":endpoint}
            })
            .to_string(),
        )
        .unwrap();
    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("mcp_import_review_required"));

    let response = router
        .route(
            "POST",
            "/v1/mcp/servers/import",
            &json!({
                "serverId":"server.unscoped",
                "reviewed":true,
                "permissionScopes":[],
                "transport":{"kind":"http","endpoint":"http://127.0.0.1:1"}
            })
            .to_string(),
        )
        .unwrap();
    assert_eq!(response.status(), "400 Bad Request");
    assert!(response.body().contains("permission_scope_required"));
}

#[test]
fn mcp_tool_bridge_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/src/router/mcp.rs",
        include_str!("../src/router/mcp.rs"),
        260,
    )
    .expect("MCP router should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_mcp_tool_bridge.rs",
        include_str!("local_api_mcp_tool_bridge.rs"),
        150,
    )
    .expect("MCP bridge route tests should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn serve_tools_list() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut bytes = [0_u8; 16 * 1024];
        let count = stream.read(&mut bytes).unwrap();
        let request = String::from_utf8_lossy(&bytes[..count]);
        let id = request
            .split("\r\n\r\n")
            .nth(1)
            .and_then(|body| serde_json::from_str::<Value>(body).ok())
            .and_then(|body| body["id"].as_u64())
            .unwrap();
        let body = json!({
            "jsonrpc":"2.0","id":id,
            "result":{"tools":[{
                "name":"docs.search","description":"Search docs",
                "inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}
            }]}
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(), body
        )
        .unwrap();
    });
    endpoint
}
