use std::fmt;

use serde_json::Value;

use crate::json_schema_validator::validate_value;
use crate::{DesktopLabToolRegistry, IterativeToolCall};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolCallNormalizationError {
    MalformedEnvelope,
    MalformedArguments,
    UnknownTool,
    MissingArgument(String),
    UnexpectedArgument(String),
    InvalidArgumentType(String),
    InvalidArgument(String),
}

impl fmt::Display for ToolCallNormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedEnvelope => write!(formatter, "malformed_tool_call_envelope"),
            Self::MalformedArguments => write!(formatter, "malformed_tool_call_arguments"),
            Self::UnknownTool => write!(formatter, "unknown_tool"),
            Self::MissingArgument(name) => write!(formatter, "missing_argument:{name}"),
            Self::UnexpectedArgument(name) => write!(formatter, "unexpected_argument:{name}"),
            Self::InvalidArgumentType(name) => write!(formatter, "invalid_argument_type:{name}"),
            Self::InvalidArgument(name) => write!(formatter, "invalid_argument:{name}"),
        }
    }
}

impl std::error::Error for ToolCallNormalizationError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderToolCallNormalizer {
    registry: DesktopLabToolRegistry,
}

impl ProviderToolCallNormalizer {
    #[must_use]
    pub fn new(registry: DesktopLabToolRegistry) -> Self {
        Self { registry }
    }

    pub fn from_provider_value(
        &self,
        value: &Value,
    ) -> Result<IterativeToolCall, ToolCallNormalizationError> {
        if value.get("function").is_some() && value.get("tool").is_some() {
            return Err(ToolCallNormalizationError::MalformedEnvelope);
        }
        let id = value
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or(ToolCallNormalizationError::MalformedEnvelope)?;
        let (name, arguments) = if let Some(function) = value.get("function") {
            let name = function
                .get("name")
                .and_then(Value::as_str)
                .ok_or(ToolCallNormalizationError::MalformedEnvelope)?;
            let raw = function
                .get("arguments")
                .ok_or(ToolCallNormalizationError::MalformedEnvelope)?;
            (name, parse_arguments(raw)?)
        } else {
            let name = value
                .get("tool")
                .and_then(Value::as_str)
                .ok_or(ToolCallNormalizationError::MalformedEnvelope)?;
            let raw = value
                .get("arguments")
                .ok_or(ToolCallNormalizationError::MalformedEnvelope)?;
            (name, parse_arguments(raw)?)
        };
        self.normalize(id, name, arguments)
    }

    pub fn normalize(
        &self,
        id: impl Into<String>,
        name: &str,
        arguments: Value,
    ) -> Result<IterativeToolCall, ToolCallNormalizationError> {
        let schema = self
            .registry
            .get(name)
            .ok_or(ToolCallNormalizationError::UnknownTool)?;
        let input = schema.input_schema();
        validate_value(&arguments, input, "")?;
        Ok(IterativeToolCall::new(id, name, arguments))
    }

    pub fn validate_output(
        &self,
        name: &str,
        output: &Value,
    ) -> Result<(), ToolCallNormalizationError> {
        let schema = self
            .registry
            .get(name)
            .ok_or(ToolCallNormalizationError::UnknownTool)?;
        validate_value(output, schema.output_shape(), "")
    }
}

impl Default for ProviderToolCallNormalizer {
    fn default() -> Self {
        Self::new(DesktopLabToolRegistry::default())
    }
}

fn parse_arguments(value: &Value) -> Result<Value, ToolCallNormalizationError> {
    match value {
        Value::Object(_) => Ok(value.clone()),
        Value::String(raw) => {
            serde_json::from_str(raw).map_err(|_| ToolCallNormalizationError::MalformedArguments)
        }
        _ => Err(ToolCallNormalizationError::MalformedArguments),
    }
}
