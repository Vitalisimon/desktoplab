use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use desktoplab_control_plane::LocalApiRouter;
use serde_json::{Value, json};

#[test]
fn reviewed_mcp_registration_reconnects_after_router_restart() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("desktoplab.sqlite");
    let (endpoint, server) = serve_tools_lists(2);
    {
        let mut router =
            LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database).unwrap();
        let imported = route_json(
            &mut router,
            "POST",
            "/v1/mcp/servers/import",
            &json!({
                "serverId":"server.docs","reviewed":true,
                "permissionScopes":["repository.read"],
                "requiresApproval":true,"trustedServer":false,
                "transport":{"kind":"http","endpoint":endpoint}
            })
            .to_string(),
        );
        assert_eq!(
            imported["tools"][0]["toolId"],
            "mcp.server_docs.docs_search"
        );
    }

    let mut restored =
        LocalApiRouter::with_storage_path_without_host_recovery_for_test(&database).unwrap();
    let tools = route_json(&mut restored, "GET", "/v1/mcp/tools", "");
    server.join().unwrap();

    assert_eq!(tools["servers"][0]["status"], "connected");
    assert_eq!(tools["tools"][0]["toolId"], "mcp.server_docs.docs_search");
    assert_eq!(tools["tools"][0]["requiresApproval"], true);
}

#[test]
fn mcp_persistence_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_mcp_persistence.rs",
        include_str!("local_api_mcp_persistence.rs"),
        120,
    )
    .unwrap();
}

fn serve_tools_lists(requests: usize) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0_u8; 16 * 1024];
            let count = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..count]);
            let id = serde_json::from_str::<Value>(request.split("\r\n\r\n").nth(1).unwrap())
                .unwrap()["id"]
                .as_u64()
                .unwrap();
            let body = json!({"jsonrpc":"2.0","id":id,"result":{"tools":[{
                "name":"docs.search","description":"Search docs",
                "inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}
            }]}}).to_string();
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    (endpoint, server)
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router.route(method, path, body).unwrap();
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).unwrap()
}
