use std::fs;

use desktoplab_agent_engine::{
    AgentContextBuilder, AgentLoop, FirstPromptStep, LlmExecutionAdapter,
};
use desktoplab_policy::PolicyEngine;
use desktoplab_tool_gateway::ToolGateway;
use desktoplab_workspace::RepositoryInspector;
use tempfile::TempDir;
use xtask::check_logical_line_limit;

#[test]
fn first_prompt_context_includes_allowed_repository_summary() {
    let repo = fixture_repo();
    let inspection = RepositoryInspector::new(20).inspect(repo.path()).unwrap();

    let context = AgentContextBuilder::new(220)
        .with_repository_summary(inspection.summary_text(), "repository inspection")
        .build();
    let step =
        FirstPromptStep::new("session.1", "backend.ollama", "Audit the repo").with_context(context);

    let prompt = step.compiled_prompt();

    assert!(prompt.contains("Audit the repo"));
    assert!(prompt.contains("Repository context:"));
    assert!(prompt.contains("src/lib.rs"));
    assert!(prompt.contains("package.json"));
    assert!(prompt.contains("protected_files=[REDACTED]"));
}

#[test]
fn first_prompt_request_keeps_user_prompt_separate_from_compiled_context() {
    let repo = fixture_repo();
    let inspection = RepositoryInspector::new(20).inspect(repo.path()).unwrap();
    let context = AgentContextBuilder::new(220)
        .with_repository_summary(inspection.summary_text(), "repository inspection")
        .build();
    let step =
        FirstPromptStep::new("session.1", "backend.ollama", "Funzioni?").with_context(context);

    let request = step.request();

    assert_eq!(request.prompt(), Some("Funzioni?"));
    assert!(
        request
            .backend_prompt()
            .unwrap()
            .contains("Repository context:")
    );
    assert!(!request.prompt().unwrap().contains("Repository context:"));
}

#[test]
fn current_request_follows_prior_transcript_in_the_backend_prompt() {
    let context = AgentContextBuilder::new(512)
        .with_workspace_fact(
            "recent_transcript:\nuser: Patch release-note.md",
            "session-transcript:session.1",
        )
        .build();
    let step = FirstPromptStep::new(
        "session.1",
        "backend.ollama",
        "Fix calculator.js and run the tests",
    )
    .with_context(context);

    let prompt = step.compiled_prompt();

    assert!(prompt.find("Patch release-note.md") < prompt.rfind("Fix calculator.js"));
    assert!(prompt.ends_with(
        "Current user request (authoritative for this turn):\nFix calculator.js and run the tests"
    ));
}

#[test]
fn first_prompt_request_does_not_invent_readme_tool_call_without_explicit_target() {
    let step = FirstPromptStep::new("session.1", "backend.ollama", "Funzioni?");
    let mut loop_engine = AgentLoop::new(ToolGateway::new(PolicyEngine::default_conservative()))
        .with_backend_adapter(
            LlmExecutionAdapter::local("backend.ollama").with_deterministic_output("ok"),
        );

    let result = loop_engine.run(step.request());

    assert!(
        !result.event_names().contains(&"tool_decision"),
        "first prompt must not default to a README.md filesystem read"
    );
}

#[test]
fn repository_context_excludes_secrets_and_protected_paths() {
    let repo = fixture_repo();
    let inspection = RepositoryInspector::new(20).inspect(repo.path()).unwrap();

    assert!(inspection.protected_files_excluded());
    assert!(
        inspection
            .summary_paths()
            .contains(&"src/lib.rs".to_string())
    );
    assert!(!inspection.summary_text().contains("SECRET_TOKEN"));
    assert!(!inspection.summary_text().contains(".env"));
    assert!(!inspection.summary_text().contains(".git/config"));
}

#[test]
fn first_prompt_context_is_bounded() {
    let context = AgentContextBuilder::new(64)
        .with_repository_summary(
            "files=src/a.rs,src/b.rs,src/c.rs,src/d.rs",
            "repository inspection",
        )
        .with_file(
            "README.md",
            "long long long long long long long long",
            false,
        )
        .build();

    assert!(context.text().len() <= 64);
    assert!(
        context
            .provenance()
            .contains(&"repository inspection".to_string())
    );
}

#[test]
fn workspace_context_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/tests/workspace_context.rs",
        include_str!("workspace_context.rs"),
        140,
    )
    .expect("workspace context test should stay focused");
}

fn fixture_repo() -> TempDir {
    let repo = TempDir::new().unwrap();
    fs::create_dir_all(repo.path().join("src")).unwrap();
    fs::create_dir_all(repo.path().join(".git")).unwrap();
    fs::write(repo.path().join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
    fs::write(repo.path().join("package.json"), "{}\n").unwrap();
    fs::write(repo.path().join(".env"), "SECRET_TOKEN=value\n").unwrap();
    fs::write(repo.path().join(".git/config"), "[core]\n").unwrap();
    repo
}
