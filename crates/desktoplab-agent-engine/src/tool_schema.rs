use serde_json::{Value, json};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentToolRisk {
    Low,
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentToolExecutionOwner {
    CanonicalGateway,
    RouterControl,
    LoopControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentToolScope {
    Workspace,
    Session,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentToolSchema {
    id: String,
    description: String,
    risk: AgentToolRisk,
    requires_approval: bool,
    execution_owner: AgentToolExecutionOwner,
    scope: AgentToolScope,
    input_schema: Value,
    output_shape: Value,
}

impl AgentToolSchema {
    pub(crate) fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        risk: AgentToolRisk,
        requires_approval: bool,
        input_schema: Value,
        output_shape: Value,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            risk,
            requires_approval,
            execution_owner: AgentToolExecutionOwner::CanonicalGateway,
            scope: AgentToolScope::Workspace,
            input_schema,
            output_shape,
        }
    }

    pub(crate) fn with_execution(
        mut self,
        owner: AgentToolExecutionOwner,
        scope: AgentToolScope,
    ) -> Self {
        self.execution_owner = owner;
        self.scope = scope;
        self
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    #[must_use]
    pub fn risk(&self) -> AgentToolRisk {
        self.risk
    }

    #[must_use]
    pub fn requires_approval(&self) -> bool {
        self.requires_approval
    }

    #[must_use]
    pub fn execution_owner(&self) -> AgentToolExecutionOwner {
        self.execution_owner
    }

    #[must_use]
    pub fn scope(&self) -> AgentToolScope {
        self.scope
    }

    #[must_use]
    pub fn input_schema(&self) -> &Value {
        &self.input_schema
    }

    #[must_use]
    pub fn output_shape(&self) -> &Value {
        &self.output_shape
    }

    fn provider_schema(&self) -> Value {
        json!({
            "type":"function",
            "function":{
                "name":self.id,
                "description":self.description,
                "parameters":self.input_schema
            }
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DesktopLabToolRegistry {
    tools: Vec<AgentToolSchema>,
}

impl DesktopLabToolRegistry {
    pub(crate) fn from_tools(tools: Vec<AgentToolSchema>) -> Self {
        Self { tools }
    }

    #[must_use]
    pub fn tools(&self) -> &[AgentToolSchema] {
        &self.tools
    }

    #[must_use]
    pub fn get(&self, id: &str) -> Option<&AgentToolSchema> {
        self.tools.iter().find(|tool| tool.id == id)
    }

    pub fn with_mcp_tools(
        mut self,
        tools: impl IntoIterator<Item = AgentToolSchema>,
    ) -> Result<Self, String> {
        for tool in tools {
            crate::tool_schema_extensions::validate_mcp_tool(&tool)?;
            if self.get(tool.id()).is_some() {
                return Err("mcp_tool_id_duplicate".to_string());
            }
            self.tools.push(tool);
        }
        Ok(self)
    }

    #[must_use]
    pub fn provider_tool_schemas(&self) -> Vec<Value> {
        self.tools
            .iter()
            .map(AgentToolSchema::provider_schema)
            .collect()
    }

    #[must_use]
    pub fn strict_json_action_schema(&self) -> Value {
        let ids = self.tools.iter().map(|tool| tool.id()).collect::<Vec<_>>();
        json!({
            "type":"object",
            "properties":{
                "assistantMessage":{"type":"string"},
                "tool":{"type":"string","enum":ids},
                "arguments":{"type":"object"}
            },
            "required":["tool","arguments"],
            "additionalProperties":false
        })
    }
}
