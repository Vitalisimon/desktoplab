use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use desktoplab_tool_gateway::{
    McpConnection, McpConnectionPool, McpImportCandidate, McpServerConfig, McpTokenSource,
    McpToolSurface, McpTransportConfig, NoMcpToken,
};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn reviewed_stdio_and_http_transports_return_equivalent_json_rpc() {
    let fixture = TempDir::new().unwrap();
    let server = fixture.path().join("mcp-server.mjs");
    fs::write(
        &server,
        "import readline from 'node:readline'; const lines=readline.createInterface({input:process.stdin}); lines.on('line',(line)=>{const request=JSON.parse(line); console.log(JSON.stringify({jsonrpc:'2.0',id:request.id,result:{tools:[{name:'git.diff',description:'Inspect git changes',inputSchema:{type:'object'}},{name:'calendar.list',description:'List calendar',inputSchema:{type:'object'}}]}}));});",
    )
    .unwrap();
    let program = if cfg!(windows) { "node.exe" } else { "node" };
    let mut stdio = McpConnection::connect(candidate(
        "stdio",
        McpTransportConfig::Stdio {
            program: program.to_string(),
            args: vec![server.display().to_string()],
        },
    ))
    .unwrap();
    let stdio_response = stdio
        .request("tools/list", json!({}), &mut NoMcpToken)
        .unwrap();
    stdio.close().unwrap();

    let endpoint = serve_http(false);
    let mut http = McpConnection::connect(candidate(
        "http",
        McpTransportConfig::Http {
            endpoint,
            vault_ref: None,
            streaming: false,
        },
    ))
    .unwrap();
    let http_response = http
        .request("tools/list", json!({}), &mut NoMcpToken)
        .unwrap();
    assert_eq!(stdio_response["result"], http_response["result"]);
}

#[test]
fn streaming_http_refreshes_vault_token_once_without_exposing_it() {
    let endpoint = serve_http(true);
    let mut connection = McpConnection::connect(candidate(
        "stream",
        McpTransportConfig::Http {
            endpoint,
            vault_ref: Some("vault://mcp/server".to_string()),
            streaming: true,
        },
    ))
    .unwrap();
    let mut tokens = RotatingToken { calls: Vec::new() };
    let response = connection
        .request("tools/list", json!({}), &mut tokens)
        .unwrap();
    assert_eq!(response["result"]["tools"][0]["name"], "git.diff");
    assert_eq!(tokens.calls, vec![false, true]);
    assert!(!format!("{response:?}").contains("secret"));
}

#[test]
fn import_review_pooling_and_relevant_typed_surfaces_are_bounded() {
    let endpoint = serve_http(false);
    let unreviewed = McpImportCandidate {
        config: McpServerConfig {
            server_id: "server.unreviewed".to_string(),
            transport: McpTransportConfig::Http {
                endpoint: endpoint.clone(),
                vault_ref: None,
                streaming: false,
            },
        },
        reviewed: false,
    };
    assert_eq!(
        McpConnection::connect(unreviewed).err().as_deref(),
        Some("mcp_import_review_required")
    );

    let mut pool = McpConnectionPool::new();
    pool.import(candidate(
        "server.reviewed",
        McpTransportConfig::Http {
            endpoint,
            vault_ref: None,
            streaming: false,
        },
    ))
    .unwrap();
    let response = pool
        .request("server.reviewed", "tools/list", json!({}), &mut NoMcpToken)
        .unwrap();
    let surface = McpToolSurface::from_tools_list("server.reviewed", &response).unwrap();
    let relevant = surface.relevant_to("inspect git changes", 1);
    assert_eq!(relevant.tools.len(), 1);
    assert_eq!(relevant.tools[0].name, "git.diff");
    assert_eq!(pool.healthy_servers(), vec!["server.reviewed"]);
    assert_eq!(relevant.stable_json(), relevant.stable_json());
}

#[test]
fn mcp_transport_sources_stay_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/mcp_transport.rs",
        include_str!("../src/mcp_transport.rs"),
        390,
    )
    .unwrap();
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/mcp_surface.rs",
        include_str!("../src/mcp_surface.rs"),
        140,
    )
    .unwrap();
}

struct RotatingToken {
    calls: Vec<bool>,
}

impl McpTokenSource for RotatingToken {
    fn access_token(&mut self, vault_ref: &str, refresh: bool) -> Result<String, String> {
        assert_eq!(vault_ref, "vault://mcp/server");
        self.calls.push(refresh);
        Ok(if refresh {
            "fresh-secret"
        } else {
            "stale-secret"
        }
        .to_string())
    }
}

fn candidate(server_id: &str, transport: McpTransportConfig) -> McpImportCandidate {
    McpImportCandidate {
        config: McpServerConfig {
            server_id: server_id.to_string(),
            transport,
        },
        reviewed: true,
    }
}

fn serve_http(refresh: bool) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    thread::spawn(move || {
        let attempts = if refresh { 2 } else { 1 };
        for attempt in 0..attempts {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            assert!(request.contains("tools/list"));
            if refresh && attempt == 0 {
                assert!(request.contains("Bearer stale-secret"));
                write_response(
                    &mut stream,
                    401,
                    "application/json",
                    r#"{"error":"expired"}"#,
                );
                continue;
            }
            if refresh {
                assert!(request.contains("Bearer fresh-secret"));
            }
            let body = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[{"name":"git.diff","description":"Inspect git changes","inputSchema":{"type":"object"}},{"name":"calendar.list","description":"List calendar","inputSchema":{"type":"object"}}]}}"#;
            if refresh {
                write_response(
                    &mut stream,
                    200,
                    "text/event-stream",
                    &format!("data: {body}\n\n"),
                );
            } else {
                write_response(&mut stream, 200, "application/json", body);
            }
        }
    });
    endpoint
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut bytes = [0_u8; 8192];
    let count = stream.read(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes[..count]).to_string()
}

fn write_response(stream: &mut std::net::TcpStream, status: u16, content_type: &str, body: &str) {
    let reason = if status == 200 { "OK" } else { "Unauthorized" };
    write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
}
