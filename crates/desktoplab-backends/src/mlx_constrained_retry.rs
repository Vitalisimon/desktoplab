use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::BackendPrompt;

/// Retry only a provider-declared truncation with no emitted content. Never
/// interpret private reasoning or a partially emitted tool call as an action.
pub(crate) fn execute(
    prompt: &BackendPrompt,
    payload: &mut Value,
    mut send: impl FnMut(&Value, &BackendPrompt) -> Result<Value, String>,
) -> Result<Value, String> {
    let started = Instant::now();
    let first = send(payload, prompt)?;
    if first["choices"][0]["finish_reason"].as_str() != Some("length") {
        return Ok(first);
    }
    if first["choices"][0]["message"]["content"]
        .as_str()
        .is_some_and(|content| !content.is_empty())
    {
        return Err("mlx_constrained_response_truncated".to_string());
    }
    let mut retry_prompt = prompt.clone();
    if let Some(seconds) = prompt.request_timeout_seconds() {
        let remaining = Duration::from_secs(seconds).saturating_sub(started.elapsed());
        if remaining.as_secs() == 0 {
            return Err("mlx_constrained_deadline_exhausted".to_string());
        }
        retry_prompt = retry_prompt.with_request_timeout_seconds(remaining.as_secs());
    }
    payload["chat_template_kwargs"] = json!({"enable_thinking":false});
    let retry = send(payload, &retry_prompt)?;
    if retry["choices"][0]["finish_reason"].as_str() == Some("length") {
        return Err("mlx_constrained_response_truncated".to_string());
    }
    Ok(retry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_is_bounded_and_partial_or_failed_responses_never_retry() {
        let prompt = BackendPrompt::new("model", "task");
        let mut calls = 0;
        let truncated = json!({"choices":[{"finish_reason":"length","message":{"content":""}}]});
        let error = execute(&prompt, &mut json!({}), |_, _| {
            calls += 1;
            Ok(truncated.clone())
        })
        .unwrap_err();
        assert_eq!(error, "mlx_constrained_response_truncated");
        assert_eq!(calls, 2);

        calls = 0;
        let partial =
            json!({"choices":[{"finish_reason":"length","message":{"content":"partial"}}]});
        let error = execute(&prompt, &mut json!({}), |_, _| {
            calls += 1;
            Ok(partial.clone())
        })
        .unwrap_err();
        assert_eq!(error, "mlx_constrained_response_truncated");
        assert_eq!(calls, 1);

        calls = 0;
        let error = execute(&prompt, &mut json!({}), |_, _| {
            calls += 1;
            Err("network".to_string())
        })
        .unwrap_err();
        assert_eq!(error, "network");
        assert_eq!(calls, 1);
    }

    #[test]
    fn exhausted_deadline_prevents_a_retry() {
        let prompt = BackendPrompt::new("model", "task").with_request_timeout_seconds(0);
        let mut calls = 0;
        let error = execute(&prompt, &mut json!({}), |_, _| {
            calls += 1;
            Ok(json!({"choices":[{"finish_reason":"length","message":{}}]}))
        })
        .unwrap_err();
        assert_eq!(error, "mlx_constrained_deadline_exhausted");
        assert_eq!(calls, 1);
    }
}
