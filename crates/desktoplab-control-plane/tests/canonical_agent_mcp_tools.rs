use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use desktoplab_agent_engine::{IterativeToolCall, IterativeToolExecutor};
use desktoplab_control_plane::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};
use desktoplab_tool_gateway::{
    McpImportCandidate, McpServerConfig, McpTransportConfig, SharedMcpRuntime,
};
use serde_json::{Value, json};

#[test]
fn canonical_executor_invokes_connected_mcp_tools_and_honors_approval() {
    let endpoint = serve_mcp(2);
    let runtime = SharedMcpRuntime::default();
    let tools = runtime
        .connect(
            candidate(endpoint),
            vec!["repository.read".to_string()],
            false,
            false,
        )
        .unwrap();
    let call = IterativeToolCall::new(
        "call.mcp.1",
        tools[0].canonical_id(),
        json!({"query":"agent runtime"}),
    );
    let root = tempfile::tempdir().unwrap();
    let mut executor = CanonicalAgentToolExecutor::new(
        root.path(),
        "workspace.test",
        "session.test",
        CanonicalExecutionApproval::Pending,
    )
    .with_mcp_runtime(runtime)
    .unwrap();

    assert_eq!(executor.execute(&call).unwrap_err(), "approval_required");
    let observation = executor.execute_approved(&call).unwrap();
    assert_eq!(
        observation.output()["result"]["content"][0]["text"],
        "found"
    );
}

#[test]
fn canonical_mcp_executor_test_stays_small() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/canonical_agent_mcp_tools.rs",
        include_str!("canonical_agent_mcp_tools.rs"),
        120,
    )
    .unwrap();
}

fn candidate(endpoint: String) -> McpImportCandidate {
    McpImportCandidate {
        config: McpServerConfig {
            server_id: "server.docs".to_string(),
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
            let id = request["id"].as_u64().unwrap();
            let result = if request["method"] == "tools/list" {
                json!({"tools":[{
                    "name":"docs.search","description":"Search repository docs",
                    "inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}
                }]})
            } else {
                assert_eq!(request["method"], "tools/call");
                assert_eq!(request["params"]["arguments"]["query"], "agent runtime");
                json!({"content":[{"type":"text","text":"found"}]})
            };
            let body = json!({"jsonrpc":"2.0","id":id,"result":result}).to_string();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(), body
            )
            .unwrap();
        }
    });
    endpoint
}

fn read_request(stream: &mut std::net::TcpStream) -> Value {
    let mut bytes = [0_u8; 16 * 1024];
    let count = stream.read(&mut bytes).unwrap();
    let request = String::from_utf8_lossy(&bytes[..count]);
    serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap()
}
