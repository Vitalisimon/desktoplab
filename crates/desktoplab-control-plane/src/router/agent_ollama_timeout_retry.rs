use std::sync::atomic::{AtomicBool, Ordering};

pub(super) fn retry_inference_timeout(
    cancellation: &AtomicBool,
    mut execute: impl FnMut() -> Result<String, String>,
) -> Result<String, String> {
    for attempt in 0..2 {
        if cancellation.load(Ordering::SeqCst) {
            return Err("agent_cancelled".to_string());
        }
        let result = execute();
        if cancellation.load(Ordering::SeqCst) {
            return Err("agent_cancelled".to_string());
        }
        if attempt == 0 && matches!(&result, Err(reason) if reason == "ollama_request_timeout") {
            continue;
        }
        return result;
    }
    unreachable!("the final attempt always returns")
}

#[cfg(test)]
#[path = "agent_ollama_timeout_integration_tests.rs"]
mod integration_tests;

#[cfg(test)]
#[path = "agent_ollama_timeout_retry_tests.rs"]
mod tests;
