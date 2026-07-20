#![forbid(unsafe_code)]

mod batch_patch;
mod filesystem;
mod filesystem_mutation;
mod gateway;
mod git;
mod intent;
mod intent_execution;
mod managed_process;
mod mcp_runtime;
mod mcp_surface;
mod mcp_transport;
mod patch;
mod path_security;
mod process;
mod process_platform;
mod terminal;
mod terminal_classification;
mod terminal_event;
mod test_runner;
mod tool_identity;
mod workspace_root;

pub use batch_patch::{BatchPatchItem, BatchPatchOutcome, FilesystemBatchPatchExecutor};
pub use filesystem::{FilesystemApproval, FilesystemToolExecutor, FilesystemToolOutcome};
pub use filesystem_mutation::{FilesystemMutationExecutor, FilesystemMutationOutcome};
pub use gateway::{ApprovalRequest, ToolAuditRecord, ToolGateway, ToolOutcome};
pub use git::{GitToolExecutor, GitToolOutcome, ParallelGitExecution};
pub use intent::{TerminalRiskClass, ToolIntent};
pub use managed_process::{ManagedProcessSnapshot, ManagedProcessState, SharedProcessRegistry};
pub use mcp_runtime::{ConnectedMcpTool, SharedMcpRuntime};
pub use mcp_surface::{McpToolSurface, McpTypedTool};
pub use mcp_transport::{
    McpConnection, McpConnectionPool, McpImportCandidate, McpServerConfig, McpTokenSource,
    McpTransportConfig, NoMcpToken,
};
pub use patch::{
    FilesystemPatchApproval, FilesystemPatchEvidence, FilesystemPatchExecutor,
    FilesystemPatchOutcome, FilesystemPatchRequest,
};
pub use process::{
    TerminalProcessAdapter, TerminalProcessOutput, TerminalProcessRequest, TerminalProcessStatus,
};
pub use terminal::{
    TerminalApproval, TerminalCommandRequest, TerminalExecutionResult, TerminalExecutionStatus,
    TerminalToolExecutor, TerminalToolOutcome,
};
pub use terminal_classification::{TerminalCommandClass, classify_terminal_command};
pub use terminal_event::TerminalOutputEvent;
pub use test_runner::{
    SelectedTestCommand, TestCommandSelection, TestRunApproval, TestRunEvidence, TestRunOutcome,
    TestRunRequest, TestRunnerExecutor,
};
pub use tool_identity::{canonical_tool_from_record, canonical_tool_mutates};
pub use workspace_root::{WorkspacePathState, WorkspaceRoot, WorkspaceRootError};
