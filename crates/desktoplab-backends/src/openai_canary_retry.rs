use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Retry only responses explicitly truncated by the provider, with one shared
/// deadline and a finite completion budget. Other errors remain fail-closed.
pub(super) fn request_bounded(
    payload: &mut Value,
    timeout: Duration,
    mut send: impl FnMut(&Value, Duration) -> Result<Value, String>,
) -> Result<Value, String> {
    let started = Instant::now();
    for budget in [512_u64, 1024, 2048] {
        let remaining = timeout.saturating_sub(started.elapsed());
        if remaining.is_zero() {
            return Err("local_canary_deadline_exhausted".to_string());
        }
        payload["max_tokens"] = json!(budget);
        let response = send(payload, remaining)?;
        if response["choices"][0]["finish_reason"].as_str() != Some("length") {
            return Ok(response);
        }
    }
    Err("local_canary_completion_truncated".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_is_bounded_and_other_errors_do_not_retry() {
        let mut payload = json!({});
        let mut calls = 0;
        let result = request_bounded(&mut payload, Duration::from_secs(2), |_, _| {
            calls += 1;
            Ok(json!({"choices":[{"finish_reason":"length"}]}))
        });
        assert_eq!(calls, 3);
        assert_eq!(result.unwrap_err(), "local_canary_completion_truncated");

        calls = 0;
        let result = request_bounded(&mut payload, Duration::from_secs(2), |_, _| {
            calls += 1;
            Err("network failure".to_string())
        });
        assert_eq!(calls, 1);
        assert_eq!(result.unwrap_err(), "network failure");
    }

    #[test]
    fn zero_timeout_fails_without_a_request() {
        let mut payload = json!({});
        let result = request_bounded(&mut payload, Duration::ZERO, |_, _| panic!("must not run"));
        assert_eq!(result.unwrap_err(), "local_canary_deadline_exhausted");
    }
}
