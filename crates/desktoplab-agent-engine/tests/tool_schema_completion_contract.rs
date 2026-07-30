use desktoplab_agent_engine::DesktopLabToolRegistry;

#[test]
fn completion_schema_requires_grounded_outcome_evidence() {
    let complete = DesktopLabToolRegistry::default()
        .provider_tool_schemas()
        .into_iter()
        .find(|schema| schema["function"]["name"] == "desktoplab.complete")
        .expect("completion provider schema should exist");

    let description = complete["function"]["description"]
        .as_str()
        .expect("completion should describe its evidence contract");
    assert!(description.contains("cite every successful executor call"));

    let outcome = complete["function"]["parameters"]["properties"]["outcome"]["description"]
        .as_str()
        .expect("completion outcome should define its evidence semantics");
    for required in [
        "answered for read-only findings",
        "reports about existing Git changes",
        "agent applied a mutation",
        "verified only with passing test evidence",
    ] {
        assert!(outcome.contains(required), "{outcome}");
    }
    assert_eq!(
        complete["function"]["parameters"]["required"],
        serde_json::json!(["message", "outcome", "evidenceCallIds"])
    );
}
