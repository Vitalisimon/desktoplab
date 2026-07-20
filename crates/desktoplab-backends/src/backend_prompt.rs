use serde_json::{Value, json};

use crate::BackendToolSchema;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BackendPrompt {
    model: String,
    messages: Vec<BackendMessage>,
    tools: Vec<BackendToolSchema>,
    context_window_tokens: Option<u32>,
    request_timeout_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BackendMessage {
    User(String),
    Assistant(String),
    AssistantToolCall {
        call_id: String,
        name: String,
        arguments: Value,
    },
    ToolResult {
        call_id: String,
        name: String,
        output: Value,
    },
}

impl BackendMessage {
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self::User(content.into())
    }

    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant(content.into())
    }

    #[must_use]
    pub fn assistant_tool_call(
        call_id: impl Into<String>,
        name: impl Into<String>,
        arguments: Value,
    ) -> Self {
        Self::AssistantToolCall {
            call_id: call_id.into(),
            name: name.into(),
            arguments,
        }
    }

    #[must_use]
    pub fn tool_result(call_id: impl Into<String>, name: impl Into<String>, output: Value) -> Self {
        Self::ToolResult {
            call_id: call_id.into(),
            name: name.into(),
            output,
        }
    }

    fn ollama_value(&self) -> Value {
        match self {
            Self::User(content) => json!({"role":"user","content":content}),
            Self::Assistant(content) => json!({"role":"assistant","content":content}),
            Self::AssistantToolCall {
                call_id,
                name,
                arguments,
            } => json!({
                "role":"assistant",
                "content":"",
                "tool_calls":[{
                    "id":call_id,
                    "type":"function",
                    "function":{"name":name,"arguments":arguments}
                }]
            }),
            Self::ToolResult {
                call_id,
                name: _,
                output,
            } => json!({
                "role":"tool",
                "tool_call_id":call_id,
                "content":output.to_string()
            }),
        }
    }

    fn openai_value(&self) -> Value {
        match self {
            Self::User(content) => json!({"role":"user","content":content}),
            Self::Assistant(content) => json!({"role":"assistant","content":content}),
            Self::AssistantToolCall {
                call_id,
                name,
                arguments,
            } => json!({
                "role":"assistant",
                "content":null,
                "tool_calls":[{
                    "id":call_id,
                    "type":"function",
                    "function":{"name":name,"arguments":arguments.to_string()}
                }]
            }),
            Self::ToolResult {
                call_id,
                name: _,
                output,
            } => json!({
                "role":"tool",
                "tool_call_id":call_id,
                "content":output.to_string()
            }),
        }
    }
}

impl BackendPrompt {
    #[must_use]
    pub fn new(model: impl Into<String>, prompt: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            messages: vec![BackendMessage::user(prompt)],
            tools: Vec::new(),
            context_window_tokens: None,
            request_timeout_seconds: None,
        }
    }

    #[must_use]
    pub fn with_tools(mut self, tools: Vec<BackendToolSchema>) -> Self {
        self.tools = tools;
        self
    }

    #[must_use]
    pub fn with_messages(mut self, messages: Vec<BackendMessage>) -> Self {
        self.messages = messages;
        self
    }

    #[must_use]
    pub fn with_context_window_tokens(mut self, context_window_tokens: u32) -> Self {
        self.context_window_tokens = Some(context_window_tokens);
        self
    }

    #[must_use]
    pub fn with_request_timeout_seconds(mut self, request_timeout_seconds: u64) -> Self {
        self.request_timeout_seconds = Some(request_timeout_seconds);
        self
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub(crate) fn ollama_messages(&self) -> Vec<Value> {
        self.messages
            .iter()
            .map(BackendMessage::ollama_value)
            .collect()
    }

    pub(crate) fn openai_messages(&self) -> Vec<Value> {
        self.messages
            .iter()
            .map(BackendMessage::openai_value)
            .collect()
    }

    pub(crate) fn tools(&self) -> &[BackendToolSchema] {
        &self.tools
    }

    pub fn context_window_tokens(&self) -> Option<u32> {
        self.context_window_tokens
    }

    pub fn request_timeout_seconds(&self) -> Option<u64> {
        self.request_timeout_seconds
    }
}
