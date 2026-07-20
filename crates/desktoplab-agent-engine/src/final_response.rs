use serde_json::Value;

use crate::IterativeLoopState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FinalResponseError {
    RawToolEnvelope,
    UnsupportedTestClaim,
}

pub(crate) fn validate(
    response: &str,
    state: &IterativeLoopState,
) -> Result<(), FinalResponseError> {
    if is_raw_tool_envelope(response) {
        return Err(FinalResponseError::RawToolEnvelope);
    }
    if claims_tests_passed(response) && !state.has_passing_test_evidence() {
        return Err(FinalResponseError::UnsupportedTestClaim);
    }
    Ok(())
}

fn is_raw_tool_envelope(response: &str) -> bool {
    let trimmed = response.trim();
    if trimmed.contains("```json") || trimmed.contains("```JSON") {
        return true;
    }
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(trimmed) else {
        return false;
    };
    [
        "tool",
        "function",
        "assistantMessage",
        "desktoplabAction",
        "tool_calls",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}

fn claims_tests_passed(response: &str) -> bool {
    let normalized = response.to_ascii_lowercase();
    [
        "test passed",
        "tests passed",
        "test passati",
        "test superati",
        "test succeeded",
        "tests succeeded",
    ]
    .iter()
    .any(|claim| normalized.contains(claim))
}
