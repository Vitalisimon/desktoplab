use crate::{AgentContext, AgentRunRequest, PlannedToolCall};
use desktoplab_tool_gateway::ToolIntent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstPromptStep {
    session_id: String,
    backend_id: String,
    prompt: String,
    read_path: String,
    planned_tool: Option<ToolIntent>,
    context: Option<AgentContext>,
}

impl FirstPromptStep {
    #[must_use]
    pub fn new(
        session_id: impl Into<String>,
        backend_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            backend_id: backend_id.into(),
            prompt: prompt.into(),
            read_path: String::new(),
            planned_tool: None,
            context: None,
        }
    }

    pub fn with_read_path(mut self, read_path: impl Into<String>) -> Self {
        self.read_path = read_path.into();
        self
    }

    #[must_use]
    pub fn with_planned_tool(mut self, intent: ToolIntent) -> Self {
        self.planned_tool = Some(intent);
        self
    }

    pub fn with_context(mut self, context: AgentContext) -> Self {
        self.context = (!context.is_empty()).then_some(context);
        self
    }

    #[must_use]
    pub fn compiled_prompt(&self) -> String {
        let Some(context) = &self.context else {
            return self.prompt.clone();
        };
        format!(
            "{}\n\nRepository context:\n{}\n\nCurrent user request (authoritative for this turn):\n{}",
            self.prompt,
            context.text(),
            self.prompt
        )
    }

    #[must_use]
    pub fn request(&self) -> AgentRunRequest {
        let mut request = AgentRunRequest::new(&self.session_id, &self.backend_id)
            .with_prompt(self.prompt.clone())
            .with_backend_prompt(self.compiled_prompt());
        let tool = self.planned_tool.clone().or_else(|| {
            (!self.read_path.is_empty())
                .then(|| ToolIntent::filesystem_read(self.read_path.clone()))
        });
        if let Some(tool) = tool {
            request = request.with_tool_call(PlannedToolCall::new(tool));
        }
        request
    }
}
