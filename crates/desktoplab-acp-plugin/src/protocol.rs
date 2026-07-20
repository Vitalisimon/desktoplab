use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AcpCapabilityStatus {
    Supported,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcpHostPrompt {
    pub message: String,
    pub stop_reason: String,
}

pub trait AcpSessionHost {
    fn create_session(&mut self, cwd: &str) -> Result<String, String>;
    fn prompt(&mut self, session_id: &str, prompt: &str) -> Result<AcpHostPrompt, String>;
    fn cancel(&mut self, session_id: &str) -> Result<(), String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcpDispatch {
    pub response: Option<Value>,
    pub notifications: Vec<Value>,
}

pub struct AcpProtocolAdapter<H> {
    host: H,
    initialized: bool,
}

impl<H: AcpSessionHost> AcpProtocolAdapter<H> {
    pub fn new(host: H) -> Self {
        Self {
            host,
            initialized: false,
        }
    }

    pub fn host(&self) -> &H {
        &self.host
    }

    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    pub fn dispatch(&mut self, request: &Value) -> AcpDispatch {
        let id = request.get("id").cloned();
        if request.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return failure(id, -32600, "invalid JSON-RPC envelope");
        }
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            return failure(id, -32600, "method missing");
        };
        if method == "initialize" {
            self.initialized = true;
            return success(id, initialize_result());
        }
        if !self.initialized {
            return failure(id, -32002, "ACP connection is not initialized");
        }
        match method {
            "session/new" => self.new_session(id, request.get("params")),
            "session/prompt" => self.prompt_session(id, request.get("params")),
            "session/cancel" => self.cancel_session(request.get("params")),
            _ => failure(id, -32601, "ACP method is not supported"),
        }
    }

    fn new_session(&mut self, id: Option<Value>, params: Option<&Value>) -> AcpDispatch {
        let Some(cwd) = params
            .and_then(|value| value.get("cwd"))
            .and_then(Value::as_str)
        else {
            return failure(id, -32602, "session/new requires cwd");
        };
        let mcp_servers = params
            .and_then(|value| value.get("mcpServers"))
            .and_then(Value::as_array);
        if mcp_servers.is_some_and(|servers| !servers.is_empty()) {
            return failure(
                id,
                -32602,
                "ACP MCP servers are not supported by this adapter",
            );
        }
        match self.host.create_session(cwd) {
            Ok(session_id) => success(id, json!({"sessionId":session_id})),
            Err(error) => failure(id, -32000, &error),
        }
    }

    fn prompt_session(&mut self, id: Option<Value>, params: Option<&Value>) -> AcpDispatch {
        let Some(session_id) = string_param(params, "sessionId") else {
            return failure(id, -32602, "session/prompt requires sessionId");
        };
        let Some(prompt) = text_prompt(params) else {
            return failure(id, -32602, "session/prompt requires text content");
        };
        match self.host.prompt(session_id, &prompt) {
            Ok(output) => AcpDispatch {
                response: response(id, json!({"stopReason":output.stop_reason})),
                notifications: vec![json!({
                    "jsonrpc":"2.0",
                    "method":"session/update",
                    "params":{"sessionId":session_id,"update":{
                        "sessionUpdate":"agent_message_chunk",
                        "content":{"type":"text","text":output.message}
                    }}
                })],
            },
            Err(error) => failure(id, -32000, &error),
        }
    }

    fn cancel_session(&mut self, params: Option<&Value>) -> AcpDispatch {
        let Some(session_id) = string_param(params, "sessionId") else {
            return failure(None, -32602, "session/cancel requires sessionId");
        };
        match self.host.cancel(session_id) {
            Ok(()) => AcpDispatch {
                response: None,
                notifications: Vec::new(),
            },
            Err(error) => failure(None, -32000, &error),
        }
    }
}

pub fn acp_capability_matrix() -> &'static [(&'static str, AcpCapabilityStatus)] {
    use AcpCapabilityStatus::{Supported, Unsupported};
    &[
        ("initialize", Supported),
        ("session/new", Supported),
        ("session/prompt", Supported),
        ("session/cancel", Supported),
        ("session/update", Supported),
        ("session/load", Unsupported),
        ("session/resume", Unsupported),
        ("session/close", Unsupported),
        ("session/list", Unsupported),
        ("session/delete", Unsupported),
        ("session/request_permission", Unsupported),
        ("$/cancel_request", Unsupported),
        ("mcp/stdio", Unsupported),
    ]
}

fn initialize_result() -> Value {
    json!({
        "protocolVersion":1,
        "agentCapabilities":{"loadSession":false,"promptCapabilities":{}},
        "agentInfo":{"name":"desktoplab","title":"DesktopLab","version":env!("CARGO_PKG_VERSION")},
        "authMethods":[]
    })
}

fn text_prompt(params: Option<&Value>) -> Option<String> {
    let blocks = params?.get("prompt")?.as_array()?;
    let text = blocks
        .iter()
        .filter_map(|block| {
            (block.get("type")?.as_str()? == "text")
                .then(|| block.get("text")?.as_str())
                .flatten()
        })
        .collect::<Vec<_>>()
        .join("\n");
    (!text.trim().is_empty()).then_some(text)
}

fn string_param<'a>(params: Option<&'a Value>, name: &str) -> Option<&'a str> {
    params?.get(name)?.as_str()
}

fn success(id: Option<Value>, result: Value) -> AcpDispatch {
    AcpDispatch {
        response: response(id, result),
        notifications: Vec::new(),
    }
}

fn response(id: Option<Value>, result: Value) -> Option<Value> {
    id.map(|id| json!({"jsonrpc":"2.0","id":id,"result":result}))
}

fn failure(id: Option<Value>, code: i64, message: &str) -> AcpDispatch {
    AcpDispatch {
        response: id
            .map(|id| json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})),
        notifications: Vec::new(),
    }
}
