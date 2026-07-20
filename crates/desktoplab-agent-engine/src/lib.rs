#![forbid(unsafe_code)]

mod context;
mod context_planner;
mod final_response;
mod inference;
mod inference_endpoint;
mod iterative_approval;
mod iterative_loop;
mod iterative_protocol;
mod iterative_resume;
mod iterative_state;
mod json_schema_validator;
mod llm;
mod loop_engine;
mod loop_events;
mod mcp_schema_validator;
mod observation;
mod output_sanitizer;
mod parallel_policy;
mod planning;
mod product_loop;
mod prompt_step;
mod request;
mod retry;
mod tool_call_normalizer;
mod tool_decisions;
mod tool_failure_guard;
mod tool_loop;
mod tool_schema;
mod tool_schema_builders;
mod tool_schema_catalog;
mod tool_schema_control_catalog;
mod tool_schema_extensions;
mod tool_schema_inputs;
mod tool_schema_process_catalog;
mod tool_telemetry;
mod trace;

pub use context::{AgentContext, AgentContextBuilder};
pub use context_planner::{
    AgentContextPlan, AgentContextPlanner, AgentRouteContextCapabilities, ContextBudgetReport,
    ContextCandidate, ContextInclusionReason, ContextPlannedItem, ContextSectionKind,
    ContextStrategy,
};
pub use inference::{
    LocalInferenceAdapter, LocalInferenceError, LocalInferenceEvidence, LocalInferenceRequest,
    LocalInferenceResult, LocalInferenceTransport,
};
pub use inference_endpoint::{
    OpenAiCompatibleEndpoint, OpenAiCompatibleEndpointClass, OpenAiCompatibleEndpointError,
    OpenAiCompatibleEndpointPolicy,
};
pub use iterative_approval::{IterativeApproval, IterativeApprovalDecision, PendingToolApproval};
pub use iterative_loop::{IterativeAgentLoop, IterativeLoopLimits};
pub use iterative_protocol::{
    IterativeLoopEvent, IterativeLoopStatus, IterativeModelAdapter, IterativeModelDecision,
    IterativeStopReason, IterativeToolCall, IterativeToolExecutor,
};
pub use iterative_state::IterativeLoopState;
pub use llm::{LlmExecutionAdapter, LlmExecutionError, LlmExecutionStream};
pub use loop_engine::{AgentEvidence, AgentLoop, AgentRunResult, ApprovalDecision};
pub use observation::{ObservationProvenance, ToolObservation};
pub use parallel_policy::{AgentParallelDecision, AgentParallelIntent, AgentParallelPolicy};
pub use planning::{
    MultiFileRefactorFile, MultiFileRefactorPlan, MultiFileRefactorRequest, RefactorPlanError,
};
pub use product_loop::{
    AgentPlan, AgentPlanStore, AgentPlanner, ExecutionBackendAvailability, FileEditEngine,
    FileEditResult, SessionControl, TestCommandProposal, TestFeedback, TestFeedbackLoop,
};
pub use prompt_step::FirstPromptStep;
pub use request::{AgentRunRequest, PlannedToolCall};
pub use retry::{
    AgentFailureKind, FailureObservation, RetryAttempt, RetryDecision, RetryEvaluation, RetryPolicy,
};
pub use tool_call_normalizer::{ProviderToolCallNormalizer, ToolCallNormalizationError};
pub use tool_loop::{
    BoundedToolLoop, ToolLoopLimits, ToolLoopRunResult, ToolLoopStep, ToolLoopStopReason,
};
pub use tool_schema::{
    AgentToolExecutionOwner, AgentToolRisk, AgentToolSchema, AgentToolScope, DesktopLabToolRegistry,
};
pub use trace::{AgentTraceEnvelope, AgentTraceEvent};
