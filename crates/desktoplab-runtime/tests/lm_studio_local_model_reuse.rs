use std::sync::Mutex;

use desktoplab_runtime::{
    LmStudioManagedPlan, LmStudioModelProbe, ProcessCommand, ProcessOutput, ProcessRunner,
    RuntimeEndpointError, RuntimeExecutionState, execute_managed_lm_studio,
};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

const DIGEST: &str = "65d06d31a3977d553cb3af137b5c26b5f1e9297a6aaa29ae7caa98788cde53ab";
const MODEL_PATH: &str = "example/gpt-oss-20b/model.gguf";

struct Runner {
    commands: Mutex<Vec<ProcessCommand>>,
    archive_digest: String,
    inventory: String,
    model_digest: String,
}

impl ProcessRunner for Runner {
    fn run(&self, command: ProcessCommand) -> ProcessOutput {
        self.commands.lock().unwrap().push(command.clone());
        let args = command.args();
        let output = if command.program() == "shasum" && args.first().is_some_and(|a| a == "-a") {
            if args.get(1).is_some_and(|a| a == "512") {
                format!("{}  archive", self.archive_digest)
            } else {
                format!("{}  model", self.model_digest)
            }
        } else if args.first().is_some_and(|a| a == "daemon") {
            r#"{"status":"running","pid":42,"isDaemon":true}"#.to_string()
        } else if args.first().is_some_and(|a| a == "ls") {
            self.inventory.clone()
        } else {
            String::new()
        };
        ProcessOutput::new(Some(0), output, "", command)
    }
}

struct Probe;
impl LmStudioModelProbe for Probe {
    fn models(&self, _: &str) -> Result<Vec<String>, RuntimeEndpointError> {
        Ok(vec!["desktoplab-gpt-oss-20b".to_string()])
    }
}

#[test]
fn verified_local_model_is_loaded_without_a_download() {
    let inventory = format!(
        r#"[{{"type":"llm","modelKey":"gpt-oss-20b","format":"gguf","path":"{MODEL_PATH}"}}]"#
    );
    let (_temp, plan, runner) = scenario(&inventory, DIGEST);

    let result = execute_managed_lm_studio(&runner, &Probe, &plan);
    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    let commands = runner.commands.lock().unwrap();
    assert!(commands.iter().any(|c| c.args() == ["ls", "--json"]));
    assert!(
        !commands
            .iter()
            .any(|c| c.args().first().is_some_and(|a| a == "get"))
    );
    assert!(
        commands
            .iter()
            .any(|c| c.args().first().is_some_and(|a| a == "load")
                && c.args().get(1).is_some_and(|a| a == "gpt-oss-20b"))
    );
}

#[test]
fn absent_local_model_uses_the_existing_download_path() {
    let (temp, plan, runner) = scenario("[]", DIGEST);
    let result = execute_managed_lm_studio(&runner, &Probe, &plan);
    assert_eq!(result.state(), RuntimeExecutionState::Completed);
    assert!(
        runner
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.args().first().is_some_and(|a| a == "get"))
    );
    drop(temp);
}

#[test]
fn inventory_larger_than_public_evidence_limit_can_still_be_verified() {
    let padding = "x".repeat(1000);
    let inventory = format!(
        r#"[{{"type":"llm","modelKey":"gpt-oss-20b","format":"gguf","path":"{MODEL_PATH}","description":"{padding}"}}]"#
    );
    let (temp, plan, runner) = scenario(&inventory, DIGEST);
    assert_eq!(
        execute_managed_lm_studio(&runner, &Probe, &plan).state(),
        RuntimeExecutionState::Completed
    );
    assert!(
        !runner
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.args().first().is_some_and(|a| a == "get"))
    );
    drop(temp);
}

#[test]
fn invalid_untrusted_or_ambiguous_inventory_fails_closed() {
    let valid = format!(
        r#"{{"type":"llm","modelKey":"gpt-oss-20b","format":"gguf","path":"{MODEL_PATH}"}}"#
    );
    for (inventory, digest) in [
        ("not json".to_string(), DIGEST),
        (format!("[{valid},{valid}]"), DIGEST),
        (format!("[{valid}]"), "wrong-digest"),
        (r#"[{"type":"llm","modelKey":"gpt-oss-20b","format":"gguf","path":"../outside.gguf"}]"#.to_string(), DIGEST),
        (r#"[{"type":"llm","modelKey":"gpt-oss-20b","format":"mlx","path":"example/gpt-oss-20b/model.gguf"}]"#.to_string(), DIGEST),
    ] {
        let (temp, plan, runner) = scenario(&inventory, digest);
        assert_eq!(execute_managed_lm_studio(&runner, &Probe, &plan).state(), RuntimeExecutionState::Failed);
        assert!(!runner.commands.lock().unwrap().iter().any(|c| c.args().first().is_some_and(|a| a == "get" || a == "load")));
        drop(temp);
    }
}

#[cfg(unix)]
#[test]
fn model_symlink_outside_managed_root_is_rejected() {
    use std::os::unix::fs::symlink;
    let inventory = format!(
        r#"[{{"type":"llm","modelKey":"gpt-oss-20b","format":"gguf","path":"{MODEL_PATH}"}}]"#
    );
    let (temp, plan, runner) = scenario(&inventory, DIGEST);
    let model = plan
        .managed_home()
        .join(".lmstudio/models")
        .join(MODEL_PATH);
    std::fs::remove_file(&model).unwrap();
    let outside = temp.path().join("outside.gguf");
    std::fs::write(&outside, b"outside").unwrap();
    symlink(outside, model).unwrap();
    assert_eq!(
        execute_managed_lm_studio(&runner, &Probe, &plan).state(),
        RuntimeExecutionState::Failed
    );
    assert!(
        !runner
            .commands
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.args().first().is_some_and(|a| a == "load"))
    );
}

fn scenario(inventory: &str, digest: &str) -> (TempDir, LmStudioManagedPlan, Runner) {
    let temp = TempDir::new().unwrap();
    let plan = LmStudioManagedPlan::new(
        temp.path().join("DesktopLab/runtimes/lm-studio"),
        "darwin-arm64",
        "openai/gpt-oss-20b",
        "desktoplab-gpt-oss-20b",
        12345,
        true,
    )
    .unwrap();
    let model = plan
        .managed_home()
        .join(".lmstudio/models")
        .join(MODEL_PATH);
    std::fs::create_dir_all(model.parent().unwrap()).unwrap();
    std::fs::write(model, b"mocked digest").unwrap();
    let runner = Runner {
        commands: Mutex::new(Vec::new()),
        archive_digest: plan.artifact().sha512().to_string(),
        inventory: inventory.to_string(),
        model_digest: digest.to_string(),
    };
    (temp, plan, runner)
}

#[test]
fn reuse_tests_stay_focused() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/tests/lm_studio_local_model_reuse.rs",
        include_str!("lm_studio_local_model_reuse.rs"),
        200,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/lm_studio_local_model.rs",
        include_str!("../src/lm_studio_local_model.rs"),
        160,
    )
    .unwrap();
}
