use desktoplab_agent_engine::{AgentToolExecutionOwner, DesktopLabToolRegistry};

#[test]
fn provider_schemas_keep_parameter_names_stable() {
    let registry = DesktopLabToolRegistry::default();
    let read_file = provider_schema(&registry, "desktoplab.read_file");
    let list_files = provider_schema(&registry, "desktoplab.list_files");
    let write_file = provider_schema(&registry, "desktoplab.write_file");
    let patch_file = provider_schema(&registry, "desktoplab.patch_file");
    let search_text = provider_schema(&registry, "desktoplab.search_text");
    let run_terminal = provider_schema(&registry, "desktoplab.run_terminal");
    let start_process = provider_schema(&registry, "desktoplab.start_process");
    let run_tests = provider_schema(&registry, "desktoplab.run_tests");
    let complete = provider_schema(&registry, "desktoplab.complete");
    let commit = provider_schema(&registry, "desktoplab.commit_changes");
    let spawn = provider_schema(&registry, "desktoplab.spawn_subagent");
    let update_plan = provider_schema(&registry, "desktoplab.update_plan");
    let clarify = provider_schema(&registry, "desktoplab.clarify");

    assert_eq!(read_file["function"]["parameters"]["required"][0], "path");
    for schema in [
        &list_files,
        &read_file,
        &write_file,
        &patch_file,
        &search_text,
    ] {
        let path = property(schema, "path")["description"]
            .as_str()
            .expect("filesystem path should describe its scope");
        assert!(path.contains("Workspace-relative"), "{path}");
        assert!(path.contains("absolute path"), "{path}");
    }
    assert!(
        property(&list_files, "path")["description"]
            .as_str()
            .unwrap()
            .contains("omit it to target the workspace root")
    );
    assert_eq!(property(&read_file, "offset")["type"], "integer");
    assert_eq!(property(&read_file, "limit")["maximum"], 2000);
    assert_eq!(patch_file["function"]["parameters"]["required"][0], "path");
    assert_eq!(property(&search_text, "regex")["type"], "boolean");
    assert_eq!(property(&search_text, "caseSensitive")["type"], "boolean");
    assert_eq!(
        patch_file["function"]["parameters"]["required"][1],
        "expected"
    );
    assert_eq!(
        patch_file["function"]["parameters"]["required"][2],
        "replacement"
    );
    assert_eq!(property(&patch_file, "replaceAll")["type"], "boolean");
    assert_eq!(property(&run_tests, "command")["type"], "string");
    assert_eq!(property(&run_tests, "timeoutSeconds")["maximum"], 1800);
    assert_eq!(property(&run_terminal, "timeoutSeconds")["maximum"], 1800);
    for schema in [&run_terminal, &start_process] {
        let cwd = property(schema, "cwd")["description"]
            .as_str()
            .expect("process cwd should describe its scope");
        assert!(cwd.contains("Workspace-relative"), "{cwd}");
        assert!(cwd.contains("Omit it for the workspace root"), "{cwd}");
        assert!(cwd.contains("absolute path"), "{cwd}");
    }
    assert!(description(&complete).contains("cite every successful executor call"));
    let outcome = property(&complete, "outcome")["description"]
        .as_str()
        .expect("completion outcome should define its evidence semantics");
    assert!(outcome.contains("answered for read-only findings"), "{outcome}");
    assert!(outcome.contains("reports about existing Git changes"), "{outcome}");
    assert!(outcome.contains("agent applied a mutation"), "{outcome}");
    assert!(outcome.contains("verified only with passing test evidence"), "{outcome}");
    assert_eq!(
        complete["function"]["parameters"]["required"],
        serde_json::json!(["message", "outcome", "evidenceCallIds"])
    );
    assert!(description(&clarify).contains("absent from executor observations"));
    assert_eq!(
        clarify["function"]["parameters"]["required"],
        serde_json::json!(["question", "blockedOn"])
    );
    assert_eq!(read_file["function"]["name"], "desktoplab.read_file");
    assert_eq!(
        spawn["function"]["parameters"]["properties"]["intent"]["enum"],
        serde_json::json!(["read_only", "write_capable"])
    );
    assert!(description(&spawn).contains("must commit completed changes"));
    assert!(
        description(&provider_schema(&registry, "desktoplab.get_subagent"))
            .contains("readyToIntegrate")
    );
    assert_eq!(
        update_plan["function"]["parameters"]["properties"]["steps"]["maxItems"],
        20
    );
    assert!(description(&write_file).contains("complete content requested"));
    assert!(description(&patch_file).contains("localized exact-text replacement"));
    assert_eq!(
        commit["function"]["parameters"]["properties"]["paths"]["items"]["type"],
        "string"
    );
}

#[test]
fn output_contracts_are_standard_json_schemas() {
    let registry = DesktopLabToolRegistry::default();

    for tool in registry.tools() {
        let output = tool.output_shape();
        if tool.execution_owner() == AgentToolExecutionOwner::LoopControl {
            assert_eq!(output["type"], "null", "{}", tool.id());
            continue;
        }
        assert_eq!(output["type"], "object", "{}", tool.id());
        assert!(output.get("fields").is_none(), "{}", tool.id());
        if let Some(required) = output.get("required").and_then(serde_json::Value::as_array) {
            let properties = output["properties"]
                .as_object()
                .unwrap_or_else(|| panic!("{} requires output properties", tool.id()));
            for field in required {
                let field = field.as_str().expect("required field must be a string");
                assert!(
                    properties.contains_key(field),
                    "{} missing {field}",
                    tool.id()
                );
            }
        }
    }
}

fn description(schema: &serde_json::Value) -> &str {
    schema["function"]["description"].as_str().unwrap()
}

fn property<'a>(schema: &'a serde_json::Value, name: &str) -> &'a serde_json::Value {
    &schema["function"]["parameters"]["properties"][name]
}

fn provider_schema(registry: &DesktopLabToolRegistry, id: &str) -> serde_json::Value {
    registry
        .provider_tool_schemas()
        .into_iter()
        .find(|schema| schema["function"]["name"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("{id} provider schema should exist"))
}
