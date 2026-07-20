use std::path::{Path, PathBuf};

use desktoplab_agent_engine::{
    AgentToolSchema, DesktopLabToolRegistry, IterativeToolCall, IterativeToolExecutor,
    ProviderToolCallNormalizer, ToolObservation,
};
use desktoplab_policy::ApprovalMode;
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::{SharedMcpRuntime, SharedProcessRegistry};
use serde_json::Value;

use crate::mcp_tokens::NativeMcpTokenSource;
use crate::{canonical_tool_files, canonical_tool_git, canonical_tool_process};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalExecutionApproval {
    Pending,
    Approved,
    Denied,
}

pub struct CanonicalAgentToolExecutor {
    root: PathBuf,
    workspace_id: String,
    session_id: String,
    approval: CanonicalExecutionApproval,
    normalizer: ProviderToolCallNormalizer,
    process_registry: SharedProcessRegistry,
    mcp_runtime: SharedMcpRuntime,
    approval_mode: ApprovalMode,
}

impl CanonicalAgentToolExecutor {
    #[must_use]
    pub fn new(
        root: &Path,
        workspace_id: impl Into<String>,
        session_id: impl Into<String>,
        approval: CanonicalExecutionApproval,
    ) -> Self {
        Self {
            root: root.to_path_buf(),
            workspace_id: workspace_id.into(),
            session_id: session_id.into(),
            approval,
            normalizer: ProviderToolCallNormalizer::default(),
            process_registry: SharedProcessRegistry::default(),
            mcp_runtime: SharedMcpRuntime::default(),
            approval_mode: ApprovalMode::RequireApproval,
        }
    }

    #[must_use]
    pub fn with_process_registry(mut self, process_registry: SharedProcessRegistry) -> Self {
        self.process_registry = process_registry;
        self
    }

    pub fn with_mcp_runtime(mut self, mcp_runtime: SharedMcpRuntime) -> Result<Self, String> {
        let registry = registry_with_mcp_tools(&mcp_runtime)?;
        self.normalizer = ProviderToolCallNormalizer::new(registry);
        self.mcp_runtime = mcp_runtime;
        Ok(self)
    }

    #[must_use]
    pub fn with_approval_mode(mut self, approval_mode: ApprovalMode) -> Self {
        self.approval_mode = approval_mode;
        self
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }

    pub(crate) fn approval(&self) -> CanonicalExecutionApproval {
        self.approval
    }

    pub(crate) fn policy(&self) -> PolicyEngine {
        PolicyEngine::default_conservative().with_approval_mode(self.approval_mode)
    }

    pub(crate) fn process_registry(&self) -> &SharedProcessRegistry {
        &self.process_registry
    }

    pub fn execute_provider_call(&mut self, value: &Value) -> Result<ToolObservation, String> {
        let call = self
            .normalizer
            .from_provider_value(value)
            .map_err(|error| error.to_string())?;
        self.execute(&call)
    }

    fn dispatch(&self, call: &IterativeToolCall) -> Result<Value, String> {
        match call.name() {
            "desktoplab.list_files"
            | "desktoplab.read_file"
            | "desktoplab.search_text"
            | "desktoplab.write_file"
            | "desktoplab.patch_file"
            | "desktoplab.create_directory"
            | "desktoplab.move_path"
            | "desktoplab.delete_path" => canonical_tool_files::execute(self, call),
            "desktoplab.run_terminal"
            | "desktoplab.run_tests"
            | "desktoplab.start_process"
            | "desktoplab.poll_process"
            | "desktoplab.write_process_stdin"
            | "desktoplab.kill_process" => canonical_tool_process::execute(self, call),
            "desktoplab.git_status"
            | "desktoplab.git_diff"
            | "desktoplab.create_checkpoint"
            | "desktoplab.commit_changes"
            | "desktoplab.push_changes" => canonical_tool_git::execute(self, call),
            name if name.starts_with("mcp.") => match self.approval {
                CanonicalExecutionApproval::Denied => Err("approval_denied".to_string()),
                CanonicalExecutionApproval::Pending | CanonicalExecutionApproval::Approved => {
                    self.mcp_runtime.invoke_with_tokens(
                        name,
                        call.arguments().clone(),
                        self.approval == CanonicalExecutionApproval::Approved,
                        &mut NativeMcpTokenSource,
                    )
                }
            },
            _ => Err("unsupported_canonical_tool".to_string()),
        }
    }
}

pub(crate) fn registry_with_mcp_tools(
    runtime: &SharedMcpRuntime,
) -> Result<DesktopLabToolRegistry, String> {
    let schemas = runtime
        .tools()
        .into_iter()
        .map(|tool| {
            AgentToolSchema::mcp(
                tool.canonical_id(),
                tool.description(),
                tool.requires_approval(),
                tool.input_schema().clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    DesktopLabToolRegistry::default().with_mcp_tools(schemas)
}

impl IterativeToolExecutor for CanonicalAgentToolExecutor {
    fn execute(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        self.normalizer
            .normalize(call.id(), call.name(), call.arguments().clone())
            .map_err(|error| error.to_string())?;
        let output = self.dispatch(call)?;
        self.normalizer
            .validate_output(call.name(), &output)
            .map_err(|error| format!("tool_output_contract_violation:{error}"))?;
        Ok(match execution_failure(call, &output) {
            Some(reason) => ToolObservation::failure_with_output(call, output, reason),
            None => ToolObservation::success(call, output),
        })
    }

    fn execute_approved(&mut self, call: &IterativeToolCall) -> Result<ToolObservation, String> {
        let previous = self.approval;
        self.approval = CanonicalExecutionApproval::Approved;
        let result = self.execute(call);
        self.approval = previous;
        result
    }
}

fn execution_failure(call: &IterativeToolCall, output: &Value) -> Option<String> {
    if !matches!(
        call.name(),
        "desktoplab.run_terminal" | "desktoplab.run_tests"
    ) {
        return None;
    }
    match (
        output.get("status").and_then(Value::as_str),
        output.get("exitCode").and_then(Value::as_i64),
    ) {
        (Some("exited"), Some(0)) => None,
        (Some("exited"), Some(code)) if call.name() == "desktoplab.run_tests" => {
            Some(format!("tests_failed:{code}"))
        }
        (Some("exited"), Some(code)) => Some(format!("command_exit_nonzero:{code}")),
        (Some(status), _) => Some(format!("command_{status}")),
        _ => Some("command_result_invalid".to_string()),
    }
}

pub(crate) fn required_string<'a>(
    call: &'a IterativeToolCall,
    name: &str,
) -> Result<&'a str, String> {
    call.arguments()
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing_argument:{name}"))
}

pub(crate) fn string_argument<'a>(
    call: &'a IterativeToolCall,
    name: &str,
) -> Result<&'a str, String> {
    call.arguments()
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing_argument:{name}"))
}

pub(crate) fn optional_string<'a>(call: &'a IterativeToolCall, name: &str) -> Option<&'a str> {
    call.arguments()
        .get(name)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

pub(crate) fn optional_usize(
    call: &IterativeToolCall,
    name: &str,
    default: usize,
    maximum: usize,
) -> Result<usize, String> {
    let Some(value) = call.arguments().get(name) else {
        return Ok(default);
    };
    let value = value
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| *value <= maximum)
        .ok_or_else(|| format!("invalid_argument:{name}"))?;
    if name == "limit" && value == 0 {
        return Err("invalid_argument:limit".to_string());
    }
    Ok(value)
}
