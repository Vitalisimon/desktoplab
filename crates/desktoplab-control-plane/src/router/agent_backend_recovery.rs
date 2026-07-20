use std::time::Duration;

pub(super) const LOCAL_BACKEND_TRANSPORT_ATTEMPTS: usize = 2;
#[cfg(debug_assertions)]
pub(super) const INITIAL_PROTOCOL_RECOVERY_ATTEMPTS: usize = 2;

#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InitialBackendRecoveryState {
    NotNeeded,
    Recovered,
    Exhausted,
}

#[cfg(debug_assertions)]
impl InitialBackendRecoveryState {
    pub(super) fn used(self) -> bool {
        self != Self::NotNeeded
    }

    pub(super) fn exhausted(self) -> bool {
        self == Self::Exhausted
    }
}

#[cfg(debug_assertions)]
pub(super) struct InitialBackendOutput {
    pub(super) output: String,
    pub(super) recovery: InitialBackendRecoveryState,
}

pub(super) fn retry_backend_transport(
    max_attempts: usize,
    delay: Duration,
    mut execute: impl FnMut() -> Result<String, ()>,
) -> Result<String, ()> {
    let max_attempts = max_attempts.max(1);
    for attempt in 0..max_attempts {
        match execute() {
            Ok(output) => return Ok(output),
            Err(()) if attempt + 1 < max_attempts => {
                if !delay.is_zero() {
                    std::thread::sleep(delay);
                }
            }
            Err(()) => return Err(()),
        }
    }
    Err(())
}

#[cfg(debug_assertions)]
pub(super) fn recover_initial_backend_output(
    initial_output: String,
    user_goal: &str,
    should_retry: impl Fn(&str) -> bool,
    recovery_prompt: impl Fn(&str) -> String,
    mut execute: impl FnMut(&str) -> Result<String, ()>,
) -> Result<InitialBackendOutput, ()> {
    let mut output = initial_output;
    let mut recovery = InitialBackendRecoveryState::NotNeeded;
    for _ in 0..INITIAL_PROTOCOL_RECOVERY_ATTEMPTS {
        if !should_retry(&output) {
            return Ok(InitialBackendOutput { output, recovery });
        }
        recovery = InitialBackendRecoveryState::Recovered;
        output = execute(&recovery_prompt(user_goal))?;
    }
    if should_retry(&output) {
        recovery = InitialBackendRecoveryState::Exhausted;
    }
    Ok(InitialBackendOutput { output, recovery })
}

#[cfg(test)]
mod tests {
    use super::{
        InitialBackendRecoveryState, recover_initial_backend_output, retry_backend_transport,
    };
    use std::collections::VecDeque;
    use std::time::Duration;
    use xtask::check_logical_line_limit;

    #[test]
    fn local_transport_retries_one_transient_failure() {
        let mut attempts = 0;
        let output = retry_backend_transport(2, Duration::ZERO, || {
            attempts += 1;
            (attempts == 2).then(|| "ready".to_string()).ok_or(())
        })
        .expect("second transport attempt should succeed");

        assert_eq!(output, "ready");
        assert_eq!(attempts, 2);
    }

    #[test]
    fn protocol_recovery_replans_until_a_valid_tool_is_returned() {
        let mut outputs = VecDeque::from(["clarify_without_blocked_on", "read_file"]);
        let recovered = recover_initial_backend_output(
            "malformed".to_string(),
            "Read README.md",
            |output| output != "read_file",
            |goal| format!("recover:{goal}:blockedOn"),
            |prompt| {
                assert!(prompt.contains("blockedOn"));
                outputs.pop_front().map(str::to_string).ok_or(())
            },
        )
        .expect("bounded protocol recovery should succeed");

        assert_eq!(recovered.recovery, InitialBackendRecoveryState::Recovered);
        assert_eq!(recovered.output, "read_file");
    }

    #[test]
    fn protocol_recovery_fails_closed_after_its_bounded_attempts() {
        let mut attempts = 0;
        let recovered = recover_initial_backend_output(
            "malformed".to_string(),
            "Read README.md",
            |_| true,
            |goal| goal.to_string(),
            |_| {
                attempts += 1;
                Ok("still_malformed".to_string())
            },
        )
        .expect("protocol failure should remain a modeled result");

        assert_eq!(recovered.recovery, InitialBackendRecoveryState::Exhausted);
        assert_eq!(attempts, 2);
    }

    #[test]
    fn backend_recovery_source_stays_below_line_guard() {
        check_logical_line_limit(
            "crates/desktoplab-control-plane/src/router/agent_backend_recovery.rs",
            include_str!("agent_backend_recovery.rs"),
            180,
        )
        .expect("backend recovery source should stay focused");
    }
}
