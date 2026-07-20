use desktoplab_policy::{Action, DecisionOutcome, PolicyEngine};
use desktoplab_tool_gateway::{ToolGateway, ToolIntent, ToolOutcome};
use xtask::check_logical_line_limit;

#[test]
fn filesystem_write_requires_approval_by_default() {
    let outcome = ToolGateway::new(PolicyEngine::default_conservative())
        .authorize(ToolIntent::filesystem_write("src/lib.rs"));

    assert_eq!(
        outcome,
        ToolOutcome::ApprovalRequired(Action::FilesystemWrite)
    );
}

#[test]
fn terminal_execution_requires_approval_by_default() {
    let outcome = ToolGateway::new(PolicyEngine::default_conservative())
        .authorize(ToolIntent::terminal("cargo test"));

    assert_eq!(
        outcome,
        ToolOutcome::ApprovalRequired(Action::TerminalCommand)
    );
}

#[test]
fn dependency_installs_and_lockfile_writes_are_high_risk_tool_actions() {
    let mut gateway = ToolGateway::new(
        PolicyEngine::default_conservative()
            .with_approval_mode(desktoplab_policy::ApprovalMode::ApproveForMe),
    );

    assert_eq!(
        gateway.authorize(ToolIntent::terminal("npm install left-pad")),
        ToolOutcome::ApprovalRequired(Action::DependencyInstall)
    );
    assert_eq!(
        gateway.authorize(ToolIntent::filesystem_write("package-lock.json")),
        ToolOutcome::ApprovalRequired(Action::GeneratedArtifactWrite)
    );
}

#[test]
fn runtime_install_execution_is_allowed_and_audited_after_setup_acceptance() {
    let mut gateway = ToolGateway::new(PolicyEngine::default_conservative());

    let outcome = gateway.authorize(ToolIntent::runtime_install("runtime.ollama"));

    assert_eq!(outcome, ToolOutcome::Allowed(Action::RuntimeInstall));
    assert_eq!(gateway.approval_requests().len(), 0);
    assert_eq!(gateway.audit_records().len(), 1);
    assert_eq!(
        gateway.audit_records()[0].decision().outcome(),
        DecisionOutcome::AllowedAutomatic
    );
}

#[test]
fn git_commit_and_push_require_approval_by_default() {
    let mut gateway = ToolGateway::new(PolicyEngine::default_conservative());

    assert_eq!(
        gateway.authorize(ToolIntent::git_commit("feat: change")),
        ToolOutcome::ApprovalRequired(Action::GitCommit)
    );
    assert_eq!(
        gateway.authorize(ToolIntent::git_push("origin", "main")),
        ToolOutcome::ApprovalRequired(Action::GitPush)
    );
}

#[test]
fn local_only_paths_are_protected_before_policy_approval() {
    let outcome = ToolGateway::new(PolicyEngine::default_conservative())
        .authorize(ToolIntent::filesystem_read(".env"));

    assert_eq!(outcome, ToolOutcome::Blocked("local_only_path".to_string()));
}

#[test]
fn protected_path_policy_normalizes_windows_separators_and_case() {
    let mut gateway = ToolGateway::new(PolicyEngine::default_conservative());

    for path in [
        r"src\.git\config",
        r"config\.ENV.local",
        r"nested\.ssh\id_ed25519",
        r"certs\release.PEM",
    ] {
        assert_eq!(
            gateway.authorize(ToolIntent::filesystem_read(path)),
            ToolOutcome::Blocked("local_only_path".to_string()),
            "{path} must remain protected"
        );
    }
    assert!(matches!(
        gateway.authorize(ToolIntent::filesystem_read(".gitignore")),
        ToolOutcome::Allowed(_)
    ));
}

#[test]
fn approval_requests_and_audit_records_are_created_for_sensitive_tools() {
    let mut gateway = ToolGateway::new(PolicyEngine::default_conservative());

    let outcome = gateway.authorize(ToolIntent::filesystem_write("README.md"));

    assert!(matches!(outcome, ToolOutcome::ApprovalRequired(_)));
    assert_eq!(gateway.approval_requests().len(), 1);
    assert_eq!(gateway.audit_records().len(), 1);
    assert_eq!(
        gateway.audit_records()[0].decision().outcome(),
        DecisionOutcome::RequiresApproval
    );
}

#[test]
fn every_read_control_and_coordination_intent_is_audited_with_its_real_action() {
    let mut gateway = ToolGateway::new(PolicyEngine::default_conservative());
    let cases = [
        (
            ToolIntent::filesystem_read("README.md"),
            Action::FilesystemRead,
        ),
        (
            ToolIntent::process_poll("process.1"),
            Action::ProcessControl,
        ),
        (ToolIntent::git_status(), Action::GitRead),
        (
            ToolIntent::create_checkpoint("before"),
            Action::CheckpointCreate,
        ),
        (
            ToolIntent::mcp_invoke("mcp.docs.search", serde_json::json!({})),
            Action::McpInvoke,
        ),
        (ToolIntent::clarify("Which target?"), Action::Clarification),
    ];
    let case_count = cases.len();

    for (intent, action) in cases {
        assert_eq!(gateway.authorize(intent), ToolOutcome::Allowed(action));
        assert_eq!(
            gateway.audit_records().last().unwrap().decision().action(),
            action
        );
    }
    assert_eq!(gateway.audit_records().len(), case_count);
}

#[test]
fn tool_gateway_source_files_stay_below_initial_line_count_guard() {
    for (path, source, max_lines) in [
        (
            "crates/desktoplab-tool-gateway/src/lib.rs",
            include_str!("../src/lib.rs"),
            250,
        ),
        (
            "crates/desktoplab-tool-gateway/src/gateway.rs",
            include_str!("../src/gateway.rs"),
            250,
        ),
        (
            "crates/desktoplab-tool-gateway/src/terminal_classification.rs",
            include_str!("../src/terminal_classification.rs"),
            250,
        ),
        (
            "crates/desktoplab-tool-gateway/src/intent.rs",
            include_str!("../src/intent.rs"),
            250,
        ),
        (
            "crates/desktoplab-tool-gateway/src/intent_execution.rs",
            include_str!("../src/intent_execution.rs"),
            150,
        ),
    ] {
        check_logical_line_limit(path, source, max_lines)
            .expect("tool gateway source should stay below the initial line-count guard");
    }
}
