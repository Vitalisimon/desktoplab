use desktoplab_agent_session::{AgentSession, SessionState};
use desktoplab_redaction::redact_sensitive_bounded;
use serde_json::{Value, json};

const PRIORITY: &[&str] = &[
    "unsafe_mutation",
    "suspected_verifier_gaming",
    "hallucinated_completion",
    "tool_misuse",
    "skipped_verification",
    "validation_failed",
    "state_regression",
    "repeated_error_loop",
    "timeout",
    "local_inference_failure",
    "model_transport_failure",
    "environment_unavailable",
    "failed_delegation",
    "memory_miss",
    "unclassified",
];

pub(crate) fn session_failure_payload(session: &AgentSession) -> Value {
    if session.state() != SessionState::Failed {
        return Value::Null;
    }
    let original = session.failed_reason().unwrap_or("agent run failed");
    let lower = original.to_ascii_lowercase();
    let mut codes = Vec::new();
    add_if(
        &mut codes,
        "unsafe_mutation",
        contains_any(&lower, &["unsafe mutation", "approval bypass"]),
    );
    add_if(
        &mut codes,
        "suspected_verifier_gaming",
        contains_any(&lower, &["verifier", "holdout"]),
    );
    add_if(
        &mut codes,
        "hallucinated_completion",
        contains_any(
            &lower,
            &[
                "claimed completion",
                "completion without evidence",
                "unsupported_test_claim",
            ],
        ),
    );
    add_if(
        &mut codes,
        "tool_misuse",
        contains_any(
            &lower,
            &[
                "unknown tool",
                "unknown_tool",
                "malformed structured",
                "invalid tool",
                "unrecognized_shape",
                "model_protocol_error",
            ],
        ),
    );
    add_if(
        &mut codes,
        "skipped_verification",
        contains_any(
            &lower,
            &[
                "verification missing",
                "verification skipped",
                "test not run",
            ],
        ),
    );
    add_if(
        &mut codes,
        "validation_failed",
        contains_any(&lower, &["tests_failed:", "validation_still_failing"])
            || unresolved_failed_validation(session),
    );
    add_if(
        &mut codes,
        "state_regression",
        contains_any(
            &lower,
            &["state regression", "session continuity", "replay failed"],
        ),
    );
    add_if(
        &mut codes,
        "repeated_error_loop",
        contains_any(
            &lower,
            &[
                "no_progress",
                "repeated read",
                "repeated error",
                "repeated action",
                "repeated_tool_failure",
            ],
        ),
    );
    add_if(
        &mut codes,
        "timeout",
        contains_any(&lower, &["timeout", "timed out"]),
    );
    add_if(
        &mut codes,
        "local_inference_failure",
        lower.contains("local_inference_failed"),
    );
    add_if(
        &mut codes,
        "model_transport_failure",
        contains_any(
            &lower,
            &[
                "ollama_request_failed",
                "ollama_http_status",
                "ollama_stream_read_failed",
                "lm_studio_request_failed",
                "lm_studio_http_status",
                "openai_compatible_local_request_failed",
                "openai_compatible_local_http_status",
                "openai_compatible_stream_request_failed",
                "openai_compatible_stream_http_status",
                "openai_compatible_stream_read_failed",
            ],
        ),
    );
    add_if(
        &mut codes,
        "environment_unavailable",
        environment_unavailable(&lower),
    );
    add_if(
        &mut codes,
        "failed_delegation",
        lower.contains("delegat") && lower.contains("fail"),
    );
    add_if(
        &mut codes,
        "memory_miss",
        lower.contains("memory") && contains_any(&lower, &["miss", "missing", "not found"]),
    );
    if codes.is_empty() {
        codes.push("unclassified");
    }
    codes.sort_by_key(|code| {
        PRIORITY
            .iter()
            .position(|candidate| candidate == code)
            .unwrap_or(usize::MAX)
    });
    let primary = codes[0];
    let redacted = redact_sensitive_bounded(original, 240);
    json!({
        "schemaVersion":1,
        "primary":primary,
        "findings":codes.iter().map(|code| json!({"code":code,"message":message(code)})).collect::<Vec<_>>(),
        "originalStopReason":redacted.value(),
        "userMessage":message(primary)
    })
}

fn add_if(codes: &mut Vec<&'static str>, code: &'static str, condition: bool) {
    if condition {
        codes.push(code);
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn environment_unavailable(value: &str) -> bool {
    contains_any(
        value,
        &[
            "runtime",
            "model",
            "provider",
            "environment",
            "workspace root",
        ],
    ) && contains_any(value, &["unavailable", "missing", "offline", "not ready"])
}

fn unresolved_failed_validation(session: &AgentSession) -> bool {
    use desktoplab_agent_session::SessionEvent;

    let mut next_terminal_is_validation = false;
    let mut latest_validation_passed = None;
    for event in session.event_log() {
        match event {
            SessionEvent::ToolDecisionRecorded { decision }
                if decision.contains("state=observed") || decision.contains("state=failed") =>
            {
                next_terminal_is_validation = decision.contains("canonical=desktoplab.run_tests");
            }
            SessionEvent::TerminalEvidenceRecorded { evidence } => {
                if next_terminal_is_validation {
                    latest_validation_passed = Some(evidence.exit_code() == Some(0));
                }
                next_terminal_is_validation = false;
            }
            _ => {}
        }
    }
    latest_validation_passed == Some(false)
}

fn message(code: &str) -> &'static str {
    match code {
        "unsafe_mutation" => "A workspace change did not satisfy the required safety controls.",
        "suspected_verifier_gaming" => "The run attempted to influence or bypass its verifier.",
        "hallucinated_completion" => "The agent claimed completion without executable proof.",
        "tool_misuse" => {
            "The model returned an invalid tool request. DesktopLab stopped without applying it."
        }
        "skipped_verification" => "The result was not verified before the run ended.",
        "validation_failed" => {
            "The latest validation command failed. Review the output, repair the issue, and run it again."
        }
        "state_regression" => "The session lost or contradicted previously recorded state.",
        "repeated_error_loop" => "The agent repeated the same failing action without progress.",
        "timeout" => "The agent run exceeded its time limit.",
        "local_inference_failure" => "Local inference failed before the agent could continue.",
        "model_transport_failure" => {
            "The local model runner stopped responding. Check that it is running, then retry the turn."
        }
        "environment_unavailable" => "The required local runtime or workspace was unavailable.",
        "failed_delegation" => "A delegated agent operation failed.",
        "memory_miss" => "Required saved workspace context was not recovered.",
        _ => "The agent run failed for an unclassified reason.",
    }
}

#[cfg(test)]
#[path = "agent_failure_tests.rs"]
mod agent_failure_tests;
