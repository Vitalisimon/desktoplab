use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{Value, json};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum McpTransportConfig {
    Stdio {
        program: String,
        args: Vec<String>,
    },
    Http {
        endpoint: String,
        vault_ref: Option<String>,
        streaming: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpServerConfig {
    pub server_id: String,
    pub transport: McpTransportConfig,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpImportCandidate {
    pub config: McpServerConfig,
    pub reviewed: bool,
}

pub trait McpTokenSource {
    fn access_token(&mut self, vault_ref: &str, refresh: bool) -> Result<String, String>;
}

pub struct NoMcpToken;

impl McpTokenSource for NoMcpToken {
    fn access_token(&mut self, _vault_ref: &str, _refresh: bool) -> Result<String, String> {
        Err("mcp_token_unavailable".to_string())
    }
}

enum ConnectedTransport {
    Stdio(StdioConnection),
    Http(HttpConnection),
}

pub struct McpConnection {
    server_id: String,
    transport: ConnectedTransport,
    next_id: u64,
    healthy: bool,
}

impl McpConnection {
    pub fn connect(candidate: McpImportCandidate) -> Result<Self, String> {
        if !candidate.reviewed {
            return Err("mcp_import_review_required".to_string());
        }
        if candidate.config.server_id.is_empty() {
            return Err("mcp_server_id_required".to_string());
        }
        let transport = match candidate.config.transport {
            McpTransportConfig::Stdio { program, args } => {
                ConnectedTransport::Stdio(StdioConnection::spawn(&program, &args)?)
            }
            McpTransportConfig::Http {
                endpoint,
                vault_ref,
                streaming,
            } => ConnectedTransport::Http(HttpConnection::new(endpoint, vault_ref, streaming)?),
        };
        Ok(Self {
            server_id: candidate.config.server_id,
            transport,
            next_id: 1,
            healthy: true,
        })
    }

    pub fn request(
        &mut self,
        method: &str,
        params: Value,
        tokens: &mut dyn McpTokenSource,
    ) -> Result<Value, String> {
        if !self.healthy {
            return Err("mcp_connection_unhealthy".to_string());
        }
        let request = json!({"jsonrpc":"2.0","id":self.next_id,"method":method,"params":params});
        self.next_id = self.next_id.saturating_add(1);
        let response = match &mut self.transport {
            ConnectedTransport::Stdio(connection) => connection.request(&request),
            ConnectedTransport::Http(connection) => connection.request(&request, tokens),
        };
        match response {
            Ok(value) if value.get("error").is_none() => Ok(value),
            Ok(_) => Err("mcp_protocol_error".to_string()),
            Err(error) => {
                self.healthy = false;
                Err(error)
            }
        }
    }

    pub fn server_id(&self) -> &str {
        &self.server_id
    }

    pub fn healthy(&self) -> bool {
        self.healthy
    }

    pub fn close(mut self) -> Result<(), String> {
        if let ConnectedTransport::Stdio(connection) = &mut self.transport {
            connection.close()?;
        }
        self.healthy = false;
        Ok(())
    }
}

pub struct McpConnectionPool {
    connections: BTreeMap<String, McpConnection>,
}

impl McpConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
        }
    }

    pub fn import(&mut self, candidate: McpImportCandidate) -> Result<(), String> {
        let connection = McpConnection::connect(candidate)?;
        let server_id = connection.server_id().to_string();
        if self.connections.contains_key(&server_id) {
            return Err("mcp_server_already_connected".to_string());
        }
        self.connections.insert(server_id, connection);
        Ok(())
    }

    pub fn request(
        &mut self,
        server_id: &str,
        method: &str,
        params: Value,
        tokens: &mut dyn McpTokenSource,
    ) -> Result<Value, String> {
        self.connections
            .get_mut(server_id)
            .ok_or_else(|| "mcp_server_not_connected".to_string())?
            .request(method, params, tokens)
    }

    pub fn healthy_servers(&self) -> Vec<String> {
        self.connections
            .iter()
            .filter(|(_, connection)| connection.healthy())
            .map(|(server_id, _)| server_id.clone())
            .collect()
    }

    pub fn disconnect(&mut self, server_id: &str) -> Result<(), String> {
        self.connections
            .remove(server_id)
            .ok_or_else(|| "mcp_server_not_connected".to_string())?
            .close()
    }
}

impl Default for McpConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

struct StdioConnection {
    child: Child,
    stdin: ChildStdin,
    responses: Receiver<Result<String, String>>,
    reader: Option<JoinHandle<()>>,
}

impl Drop for StdioConnection {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

impl StdioConnection {
    fn spawn(program: &str, args: &[String]) -> Result<Self, String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "mcp_stdio_spawn_failed".to_string())?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "mcp_stdio_missing_stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "mcp_stdio_missing_stdout".to_string())?;
        let (sender, responses) = mpsc::sync_channel(16);
        let reader = thread::spawn(move || read_stdio_responses(stdout, sender));
        Ok(Self {
            child,
            stdin,
            responses,
            reader: Some(reader),
        })
    }

    fn request(&mut self, request: &Value) -> Result<Value, String> {
        writeln!(self.stdin, "{request}").map_err(|_| "mcp_stdio_write_failed".to_string())?;
        self.stdin
            .flush()
            .map_err(|_| "mcp_stdio_write_failed".to_string())?;
        let line = self
            .responses
            .recv_timeout(Duration::from_secs(20))
            .map_err(|_| "mcp_stdio_response_timeout".to_string())??;
        serde_json::from_str(&line).map_err(|_| "mcp_stdio_response_invalid".to_string())
    }

    fn close(&mut self) -> Result<(), String> {
        self.child
            .kill()
            .or_else(|error| {
                if self.child.try_wait().ok().flatten().is_some() {
                    Ok(())
                } else {
                    Err(error)
                }
            })
            .map_err(|_| "mcp_stdio_close_failed".to_string())?;
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        Ok(())
    }
}

fn read_stdio_responses(stdout: ChildStdout, sender: mpsc::SyncSender<Result<String, String>>) {
    for line in BufReader::new(stdout).lines() {
        let value = line.map_err(|_| "mcp_stdio_read_failed".to_string());
        if sender.send(value).is_err() {
            break;
        }
    }
}

struct HttpConnection {
    client: Client,
    endpoint: String,
    vault_ref: Option<String>,
    streaming: bool,
}

impl HttpConnection {
    fn new(endpoint: String, vault_ref: Option<String>, streaming: bool) -> Result<Self, String> {
        if !endpoint.starts_with("http://127.0.0.1:")
            && !endpoint.starts_with("http://localhost:")
            && !endpoint.starts_with("https://")
        {
            return Err("mcp_http_endpoint_not_allowed".to_string());
        }
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|_| "mcp_http_client_failed".to_string())?;
        Ok(Self {
            client,
            endpoint,
            vault_ref,
            streaming,
        })
    }

    fn request(&self, payload: &Value, tokens: &mut dyn McpTokenSource) -> Result<Value, String> {
        let first = self.send(payload, tokens, false)?;
        if first.status().as_u16() == 401 && self.vault_ref.is_some() {
            return self.parse(self.send(payload, tokens, true)?);
        }
        self.parse(first)
    }

    fn send(
        &self,
        payload: &Value,
        tokens: &mut dyn McpTokenSource,
        refresh: bool,
    ) -> Result<reqwest::blocking::Response, String> {
        let mut request = self
            .client
            .post(&self.endpoint)
            .header(CONTENT_TYPE, "application/json")
            .header(
                ACCEPT,
                if self.streaming {
                    "text/event-stream"
                } else {
                    "application/json"
                },
            )
            .json(payload);
        if let Some(vault_ref) = &self.vault_ref {
            let token = tokens.access_token(vault_ref, refresh)?;
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        request
            .send()
            .map_err(|_| "mcp_http_request_failed".to_string())
    }

    fn parse(&self, response: reqwest::blocking::Response) -> Result<Value, String> {
        if !response.status().is_success() {
            return Err(format!("mcp_http_status:{}", response.status().as_u16()));
        }
        let text = response
            .text()
            .map_err(|_| "mcp_http_body_failed".to_string())?;
        if self.streaming {
            return text
                .lines()
                .find_map(|line| line.strip_prefix("data: "))
                .ok_or_else(|| "mcp_stream_response_missing".to_string())
                .and_then(|value| {
                    serde_json::from_str(value)
                        .map_err(|_| "mcp_stream_response_invalid".to_string())
                });
        }
        serde_json::from_str(&text).map_err(|_| "mcp_http_response_invalid".to_string())
    }
}
