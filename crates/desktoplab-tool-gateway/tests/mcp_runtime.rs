use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use desktoplab_tool_gateway::{
    McpImportCandidate, McpServerConfig, McpTransportConfig, SharedMcpRuntime,
};
use serde_json::json;

#[test]
fn connected_mcp_tool_lists_invokes_redacts_and_disconnects() {
    let endpoint = serve_mcp(2);
    let runtime = SharedMcpRuntime::default();
    let tools = runtime
        .connect(
            candidate("server.docs", endpoint),
            vec!["repository.read".to_string()],
            false,
            true,
        )
        .unwrap();
    assert_eq!(tools[0].canonical_id(), "mcp.server_docs.docs_search");
    assert_eq!(tools[0].remote_name(), "docs.search");
    assert_eq!(
        tools[0].input_schema()["properties"]["query"]["type"],
        "string"
    );

    let response = runtime
        .invoke(tools[0].canonical_id(), json!({"query":"agent"}), false)
        .unwrap();
    assert_eq!(response["result"]["content"][0]["text"], "token=[REDACTED]");
    runtime.disconnect("server.docs").unwrap();
    assert!(runtime.tools().is_empty());
}

#[test]
fn untrusted_or_explicitly_sensitive_mcp_tools_require_approval() {
    let endpoint = serve_mcp(2);
    let runtime = SharedMcpRuntime::default();
    let tools = runtime
        .connect(
            candidate("server.remote", endpoint),
            vec!["network.read".to_string()],
            false,
            false,
        )
        .unwrap();
    assert_eq!(
        runtime
            .invoke(tools[0].canonical_id(), json!({"query":"agent"}), false)
            .unwrap_err(),
        "approval_required"
    );
    assert!(
        runtime
            .invoke(tools[0].canonical_id(), json!({"query":"agent"}), true)
            .is_ok()
    );
}

#[test]
fn import_requires_declared_scopes_and_review() {
    let runtime = SharedMcpRuntime::default();
    let mut unreviewed = candidate("server.unreviewed", serve_mcp(0));
    unreviewed.reviewed = false;
    assert_eq!(
        runtime
            .connect(unreviewed, vec!["read".to_string()], false, false)
            .unwrap_err(),
        "mcp_import_review_required"
    );
    assert_eq!(
        runtime
            .connect(
                candidate("server.no-scope", serve_mcp(0)),
                Vec::new(),
                false,
                true,
            )
            .unwrap_err(),
        "permission_scope_required"
    );
}

#[test]
fn mcp_runtime_sources_stay_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/mcp_runtime.rs",
        include_str!("../src/mcp_runtime.rs"),
        360,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/tests/mcp_runtime.rs",
        include_str!("mcp_runtime.rs"),
        180,
    )
    .unwrap();
}

fn candidate(server_id: &str, endpoint: String) -> McpImportCandidate {
    McpImportCandidate {
        config: McpServerConfig {
            server_id: server_id.to_string(),
            transport: McpTransportConfig::Http {
                endpoint,
                vault_ref: None,
                streaming: false,
            },
        },
        reviewed: true,
    }
}

fn serve_mcp(requests: usize) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            let id = json_rpc_id(&request);
            let result = if request.contains("tools/list") {
                json!({"tools":[{
                    "name":"docs.search","description":"Search docs",
                    "inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}
                }]})
            } else {
                assert!(request.contains("tools/call"), "{request}");
                json!({"content":[{"type":"text","text":"token=secret"}]})
            };
            let body = json!({"jsonrpc":"2.0","id":id,"result":result}).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        }
    });
    endpoint
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = [0_u8; 16 * 1024];
    let count = stream.read(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes[..count]).to_string()
}

fn json_rpc_id(request: &str) -> u64 {
    let body = request.split("\r\n\r\n").nth(1).unwrap();
    serde_json::from_str::<serde_json::Value>(body).unwrap()["id"]
        .as_u64()
        .unwrap()
}
