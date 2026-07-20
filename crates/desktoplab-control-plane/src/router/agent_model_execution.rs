use std::sync::atomic::{AtomicBool, Ordering};

use desktoplab_agent_engine::{IterativeAgentLoop, IterativeLoopState};
use desktoplab_backends::{BackendPrompt, OpenAiCodexResponderCommandPayload};
use desktoplab_runtime::{ProcessCommand, ProcessRunner, SystemProcessRunner};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum AgentModelExecutionError {
    Cancelled,
    Protocol(String),
    Runtime(String),
}

impl AgentModelExecutionError {
    pub(super) fn from_backend(reason: String) -> Self {
        if reason == "agent_cancelled" {
            Self::Cancelled
        } else if is_backend_protocol_error(&reason) {
            Self::Protocol(reason)
        } else {
            Self::Runtime(reason)
        }
    }

    pub(super) fn runtime(reason: impl Into<String>) -> Self {
        Self::Runtime(reason.into())
    }
}

impl From<String> for AgentModelExecutionError {
    fn from(reason: String) -> Self {
        Self::from_backend(reason)
    }
}

fn is_backend_protocol_error(reason: &str) -> bool {
    reason.starts_with("provider_")
        || reason.starts_with("openai_compatible_stream_tool_call")
        || reason == "parallel_tool_calls_unsupported"
}

pub(super) fn apply_model_execution_error(
    state: &mut IterativeLoopState,
    agent_loop: &IterativeAgentLoop,
    error: AgentModelExecutionError,
) {
    match error {
        AgentModelExecutionError::Cancelled => state.cancel("agent_cancelled"),
        AgentModelExecutionError::Protocol(reason) => {
            if !state.request_model_protocol_retry(reason.clone()) {
                agent_loop.fail_model_turn(state, format!("model_protocol_error:{reason}"));
            }
        }
        AgentModelExecutionError::Runtime(reason) => agent_loop.fail_model_turn(state, reason),
    }
}

pub(super) enum PreparedAgentModelExecution {
    #[cfg(debug_assertions)]
    Fixture {
        output: String,
        delay: std::time::Duration,
    },
    Ollama {
        resolver: desktoplab_backends::OllamaModelCapabilityResolver,
        expected: desktoplab_backends::BackendModelCapabilities,
        prompt: BackendPrompt,
    },
    LmStudio {
        backend: desktoplab_backends::LmStudioExecutionBackend,
        prompt: BackendPrompt,
    },
    HighEnd {
        backend: desktoplab_backends::OpenAiCompatibleLocalExecutionBackend,
        prompt: BackendPrompt,
    },
    Codex {
        responder_url: String,
        payload: OpenAiCodexResponderCommandPayload,
    },
    Mlx {
        command: ProcessCommand,
    },
    Failed(String),
}

impl PreparedAgentModelExecution {
    pub(super) fn execute(
        self,
        cancellation: &AtomicBool,
        streaming: bool,
        on_delta: &mut impl FnMut(&str),
    ) -> Result<String, AgentModelExecutionError> {
        if cancellation.load(Ordering::SeqCst) {
            return Err(AgentModelExecutionError::Cancelled);
        }
        match self {
            #[cfg(debug_assertions)]
            Self::Fixture { output, delay } => execute_fixture(output, delay, cancellation),
            Self::Ollama {
                resolver,
                expected,
                prompt,
            } => execute_ollama(
                resolver,
                expected,
                prompt,
                cancellation,
                streaming,
                on_delta,
            ),
            Self::LmStudio { backend, prompt } => if streaming {
                backend.execute_chat_stream(&prompt, cancellation, on_delta)
            } else {
                backend.execute_chat(&prompt)
            }
            .map_err(AgentModelExecutionError::from_backend),
            Self::HighEnd { backend, prompt } => if streaming {
                backend.execute_chat_stream(&prompt, cancellation, on_delta)
            } else {
                backend.execute_chat(&prompt)
            }
            .map_err(AgentModelExecutionError::from_backend),
            Self::Codex {
                responder_url,
                payload,
            } => {
                let output = desktoplab_backends::execute_openai_codex_responder_command(
                    &responder_url,
                    &payload,
                )
                .map_err(AgentModelExecutionError::runtime)?;
                Ok(output.body().to_string())
            }
            Self::Mlx { command } => {
                let output =
                    <SystemProcessRunner as ProcessRunner>::run(&SystemProcessRunner, command);
                if cancellation.load(Ordering::SeqCst) {
                    return Err(AgentModelExecutionError::Cancelled);
                }
                if !output.succeeded() {
                    return Err(AgentModelExecutionError::runtime(
                        "mlx_lm_generation_failed",
                    ));
                }
                let response = output.stdout().trim();
                if response.is_empty() {
                    return Err(AgentModelExecutionError::runtime("mlx_lm_empty_response"));
                }
                Ok(response.to_string())
            }
            Self::Failed(reason) => Err(AgentModelExecutionError::Runtime(reason)),
        }
    }
}

#[cfg(debug_assertions)]
fn execute_fixture(
    output: String,
    delay: std::time::Duration,
    cancellation: &AtomicBool,
) -> Result<String, AgentModelExecutionError> {
    let started = std::time::Instant::now();
    while started.elapsed() < delay {
        if cancellation.load(Ordering::SeqCst) {
            return Err(AgentModelExecutionError::Cancelled);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Ok(output)
}

fn execute_ollama(
    resolver: desktoplab_backends::OllamaModelCapabilityResolver,
    expected: desktoplab_backends::BackendModelCapabilities,
    prompt: BackendPrompt,
    cancellation: &AtomicBool,
    streaming: bool,
    on_delta: &mut impl FnMut(&str),
) -> Result<String, AgentModelExecutionError> {
    let endpoint = "http://127.0.0.1:11434";
    let resolved = resolver
        .resolve(endpoint, prompt.model())
        .map_err(AgentModelExecutionError::runtime)?;
    if resolved.fingerprint() != expected.fingerprint() {
        return Err(AgentModelExecutionError::runtime(
            "ollama_model_fingerprint_changed",
        ));
    }
    let certification = expected
        .tool_protocol_certification()
        .cloned()
        .ok_or_else(|| AgentModelExecutionError::runtime("model_tool_protocol_uncertified"))?;
    let model = prompt.model();
    let backend = desktoplab_backends::OllamaExecutionBackend::new(
        desktoplab_backends::BackendModelInventory::available(&[model]),
    )
    .with_model_capabilities([resolved.with_tool_protocol_certification(certification)]);
    if streaming {
        backend
            .execute_chat_stream(endpoint, &prompt, cancellation, on_delta)
            .map_err(AgentModelExecutionError::from_backend)
    } else {
        backend
            .execute_chat(endpoint, &prompt)
            .map_err(AgentModelExecutionError::from_backend)
    }
}

#[cfg(test)]
mod tests {
    use desktoplab_agent_engine::{
        IterativeAgentLoop, IterativeLoopState, IterativeLoopStatus, IterativeStopReason,
    };

    use super::{AgentModelExecutionError, apply_model_execution_error};

    #[test]
    fn backend_response_protocol_errors_are_typed_separately_from_runtime_failures() {
        assert!(matches!(
            AgentModelExecutionError::from_backend(
                "provider_constrained_tool_missing_name".to_string()
            ),
            AgentModelExecutionError::Protocol(_)
        ));
        assert!(matches!(
            AgentModelExecutionError::from_backend("ollama_request_failed:offline".to_string()),
            AgentModelExecutionError::Runtime(_)
        ));
        assert_eq!(
            AgentModelExecutionError::from_backend("agent_cancelled".to_string()),
            AgentModelExecutionError::Cancelled
        );
    }

    #[test]
    fn protocol_errors_retry_but_runtime_and_cancellation_stop_immediately() {
        let agent_loop = IterativeAgentLoop::default();
        let mut protocol = IterativeLoopState::new("session.protocol");
        apply_model_execution_error(
            &mut protocol,
            &agent_loop,
            AgentModelExecutionError::Protocol("provider_tool_call_missing_name".to_string()),
        );
        assert_eq!(protocol.status(), IterativeLoopStatus::Running);
        assert_eq!(
            protocol.model_protocol_recovery(),
            Some("provider_tool_call_missing_name")
        );

        let mut runtime = IterativeLoopState::new("session.runtime");
        apply_model_execution_error(
            &mut runtime,
            &agent_loop,
            AgentModelExecutionError::Runtime("ollama_request_failed:offline".to_string()),
        );
        assert_eq!(runtime.status(), IterativeLoopStatus::Failed);
        assert!(matches!(
            runtime.stop_reason(),
            Some(IterativeStopReason::ModelFailure(reason))
                if reason == "ollama_request_failed:offline"
        ));

        let mut cancelled = IterativeLoopState::new("session.cancelled");
        apply_model_execution_error(
            &mut cancelled,
            &agent_loop,
            AgentModelExecutionError::Cancelled,
        );
        assert_eq!(cancelled.status(), IterativeLoopStatus::Cancelled);
    }
}
