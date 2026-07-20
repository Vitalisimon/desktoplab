use desktoplab_tool_gateway::{TestCommandSelection, TestRunnerExecutor};
use desktoplab_workspace::TestCommandConfidence;
use tempfile::TempDir;

#[test]
fn selects_single_high_confidence_project_test_command_with_reason() {
    for fixture in [
        (
            "package.json",
            r#"{"scripts":{"test":"vitest"}}"#,
            "npm test",
        ),
        ("Cargo.toml", "[package]\nname='demo'\n", "cargo test"),
        ("go.mod", "module example.test/demo\n", "go test ./..."),
        (
            "Package.swift",
            "// swift-tools-version: 6.0\n",
            "swift test",
        ),
    ] {
        let temp = TempDir::new().expect("fixture should exist");
        std::fs::write(temp.path().join(fixture.0), fixture.1).expect("fixture should write");
        let runner = TestRunnerExecutor::for_selection(temp.path());

        let TestCommandSelection::Selected(selection) = runner.select_project_command().unwrap()
        else {
            panic!(
                "single project command should be selected for {}",
                fixture.0
            );
        };

        assert_eq!(selection.command(), fixture.2);
        assert_eq!(selection.confidence(), TestCommandConfidence::High);
        assert!(selection.reason().contains(fixture.0));
    }
}

#[test]
fn selects_single_low_confidence_python_command_with_reason() {
    let fixture = TempDir::new().expect("fixture should exist");
    std::fs::write(
        fixture.path().join("pyproject.toml"),
        "[tool.pytest.ini_options]\n",
    )
    .expect("python fixture should write");
    let runner = TestRunnerExecutor::for_selection(fixture.path());

    let TestCommandSelection::Selected(selection) = runner.select_project_command().unwrap() else {
        panic!("single low-confidence command should be selected with evidence");
    };

    assert_eq!(selection.command(), "pytest");
    assert_eq!(selection.confidence(), TestCommandConfidence::Low);
    assert!(selection.reason().contains("pyproject.toml"));
}

#[test]
fn asks_for_clarification_when_multiple_high_confidence_commands_exist() {
    let fixture = TempDir::new().expect("fixture should exist");
    std::fs::write(
        fixture.path().join("package.json"),
        r#"{"scripts":{"test":"vitest"}}"#,
    )
    .expect("package fixture should write");
    std::fs::write(
        fixture.path().join("Cargo.toml"),
        "[package]\nname='demo'\n",
    )
    .expect("cargo fixture should write");
    let runner = TestRunnerExecutor::for_selection(fixture.path());

    let TestCommandSelection::ClarificationRequired { candidates, reason } =
        runner.select_project_command().unwrap()
    else {
        panic!("multiple high-confidence commands should require clarification");
    };

    assert_eq!(
        candidates,
        vec!["npm test".to_string(), "cargo test".to_string()]
    );
    assert!(reason.contains("multiple"));
}
