use desktoplab_acp_plugin::{
    AcpCapabilityStatus, AcpHostPrompt, AcpProtocolAdapter, AcpSessionHost, acp_capability_matrix,
};
use serde_json::{Value, json};
use xtask::check_logical_line_limit;

#[derive(Default)]
struct Host {
    cancelled: bool,
}

impl AcpSessionHost for Host {
    fn create_session(&mut self, _: &str) -> Result<String, String> {
        Ok("session.1".to_string())
    }
    fn prompt(&mut self, _: &str, prompt: &str) -> Result<AcpHostPrompt, String> {
        Ok(AcpHostPrompt {
            message: format!("received: {prompt}"),
            stop_reason: "end_turn".to_string(),
        })
    }
    fn cancel(&mut self, _: &str) -> Result<(), String> {
        self.cancelled = true;
        Ok(())
    }
}

#[test]
fn required_v1_lifecycle_negotiates_and_dispatches() {
    let mut adapter = AcpProtocolAdapter::new(Host::default());
    let initialized = adapter.dispatch(&request(
        0,
        "initialize",
        json!({"protocolVersion":1,"clientCapabilities":{}}),
    ));
    assert_eq!(result(&initialized.response)["protocolVersion"], 1);
    assert_eq!(
        result(&initialized.response)["agentCapabilities"]["loadSession"],
        false
    );
    let created = adapter.dispatch(&request(
        1,
        "session/new",
        json!({"cwd":"/repo","mcpServers":[]}),
    ));
    assert_eq!(result(&created.response)["sessionId"], "session.1");
    let prompt = adapter.dispatch(&request(
        2,
        "session/prompt",
        json!({
            "sessionId":"session.1","prompt":[{"type":"text","text":"inspect"}]
        }),
    ));
    assert_eq!(result(&prompt.response)["stopReason"], "end_turn");
    assert_eq!(prompt.notifications[0]["method"], "session/update");
    let cancelled = adapter.dispatch(
        &json!({"jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":"session.1"}}),
    );
    assert!(cancelled.response.is_none());
    assert!(adapter.host().cancelled);
}

#[test]
fn unsupported_operations_are_not_advertised_or_silently_accepted() {
    let matrix = acp_capability_matrix();
    assert!(matrix.contains(&("session/load", AcpCapabilityStatus::Unsupported)));
    assert!(matrix.contains(&(
        "session/request_permission",
        AcpCapabilityStatus::Unsupported
    )));
    let mut adapter = AcpProtocolAdapter::new(Host::default());
    adapter.dispatch(&request(0, "initialize", json!({"protocolVersion":99})));
    let unsupported = adapter.dispatch(&request(
        3,
        "session/load",
        json!({"sessionId":"session.1"}),
    ));
    assert_eq!(unsupported.response.unwrap()["error"]["code"], -32601);
}

#[test]
fn protocol_sources_stay_below_line_guards() {
    check_logical_line_limit(
        "crates/desktoplab-acp-plugin/src/protocol.rs",
        include_str!("../src/protocol.rs"),
        240,
    )
    .unwrap();
    check_logical_line_limit(
        "crates/desktoplab-acp-plugin/tests/acp_conformance.rs",
        include_str!("acp_conformance.rs"),
        140,
    )
    .unwrap();
}

fn request(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":method,"params":params})
}

fn result(response: &Option<Value>) -> &Value {
    &response.as_ref().expect("response expected")["result"]
}
