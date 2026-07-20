use desktoplab_tool_gateway::{TerminalRiskClass, ToolIntent, canonical_tool_from_record};
use serde_json::json;

#[test]
fn every_native_intent_has_one_canonical_identity_and_effect() {
    let cases = [
        (
            ToolIntent::filesystem_list(None),
            "desktoplab.list_files",
            false,
        ),
        (
            ToolIntent::filesystem_read("README.md"),
            "desktoplab.read_file",
            false,
        ),
        (
            ToolIntent::search_text("needle", None),
            "desktoplab.search_text",
            false,
        ),
        (
            ToolIntent::filesystem_write("notes.md"),
            "desktoplab.write_file",
            true,
        ),
        (
            ToolIntent::filesystem_patch("notes.md"),
            "desktoplab.patch_file",
            true,
        ),
        (
            ToolIntent::filesystem_create_directory("docs"),
            "desktoplab.create_directory",
            true,
        ),
        (
            ToolIntent::filesystem_move("old", "new"),
            "desktoplab.move_path",
            true,
        ),
        (
            ToolIntent::filesystem_delete("old", true),
            "desktoplab.delete_path",
            true,
        ),
        (
            ToolIntent::Terminal {
                workspace_id: None,
                working_directory: String::new(),
                command: "cargo test".to_string(),
                risk_class: TerminalRiskClass::Medium,
            },
            "desktoplab.run_terminal",
            true,
        ),
        (
            ToolIntent::ProcessStart {
                workspace_id: "workspace".to_string(),
                session_id: "session".to_string(),
                working_directory: String::new(),
                command: "npm run dev".to_string(),
            },
            "desktoplab.start_process",
            true,
        ),
        (
            ToolIntent::ProcessPoll {
                process_id: "process".to_string(),
            },
            "desktoplab.poll_process",
            false,
        ),
        (
            ToolIntent::ProcessStdin {
                process_id: "process".to_string(),
            },
            "desktoplab.write_process_stdin",
            true,
        ),
        (
            ToolIntent::ProcessKill {
                process_id: "process".to_string(),
            },
            "desktoplab.kill_process",
            true,
        ),
        (
            ToolIntent::test_run("cargo test", "verify"),
            "desktoplab.run_tests",
            true,
        ),
        (ToolIntent::git_status(), "desktoplab.git_status", false),
        (ToolIntent::git_diff(None), "desktoplab.git_diff", false),
        (
            ToolIntent::git_commit("message"),
            "desktoplab.commit_changes",
            true,
        ),
        (
            ToolIntent::git_push("origin", "main"),
            "desktoplab.push_changes",
            true,
        ),
        (
            ToolIntent::create_checkpoint("before"),
            "desktoplab.create_checkpoint",
            true,
        ),
        (
            ToolIntent::mcp_invoke("mcp.browser.open", json!({})),
            "mcp.browser.open",
            true,
        ),
        (
            ToolIntent::clarify("Which file?"),
            "desktoplab.clarify",
            false,
        ),
        (
            ToolIntent::runtime_install("ollama"),
            "desktoplab.install_runtime",
            true,
        ),
    ];

    for (intent, expected_id, expected_mutation) in cases {
        assert_eq!(intent.canonical_tool_id(), expected_id);
        assert_eq!(
            intent.has_mutating_effect(),
            expected_mutation,
            "{expected_id}"
        );
        assert_eq!(
            canonical_tool_from_record(intent.telemetry_source(), &intent.telemetry_evidence()),
            Some(expected_id.to_string()),
            "{expected_id} must survive persisted telemetry"
        );
    }
}

#[test]
fn canonical_records_preserve_router_owned_tools_without_aliases() {
    for (id, mutation) in [
        ("desktoplab.update_plan", true),
        ("desktoplab.spawn_subagent", true),
        ("desktoplab.send_subagent", true),
        ("desktoplab.get_subagent", false),
        ("desktoplab.cancel_subagent", true),
        ("desktoplab.close_subagent", true),
    ] {
        assert_eq!(
            canonical_tool_from_record("agent.iterative", id),
            Some(id.to_string())
        );
        assert_eq!(
            desktoplab_tool_gateway::canonical_tool_mutates(id),
            mutation
        );
    }
}

#[test]
fn tool_identity_source_stays_focused() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-tool-gateway/src/tool_identity.rs",
        include_str!("../src/tool_identity.rs"),
        190,
    )
    .expect("tool identity source should stay focused");
}
