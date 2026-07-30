use desktoplab_runtime::{
    DeterministicProcessRunner, HttpLmStudioModelProbe, LmStudioDiscoveryState, LmStudioModelProbe,
    RuntimeEndpointError, discover_existing_lm_studio,
};
use xtask::check_logical_line_limit;

struct Models(Result<Vec<String>, RuntimeEndpointError>);

impl LmStudioModelProbe for Models {
    fn models(&self, _endpoint: &str) -> Result<Vec<String>, RuntimeEndpointError> {
        self.0.clone()
    }
}

#[test]
fn discovers_running_loopback_server_and_models() {
    let runner = DeterministicProcessRunner::sequence(vec![
        (Some(0), r#"{"running":true,"port":12345}"#, ""),
        (
            Some(0),
            r#"{"status":"running","isDaemon":true,"version":"0.4.4+1"}"#,
            "",
        ),
    ]);
    let discovery = discover_existing_lm_studio(&runner, &Models(Ok(vec!["qwen/test".into()])));

    assert!(discovery.ready());
    assert_eq!(discovery.endpoint(), Some("http://127.0.0.1:12345"));
    assert_eq!(discovery.models(), ["qwen/test"]);
    assert_eq!(discovery.version(), Some("0.4.4+1"));
    assert_eq!(discovery.daemon_managed(), Some(true));
    assert_eq!(discovery.ownership(), "user_owned");
}

#[test]
fn missing_cli_fails_without_probing_an_endpoint() {
    let discovery = discover_existing_lm_studio(
        &DeterministicProcessRunner::missing(),
        &Models(Err(RuntimeEndpointError::Unreachable)),
    );
    assert_eq!(discovery.state(), LmStudioDiscoveryState::CliMissing);
    assert!(!discovery.ready());
}

#[test]
fn stopped_server_is_not_treated_as_ready() {
    let runner = DeterministicProcessRunner::succeeds(r#"{"running":false,"port":1234}"#, "");
    let discovery =
        discover_existing_lm_studio(&runner, &Models(Err(RuntimeEndpointError::Unreachable)));
    assert_eq!(discovery.state(), LmStudioDiscoveryState::ServerStopped);
}

#[test]
fn invalid_port_fails_closed() {
    let runner = DeterministicProcessRunner::succeeds(r#"{"running":true,"port":70000}"#, "");
    let discovery =
        discover_existing_lm_studio(&runner, &Models(Err(RuntimeEndpointError::Unreachable)));
    assert_eq!(discovery.state(), LmStudioDiscoveryState::InvalidStatus);
}

#[test]
fn unreachable_model_inventory_fails_closed() {
    let runner = DeterministicProcessRunner::succeeds(r#"{"running":true,"port":1234}"#, "");
    let discovery =
        discover_existing_lm_studio(&runner, &Models(Err(RuntimeEndpointError::Unreachable)));
    assert_eq!(
        discovery.state(),
        LmStudioDiscoveryState::EndpointUnavailable
    );
}

#[test]
fn empty_model_inventory_fails_closed() {
    let runner = DeterministicProcessRunner::succeeds(r#"{"running":true,"port":1234}"#, "");
    let discovery = discover_existing_lm_studio(&runner, &Models(Ok(Vec::new())));
    assert_eq!(
        discovery.state(),
        LmStudioDiscoveryState::EndpointUnavailable
    );
}

#[test]
fn http_probe_rejects_non_loopback_endpoint_before_network_access() {
    let private_host = [192, 168, 1, 9].map(|octet| octet.to_string()).join(".");
    let error = HttpLmStudioModelProbe
        .models(&format!("http://{private_host}:1234"))
        .expect_err("LM Studio discovery must remain loopback-only");
    assert_eq!(error, RuntimeEndpointError::NonLocalEndpoint);
}

#[test]
fn lm_studio_discovery_source_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-runtime/src/lm_studio_discovery.rs",
        include_str!("../src/lm_studio_discovery.rs"),
        210,
    )
    .expect("LM Studio discovery should stay focused");
}
