use std::fs;
use std::path::Path;
use std::process::Command;

use desktoplab_agent_engine::{
    IterativeToolCall, IterativeToolExecutor, ProviderToolCallNormalizer,
};
use desktoplab_control_plane::{CanonicalAgentToolExecutor, CanonicalExecutionApproval};
use serde_json::{Value, json};
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn filesystem_search_patch_terminal_and_test_tools_use_real_executors() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("README.md"), "agent before\n").unwrap();
    let mut executor = approved_executor(&repo);

    let native_read = executor
        .execute_provider_call(&json!({
            "id":"native-read",
            "function":{
                "name":"desktoplab.read_file",
                "arguments":"{\"path\":\"README.md\"}"
            }
        }))
        .expect("native provider call should execute through the canonical bridge");

    let listed = execute(&mut executor, "desktoplab.list_files", json!({}));
    let searched = execute(
        &mut executor,
        "desktoplab.search_text",
        json!({"query":"agent","path":""}),
    );
    let read = execute(
        &mut executor,
        "desktoplab.read_file",
        json!({"path":"README.md"}),
    );
    execute(
        &mut executor,
        "desktoplab.write_file",
        json!({"path":"notes.md","content":"created by executor\n"}),
    );
    execute(
        &mut executor,
        "desktoplab.patch_file",
        json!({"path":"README.md","expected":"before","replacement":"after"}),
    );
    execute(
        &mut executor,
        "desktoplab.create_directory",
        json!({"path":"docs/guides"}),
    );
    fs::write(repo.path().join("docs/guides/draft.md"), "draft\n").unwrap();
    execute(
        &mut executor,
        "desktoplab.move_path",
        json!({"source":"docs/guides/draft.md","destination":"docs/final.md"}),
    );
    execute(
        &mut executor,
        "desktoplab.delete_path",
        json!({"path":"docs/guides","recursive":false}),
    );
    let terminal = execute(
        &mut executor,
        "desktoplab.run_terminal",
        json!({"command":write_stdout("terminal-ok"),"cwd":""}),
    );
    let tests = execute(
        &mut executor,
        "desktoplab.run_tests",
        json!({"command":write_stdout("test-ok")}),
    );

    assert!(
        listed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "README.md")
    );
    assert_eq!(searched["matches"][0]["path"], "README.md");
    assert!(read["text"].as_str().unwrap().contains("agent before"));
    assert!(
        native_read.output()["text"]
            .as_str()
            .unwrap()
            .contains("agent before")
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("notes.md")).unwrap(),
        "created by executor\n"
    );
    assert!(
        fs::read_to_string(repo.path().join("README.md"))
            .unwrap()
            .contains("after")
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("docs/final.md")).unwrap(),
        "draft\n"
    );
    assert!(!repo.path().join("docs/guides").exists());
    assert_eq!(terminal["exitCode"], 0);
    assert_eq!(terminal["passed"], false);
    assert!(terminal["stdout"].as_str().unwrap().contains("terminal-ok"));
    assert_eq!(tests["passed"], true);
}

#[test]
fn file_reads_are_pageable_and_search_results_have_source_coordinates() {
    let repo = TestRepo::init();
    fs::write(
        repo.path().join("large.txt"),
        "one\ntwo\nneedle\nfour\nfive\n",
    )
    .unwrap();
    let mut executor = approved_executor(&repo);

    let page = execute(
        &mut executor,
        "desktoplab.read_file",
        json!({"path":"large.txt","offset":2,"limit":2}),
    );
    let searched = execute(
        &mut executor,
        "desktoplab.search_text",
        json!({"query":"needle"}),
    );

    assert_eq!(page["text"], "needle\nfour");
    assert_eq!(page["startLine"], 3);
    assert_eq!(page["endLine"], 4);
    assert_eq!(page["totalLines"], 5);
    assert_eq!(page["truncated"], true);
    assert_eq!(searched["matches"][0]["lineNumber"], 3);
}

#[test]
fn canonical_search_exposes_regex_and_case_sensitive_modes() {
    let repo = TestRepo::init();
    fs::write(
        repo.path().join("symbols.rs"),
        "fn AgentLoop() {}\nfn agent_loop() {}\n",
    )
    .unwrap();
    let mut executor = approved_executor(&repo);

    let exact = execute(
        &mut executor,
        "desktoplab.search_text",
        json!({"query":"AgentLoop","caseSensitive":true}),
    );
    let regex = execute(
        &mut executor,
        "desktoplab.search_text",
        json!({"query":r"fn\s+[a-z_]+","regex":true,"caseSensitive":true}),
    );

    assert_eq!(exact["matches"].as_array().unwrap().len(), 1);
    assert_eq!(regex["matches"].as_array().unwrap().len(), 1);
    assert_eq!(regex["matches"][0]["lineNumber"], 2);
}

#[test]
fn canonical_patch_requires_explicit_replace_all_for_duplicate_anchors() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("duplicates.txt"), "same\nsame\n").unwrap();
    let mut executor = approved_executor(&repo);

    let ambiguous = normalized(
        "desktoplab.patch_file",
        json!({"path":"duplicates.txt","expected":"same","replacement":"changed"}),
    );
    assert_eq!(
        executor.execute(&ambiguous),
        Err("patch_ambiguous".to_string())
    );

    execute(
        &mut executor,
        "desktoplab.patch_file",
        json!({
            "path":"duplicates.txt",
            "expected":"same",
            "replacement":"changed",
            "replaceAll":true
        }),
    );
    assert_eq!(
        fs::read_to_string(repo.path().join("duplicates.txt")).unwrap(),
        "changed\nchanged\n"
    );
}

#[test]
fn canonical_filesystem_tools_support_empty_files_and_text_deletion() {
    let repo = TestRepo::init();
    fs::write(
        repo.path().join("delete.txt"),
        "keep\nremove me\nkeep too\n",
    )
    .unwrap();
    let mut executor = approved_executor(&repo);

    execute(
        &mut executor,
        "desktoplab.write_file",
        json!({"path":"empty.txt","content":""}),
    );
    execute(
        &mut executor,
        "desktoplab.patch_file",
        json!({"path":"delete.txt","expected":"remove me\n","replacement":""}),
    );

    assert_eq!(fs::read(repo.path().join("empty.txt")).unwrap(), b"");
    assert_eq!(
        fs::read_to_string(repo.path().join("delete.txt")).unwrap(),
        "keep\nkeep too\n"
    );
}

#[test]
fn git_status_diff_checkpoint_commit_and_push_use_real_git_operations() {
    let repo = TestRepo::init();
    fs::write(repo.path().join("README.md"), "before\n").unwrap();
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "initial"]);
    let remote = TempDir::new().unwrap();
    run(Command::new("git")
        .args(["init", "--bare"])
        .current_dir(remote.path()));
    repo.git(&["remote", "add", "origin", remote.path().to_str().unwrap()]);
    fs::write(repo.path().join("README.md"), "after\n").unwrap();
    fs::write(repo.path().join("other.md"), "other\n").unwrap();
    let mut executor = approved_executor(&repo);

    let status = execute(&mut executor, "desktoplab.git_status", json!({}));
    let diff = execute(
        &mut executor,
        "desktoplab.git_diff",
        json!({"path":"README.md"}),
    );
    let committed = execute(
        &mut executor,
        "desktoplab.commit_changes",
        json!({"message":"docs: update readme"}),
    );
    let checkpoint = execute(
        &mut executor,
        "desktoplab.create_checkpoint",
        json!({"label":"after-commit"}),
    );
    let pushed = execute(
        &mut executor,
        "desktoplab.push_changes",
        json!({"remote":"origin","branch":"HEAD:refs/heads/main"}),
    );

    assert!(status["entries"].as_array().unwrap().len() >= 2);
    assert!(diff["diff"].as_str().unwrap().contains("README.md"));
    assert!(!diff["diff"].as_str().unwrap().contains("other.md"));
    assert_eq!(committed["status"], "committed");
    assert!(
        checkpoint["ref"]
            .as_str()
            .unwrap()
            .contains("desktoplab/savepoints")
    );
    assert_eq!(pushed["status"], "pushed");
    assert!(
        repo.git_output(&["status", "--porcelain"])
            .trim()
            .is_empty()
    );
}

#[test]
fn mutations_do_not_bypass_approval() {
    let repo = TestRepo::init();
    let mut pending = CanonicalAgentToolExecutor::new(
        repo.path(),
        "workspace-test",
        "session-test",
        CanonicalExecutionApproval::Pending,
    );
    let write = normalized(
        "desktoplab.write_file",
        json!({"path":"blocked.md","content":"no"}),
    );

    assert_eq!(
        pending.execute(&write),
        Err("approval_required".to_string())
    );
    assert!(!repo.path().join("blocked.md").exists());
    pending
        .execute_approved(&write)
        .expect("approved canonical execution should apply once");
    assert_eq!(
        fs::read_to_string(repo.path().join("blocked.md")).unwrap(),
        "no"
    );
}

#[test]
fn nonzero_terminal_and_test_results_keep_output_but_are_failed_observations() {
    let repo = TestRepo::init();
    let mut executor = approved_executor(&repo);
    for (tool, expected_error) in [
        ("desktoplab.run_terminal", "command_exit_nonzero:7"),
        ("desktoplab.run_tests", "tests_failed:7"),
    ] {
        let call = normalized(tool, json!({"command":failing_command()}));
        let observation = executor.execute(&call).unwrap();

        assert_eq!(observation.error(), Some(expected_error));
        assert_eq!(observation.output()["exitCode"], 7);
        assert!(
            observation.output()["stderr"]
                .as_str()
                .unwrap()
                .contains("failure-proof")
        );
        assert_eq!(observation.provenance().exit_code(), Some(7));
    }
}

#[test]
fn detected_test_run_through_terminal_produces_passing_test_evidence() {
    let repo = TestRepo::init();
    fs::write(
        repo.path().join("package.json"),
        r#"{"scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    fs::write(repo.path().join("package-lock.json"), "{}\n").unwrap();
    let mut executor = approved_executor(&repo);

    let observation = executor
        .execute(&normalized(
            "desktoplab.run_terminal",
            json!({"command":"npm test"}),
        ))
        .unwrap();

    assert_eq!(observation.output()["passed"], true);
    assert!(observation.is_passing_test_evidence());
}

#[test]
fn terminal_timeout_is_model_selectable_and_bounded() {
    let repo = TestRepo::init();
    let mut executor = approved_executor(&repo);
    let call = normalized(
        "desktoplab.run_terminal",
        json!({"command":timeout_command(),"timeoutSeconds":1}),
    );

    let observation = executor.execute(&call).unwrap();

    assert_eq!(observation.error(), Some("command_timed_out"));
    assert_eq!(observation.output()["status"], "timed_out");
}

#[test]
fn canonical_executor_sources_stay_below_line_guards() {
    for (path, source) in [
        (
            "crates/desktoplab-control-plane/src/canonical_tool_executor.rs",
            include_str!("../src/canonical_tool_executor.rs"),
        ),
        (
            "crates/desktoplab-control-plane/src/canonical_tool_files.rs",
            include_str!("../src/canonical_tool_files.rs"),
        ),
        (
            "crates/desktoplab-control-plane/src/canonical_tool_process.rs",
            include_str!("../src/canonical_tool_process.rs"),
        ),
        (
            "crates/desktoplab-control-plane/src/canonical_tool_search.rs",
            include_str!("../src/canonical_tool_search.rs"),
        ),
        (
            "crates/desktoplab-control-plane/src/canonical_tool_git.rs",
            include_str!("../src/canonical_tool_git.rs"),
        ),
    ] {
        check_logical_line_limit(path, source, 250)
            .expect("canonical executor source grew too large");
    }
}

#[cfg(not(target_os = "windows"))]
fn failing_command() -> &'static str {
    "printf failure-proof >&2; exit 7"
}

#[cfg(not(target_os = "windows"))]
fn timeout_command() -> &'static str {
    "sleep 2"
}

#[cfg(target_os = "windows")]
fn timeout_command() -> &'static str {
    "Start-Sleep -Seconds 2"
}

#[cfg(target_os = "windows")]
fn failing_command() -> &'static str {
    "Write-Error failure-proof; exit 7"
}

fn approved_executor(repo: &TestRepo) -> CanonicalAgentToolExecutor {
    CanonicalAgentToolExecutor::new(
        repo.path(),
        "workspace-test",
        "session-test",
        CanonicalExecutionApproval::Approved,
    )
}

#[cfg(not(windows))]
fn write_stdout(value: &str) -> String {
    format!("printf '{value}'")
}

#[cfg(windows)]
fn write_stdout(value: &str) -> String {
    format!("[Console]::Write('{value}')")
}

fn execute(executor: &mut CanonicalAgentToolExecutor, name: &str, arguments: Value) -> Value {
    executor
        .execute(&normalized(name, arguments))
        .unwrap_or_else(|error| panic!("{name} failed: {error}"))
        .output()
        .clone()
}

fn normalized(name: &str, arguments: Value) -> IterativeToolCall {
    ProviderToolCallNormalizer::default()
        .normalize(format!("call-{name}"), name, arguments)
        .expect("test tool call should normalize")
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
        run(Command::new("git").args(args).current_dir(self.path()));
    }

    fn git_output(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }
}

fn run(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
