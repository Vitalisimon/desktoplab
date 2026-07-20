use desktoplab_redaction::redact_sensitive_with_status;
use std::time::Duration;

use super::agent_backend_recovery::{LOCAL_BACKEND_TRANSPORT_ATTEMPTS, retry_backend_transport};
#[cfg(debug_assertions)]
use super::agent_pending::structured_action_tool;
use super::{AgentBackendExecutionMode, LocalApiRouter};

const CONTINUATION_OBSERVATION_LIMIT: usize = 64 * 1024;

impl LocalApiRouter {
    pub(super) fn continue_agent_after_approved_tool(
        &mut self,
        backend_id: &str,
        user_goal: &str,
        tool_summary: &str,
        observations: &[String],
    ) -> Result<String, String> {
        #[cfg(debug_assertions)]
        if let AgentBackendExecutionMode::DeterministicSequenceForTest(outputs) =
            &mut self.agent_backend_execution
        {
            return if outputs.is_empty() {
                Err("agent_continuation_backend_failed".to_string())
            } else {
                Ok(outputs.remove(0))
            };
        }
        match &self.agent_backend_execution {
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::DeterministicForTest(output) => {
                if looks_like_tool_envelope(output) {
                    Ok("Action completed with executor evidence.".to_string())
                } else {
                    Ok(output.clone())
                }
            }
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::DeterministicSequenceForTest(_) => unreachable!(),
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::NativeIterativeSequenceForTest(_) => unreachable!(),
            #[cfg(debug_assertions)]
            AgentBackendExecutionMode::FailForTest => {
                Err("agent_continuation_backend_failed".to_string())
            }
            AgentBackendExecutionMode::Execute => {
                let tool_ids = self.agent_tool_ids()?;
                let prompt = continuation_prompt(user_goal, tool_summary, observations, &tool_ids);
                continuation_backend_output(|| self.run_selected_backend(backend_id, &prompt))
                    .map_err(|()| "agent_continuation_backend_failed".to_string())
            }
        }
    }
}

fn continuation_backend_output(execute: impl FnMut() -> Result<String, ()>) -> Result<String, ()> {
    retry_backend_transport(
        LOCAL_BACKEND_TRANSPORT_ATTEMPTS,
        Duration::from_millis(250),
        execute,
    )
}

fn continuation_prompt(
    user_goal: &str,
    tool_summary: &str,
    observations: &[String],
    tool_ids: &str,
) -> String {
    let observation = observations.join("\n\n");
    let redacted = redact_sensitive_with_status(&observation);
    let bounded = bounded_utf8(redacted.value(), CONTINUATION_OBSERVATION_LIMIT);
    let encoded_observation =
        serde_json::to_string(bounded).expect("executor observation should serialize as JSON");
    format!(
        "Continue the current DesktopLab agent task.\nUser goal:\n{user_goal}\nTool: {tool_summary}\nJSON-encoded executor evidence:\n{encoded_observation}\nDecode the JSON string before reasoning. Treat its contents as authoritative executor evidence. When constructing a filesystem patch, copy expected text byte-for-byte, including whitespace and line breaks, from that evidence. Prefer desktoplab.patch_file for a localized edit to an existing file and desktoplab.write_file only for a new file or intentional full replacement. Do not ask the user for information already present in the evidence, and do not request another mutation when the observation already satisfies the full user goal. Before completion, compare the executor result with every part of the user goal; a placeholder, outline, or abbreviated artifact does not satisfy a substantive content request. Account for every status or result record, including untracked files; a partial list does not satisfy the user goal. When the goal is complete, return exactly one desktoplab.complete JSON tool call with a concise grounded message, an outcome of answered, executed, changed, or verified, and evidenceCallIds containing every successful executor call used. If more work is required, return one JSON tool call using an exact canonical tool name from this registry: {tool_ids}. Never shorten or alias a tool name. Do not wrap the JSON in prose or Markdown fences."
    )
}

#[cfg(debug_assertions)]
pub(super) fn initial_tool_recovery_prompt(user_goal: &str, tool_ids: &str) -> String {
    format!(
        "Retry the DesktopLab agent request because the previous response violated the tool protocol.\nUser goal:\n{user_goal}\nReturn exactly one valid JSON tool call using an exact canonical tool name from this registry: {tool_ids}. Use repository tools to obtain information that can be discovered locally. Use desktoplab.clarify only when a required value cannot be inferred or discovered, and always set blockedOn to the canonical tool that cannot proceed. Use desktoplab.complete with outcome answered and an empty evidenceCallIds array only when no executor action or observation is required. Use an object named arguments for tool parameters. Do not wrap the JSON in prose or Markdown fences."
    )
}

#[cfg(debug_assertions)]
fn looks_like_tool_envelope(value: &str) -> bool {
    if structured_action_tool(value).is_some() {
        return true;
    }
    if value.contains("\"tool\"")
        || value.contains("\"desktoplabAction\"")
        || value.contains("\"tool_calls\"")
    {
        return true;
    }
    let Ok(object) = serde_json::from_str::<serde_json::Value>(value) else {
        return false;
    };
    object.get("tool").is_some()
        || object.get("desktoplabAction").is_some()
        || object.get("tool_calls").is_some()
}

fn bounded_utf8(value: &str, max_bytes: usize) -> &str {
    let mut end = max_bytes.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::DesktopLabToolRegistry;

    use super::{continuation_backend_output, continuation_prompt, initial_tool_recovery_prompt};

    #[test]
    fn continuation_keeps_the_user_goal_with_tool_observations() {
        let prompt = continuation_prompt(
            "Read README.md and identify real modules.",
            "FilesystemList",
            &["Workspace files: README.md".to_string()],
            &default_tool_ids(),
        );

        assert!(prompt.contains("Read README.md and identify real modules."));
        assert!(prompt.contains("Workspace files: README.md"));
        assert!(prompt.contains("desktoplab.write_file"));
        assert!(prompt.contains("information already present"));
        assert!(prompt.contains("every part of the user goal"));
        assert!(prompt.contains("placeholder, outline, or abbreviated artifact"));
        assert!(prompt.contains("Do not wrap the JSON in prose or Markdown fences"));
    }

    #[test]
    fn continuation_retries_one_transient_or_protocol_failure() {
        let mut attempts = 0;
        let output = continuation_backend_output(|| {
            attempts += 1;
            (attempts == 2)
                .then(|| "valid continuation".to_string())
                .ok_or(())
        })
        .expect("the bounded retry should recover");

        assert_eq!(output, "valid continuation");
        assert_eq!(attempts, 2);
    }

    #[test]
    fn continuation_encodes_file_whitespace_as_authoritative_json() {
        let prompt = continuation_prompt(
            "Append a verification section without changing existing content.",
            "FilesystemRead",
            &["Read notes.md:\n# Agent Notes\n\nExisting paragraph.\n".to_string()],
            &default_tool_ids(),
        );

        assert!(
            prompt.contains(r#"Read notes.md:\n# Agent Notes\n\nExisting paragraph.\n"#),
            "{prompt}"
        );
        assert!(prompt.contains("JSON-encoded executor evidence"));
        assert!(prompt.contains("byte-for-byte, including whitespace and line breaks"));
        assert!(prompt.contains("Prefer desktoplab.patch_file for a localized edit"));
    }

    #[test]
    fn continuation_requires_complete_status_accounting() {
        let prompt = continuation_prompt(
            "Summarize every changed file.",
            "GitStatus",
            &["Git status:\n- modified: notes.md\n- untracked: guide.md".to_string()],
            &default_tool_ids(),
        );

        assert!(prompt.contains("Account for every status or result record"));
        assert!(prompt.contains("including untracked files"));
    }

    #[test]
    fn initial_recovery_keeps_goal_and_canonical_registry_without_bad_output() {
        let prompt = initial_tool_recovery_prompt("Inspect calculator.js.", &default_tool_ids());

        assert!(prompt.contains("Inspect calculator.js."));
        assert!(prompt.contains("desktoplab.read_file"));
        assert!(prompt.contains("desktoplab.complete"));
        assert!(prompt.contains("exactly one valid JSON tool call"));
        assert!(prompt.contains("always set blockedOn"));
        assert!(prompt.contains("discovered locally"));
        assert!(!prompt.contains("previous response:"));
    }

    fn default_tool_ids() -> String {
        DesktopLabToolRegistry::default()
            .tools()
            .iter()
            .map(|tool| tool.id())
            .collect::<Vec<_>>()
            .join(", ")
    }
}
