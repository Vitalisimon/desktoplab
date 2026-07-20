use desktoplab_tool_gateway::ToolIntent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedToolCall {
    intent: ToolIntent,
}

impl PlannedToolCall {
    #[must_use]
    pub fn new(intent: ToolIntent) -> Self {
        Self { intent }
    }

    #[must_use]
    pub fn intent(&self) -> &ToolIntent {
        &self.intent
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentRunRequest {
    session_id: String,
    backend_id: String,
    prompt: Option<String>,
    backend_prompt: Option<String>,
    backend_response: Option<String>,
    tool_calls: Vec<PlannedToolCall>,
    diff: Option<String>,
    test_result: Option<String>,
}

impl AgentRunRequest {
    #[must_use]
    pub fn new(session_id: impl Into<String>, backend_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            backend_id: backend_id.into(),
            prompt: None,
            backend_prompt: None,
            backend_response: None,
            tool_calls: Vec::new(),
            diff: None,
            test_result: None,
        }
    }

    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    #[must_use]
    pub fn with_backend_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.backend_prompt = Some(prompt.into());
        self
    }

    #[must_use]
    pub fn with_backend_response(mut self, response: impl Into<String>) -> Self {
        self.backend_response = Some(response.into());
        self
    }

    #[must_use]
    pub fn with_tool_call(mut self, tool_call: PlannedToolCall) -> Self {
        self.tool_calls.push(tool_call);
        self
    }

    #[must_use]
    pub fn with_diff(mut self, diff: impl Into<String>) -> Self {
        self.diff = Some(diff.into());
        self
    }

    #[must_use]
    pub fn with_test_result(mut self, test_result: impl Into<String>) -> Self {
        self.test_result = Some(test_result.into());
        self
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn backend_id(&self) -> &str {
        &self.backend_id
    }

    pub fn prompt(&self) -> Option<&str> {
        self.prompt.as_deref()
    }

    pub fn backend_prompt(&self) -> Option<&str> {
        self.backend_prompt.as_deref()
    }

    pub(crate) fn backend_response(&self) -> Option<&str> {
        self.backend_response.as_deref()
    }

    pub(crate) fn tool_calls(&self) -> &[PlannedToolCall] {
        &self.tool_calls
    }

    pub(crate) fn diff(&self) -> Option<&str> {
        self.diff.as_deref()
    }

    pub(crate) fn test_result(&self) -> Option<&str> {
        self.test_result.as_deref()
    }
}
