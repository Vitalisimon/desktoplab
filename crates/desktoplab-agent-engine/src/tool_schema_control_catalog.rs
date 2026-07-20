use crate::tool_schema_builders::{
    array_output, boolean_output, loop_tool, no_output, router_tool, string_output,
};
use crate::tool_schema_inputs::{
    clarification_input, completion_input, plan_input, subagent_id_input, subagent_message_input,
    subagent_spawn_input,
};
use crate::{AgentToolRisk, AgentToolSchema};

pub(crate) fn control_tools() -> Vec<AgentToolSchema> {
    vec![
        router_tool(
            "desktoplab.update_plan",
            "Create or replace the durable task plan for the current session. Keep steps concrete, mark completed work truthfully, and allow at most one in_progress step.",
            AgentToolRisk::Low,
            plan_input(),
            array_output("steps"),
        ),
        router_tool(
            "desktoplab.spawn_subagent",
            "Create a durable child agent session for a focused delegated task. Use read_only unless the child must prepare workspace changes in an isolated worktree. A write_capable child must commit completed changes before it becomes eligible for parent review and integration.",
            AgentToolRisk::Medium,
            subagent_spawn_input(),
            string_output("subagentId"),
        ),
        router_tool(
            "desktoplab.send_subagent",
            "Send an additional instruction to an active child agent owned by this session.",
            AgentToolRisk::Low,
            subagent_message_input(),
            string_output("state"),
        ),
        router_tool(
            "desktoplab.get_subagent",
            "Read the durable state and summary of an owned child. For write_capable children this also returns a bounded redacted Git changeReview. Integrate only when readyToIntegrate is true, by running the approval-gated cherry-pick of every listed commit hash in order.",
            AgentToolRisk::Low,
            subagent_id_input(),
            string_output("state"),
        ),
        router_tool(
            "desktoplab.cancel_subagent",
            "Cancel an active child agent owned by this session.",
            AgentToolRisk::Medium,
            subagent_id_input(),
            string_output("state"),
        ),
        router_tool(
            "desktoplab.close_subagent",
            "Close a terminal child agent owned by this session after its result has been observed.",
            AgentToolRisk::Low,
            subagent_id_input(),
            boolean_output("closed"),
        ),
        loop_tool(
            "desktoplab.complete",
            "Use when the user goal is satisfied. Classify the outcome from executor evidence: answered for read-only findings, executed for a successful non-mutation action, changed only with mutation evidence where changed=true, and verified only with passing test evidence; cite every successful executor call used as evidence. Use answered with an empty evidenceCallIds array only when no repository action or observation was needed.",
            AgentToolRisk::Low,
            completion_input(),
            no_output(),
        ),
        loop_tool(
            "desktoplab.clarify",
            "Ask only for a required user decision or value absent from executor observations. Never ask the user to restate or interpret observed repository content. Set blockedOn to the canonical action that cannot proceed.",
            AgentToolRisk::Low,
            clarification_input(),
            no_output(),
        ),
    ]
}
