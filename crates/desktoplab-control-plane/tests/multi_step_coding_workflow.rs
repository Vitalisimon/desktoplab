use std::fs;
use std::path::Path;
use std::process::Command;

use desktoplab_agent_engine::{
    IterativeAgentLoop, IterativeApproval, IterativeLoopState, IterativeLoopStatus,
    IterativeModelAdapter, IterativeModelDecision, IterativeToolCall,
};
use desktoplab_control_plane::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};
use serde_json::json;
use tempfile::TempDir;

struct RepairingCodingModel;

impl IterativeModelAdapter for RepairingCodingModel {
    fn decide(&mut self, state: &IterativeLoopState) -> Result<IterativeModelDecision, String> {
        let observations = state.observations();
        let decision = match observations.len() {
            0 => tool(
                "read-instructions",
                "desktoplab.read_file",
                json!({"path":"AGENTS.md"}),
            ),
            1 => tool(
                "read-source",
                "desktoplab.read_file",
                json!({"path":"src/app.txt"}),
            ),
            2 => tool(
                "first-patch",
                "desktoplab.patch_file",
                json!({"path":"src/app.txt","expected":"broken","replacement":"still-broken"}),
            ),
            3 => tool(
                "failing-test",
                "desktoplab.run_tests",
                json!({"command":fixed_content_test_command()}),
            ),
            4 => tool(
                "repair-patch",
                "desktoplab.patch_file",
                json!({"path":"src/app.txt","expected":"still-broken","replacement":"fixed"}),
            ),
            5 => tool(
                "passing-test",
                "desktoplab.run_tests",
                json!({"command":fixed_content_test_command()}),
            ),
            6 => tool(
                "review-diff",
                "desktoplab.git_diff",
                json!({"path":"src/app.txt"}),
            ),
            _ => IterativeModelDecision::final_response(
                "Updated src/app.txt, repaired the failed validation, and all tests passed.",
            ),
        };
        Ok(decision)
    }
}

#[test]
fn one_prompt_completes_real_inspect_patch_fail_repair_pass_diff_workflow() {
    let repo = TestRepo::init();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::write(
        repo.path().join("AGENTS.md"),
        "Inspect, edit, test, and report.\n",
    )
    .unwrap();
    fs::write(repo.path().join("src/app.txt"), "broken").unwrap();
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "fixture"]);
    let mut state = IterativeLoopState::new("one-user-prompt");
    let mut model = RepairingCodingModel;
    let mut executor = CanonicalAgentToolExecutor::new(
        repo.path(),
        "workspace-fixture",
        "one-user-prompt",
        CanonicalExecutionApproval::Pending,
    );
    let agent_loop = IterativeAgentLoop::default();

    agent_loop.run(&mut state, &mut model, &mut executor);
    while state.status() == IterativeLoopStatus::WaitingForApproval {
        let pending = state.pending_approval().unwrap();
        let approval =
            IterativeApproval::approved(pending.call_id(), pending.payload_fingerprint());
        agent_loop.resume_with_approval(&mut state, approval, &mut model, &mut executor);
    }

    assert_eq!(state.status(), IterativeLoopStatus::Completed);
    assert_eq!(
        fs::read_to_string(repo.path().join("src/app.txt")).unwrap(),
        "fixed"
    );
    assert_eq!(state.model_turns(), 8);
    assert_eq!(
        state
            .observations()
            .iter()
            .map(|observation| observation.tool_name())
            .collect::<Vec<_>>(),
        [
            "desktoplab.read_file",
            "desktoplab.read_file",
            "desktoplab.patch_file",
            "desktoplab.run_tests",
            "desktoplab.patch_file",
            "desktoplab.run_tests",
            "desktoplab.git_diff",
        ]
    );
    assert_eq!(state.observations()[3].output()["passed"], false);
    assert_eq!(state.observations()[5].output()["passed"], true);
    assert!(
        state.observations()[6].output()["diff"]
            .as_str()
            .unwrap()
            .contains("fixed")
    );
}

fn tool(id: &str, name: &str, arguments: serde_json::Value) -> IterativeModelDecision {
    IterativeModelDecision::tool_call(IterativeToolCall::new(id, name, arguments))
}

#[cfg(not(windows))]
fn fixed_content_test_command() -> &'static str {
    "test \"$(cat src/app.txt)\" = fixed"
}

#[cfg(windows)]
fn fixed_content_test_command() -> &'static str {
    "if ((Get-Content -Raw -LiteralPath 'src/app.txt') -ceq 'fixed') { exit 0 } else { exit 1 }"
}

struct TestRepo(TempDir);

impl TestRepo {
    fn init() -> Self {
        let repo = Self(TempDir::new().unwrap());
        repo.git(&["init"]);
        repo.git(&["config", "user.email", "desktoplab@example.invalid"]);
        repo.git(&["config", "user.name", "DesktopLab Test"]);
        repo
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    fn git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
