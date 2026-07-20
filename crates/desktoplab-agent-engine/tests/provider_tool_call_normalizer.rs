use desktoplab_agent_engine::{ProviderToolCallNormalizer, ToolCallNormalizationError};
use serde_json::json;
use xtask::check_logical_line_limit;

#[test]
fn normalizes_native_and_direct_provider_tool_calls() {
    let normalizer = ProviderToolCallNormalizer::default();
    let native = json!({
        "id":"call-read",
        "function":{
            "name":"desktoplab.read_file",
            "arguments":"{\"path\":\"README.md\"}"
        }
    });
    let direct = json!({
        "id":"call-search",
        "tool":"desktoplab.search_text",
        "arguments":{"query":"agent","path":"crates"}
    });

    let read = normalizer
        .from_provider_value(&native)
        .expect("native call should normalize");
    let search = normalizer
        .from_provider_value(&direct)
        .expect("direct call should normalize");

    assert_eq!(read.name(), "desktoplab.read_file");
    assert_eq!(read.arguments()["path"], "README.md");
    assert_eq!(search.name(), "desktoplab.search_text");
}

#[test]
fn invalid_tool_arguments_fail_closed() {
    let normalizer = ProviderToolCallNormalizer::default();

    assert_eq!(
        normalizer.normalize("1", "desktoplab.unknown", json!({})),
        Err(ToolCallNormalizationError::UnknownTool)
    );
    assert_eq!(
        normalizer.normalize("2", "desktoplab.read_file", json!({})),
        Err(ToolCallNormalizationError::MissingArgument("path".into()))
    );
    assert_eq!(
        normalizer.normalize(
            "3",
            "desktoplab.read_file",
            json!({"path":"README.md","surprise":true}),
        ),
        Err(ToolCallNormalizationError::UnexpectedArgument(
            "surprise".into()
        ))
    );
    assert_eq!(
        normalizer.normalize("4", "desktoplab.read_file", json!({"path":7})),
        Err(ToolCallNormalizationError::InvalidArgumentType(
            "path".into()
        ))
    );
}

#[test]
fn malformed_provider_envelopes_fail_closed() {
    let normalizer = ProviderToolCallNormalizer::default();

    assert_eq!(
        normalizer.from_provider_value(&json!({
            "id":"bad",
            "function":{"name":"desktoplab.read_file","arguments":"not-json"}
        })),
        Err(ToolCallNormalizationError::MalformedArguments)
    );
    assert_eq!(
        normalizer.from_provider_value(&json!({"tool":"desktoplab.git_status"})),
        Err(ToolCallNormalizationError::MalformedEnvelope)
    );
    assert_eq!(
        normalizer.from_provider_value(&json!({
            "id":"ambiguous",
            "tool":"desktoplab.git_status",
            "arguments":{},
            "function":{"name":"desktoplab.read_file","arguments":{"path":"README.md"}}
        })),
        Err(ToolCallNormalizationError::MalformedEnvelope)
    );
}

#[test]
fn declared_schema_constraints_fail_closed_before_execution() {
    let normalizer = ProviderToolCallNormalizer::default();

    assert_eq!(
        normalizer.normalize(
            "limit",
            "desktoplab.read_file",
            json!({"path":"README.md","limit":0}),
        ),
        Err(ToolCallNormalizationError::InvalidArgument("limit".into()))
    );
    assert_eq!(
        normalizer.normalize(
            "enum",
            "desktoplab.clarify",
            json!({"question":"Choose","blockedOn":"desktoplab.unknown"}),
        ),
        Err(ToolCallNormalizationError::InvalidArgument(
            "blockedOn".into()
        ))
    );
    assert_eq!(
        normalizer.normalize(
            "unique",
            "desktoplab.commit_changes",
            json!({"message":"test","paths":["README.md","README.md"]}),
        ),
        Err(ToolCallNormalizationError::InvalidArgument("paths".into()))
    );
    assert_eq!(
        normalizer.normalize(
            "nested",
            "desktoplab.update_plan",
            json!({"steps":[{"step":"work","status":"unknown"}]}),
        ),
        Err(ToolCallNormalizationError::InvalidArgument(
            "steps[0].status".into()
        ))
    );
}

#[test]
fn declared_output_contracts_fail_closed_after_execution() {
    let normalizer = ProviderToolCallNormalizer::default();

    normalizer
        .validate_output(
            "desktoplab.read_file",
            &json!({"text":"contents","path":"README.md"}),
        )
        .expect("declared output should validate");
    assert_eq!(
        normalizer.validate_output("desktoplab.read_file", &json!({"path":"README.md"})),
        Err(ToolCallNormalizationError::MissingArgument("text".into()))
    );
    assert_eq!(
        normalizer.validate_output("desktoplab.read_file", &json!("contents")),
        Err(ToolCallNormalizationError::InvalidArgumentType(
            "arguments".into()
        ))
    );
    assert_eq!(
        normalizer.validate_output("desktoplab.run_tests", &json!({"passed":"yes"})),
        Err(ToolCallNormalizationError::InvalidArgumentType(
            "passed".into()
        ))
    );
    assert_eq!(
        normalizer.validate_output("desktoplab.list_files", &json!({"entries":false})),
        Err(ToolCallNormalizationError::InvalidArgumentType(
            "entries".into()
        ))
    );
}

#[test]
fn provider_tool_call_normalizer_stays_below_line_guard() {
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/tool_call_normalizer.rs",
        include_str!("../src/tool_call_normalizer.rs"),
        250,
    )
    .expect("provider tool call normalizer grew too large");
    check_logical_line_limit(
        "crates/desktoplab-agent-engine/src/json_schema_validator.rs",
        include_str!("../src/json_schema_validator.rs"),
        250,
    )
    .expect("JSON schema validator grew too large");
}
