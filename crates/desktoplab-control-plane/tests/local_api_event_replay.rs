use desktoplab_control_plane::LocalApiRouter;
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn setup_accept_creates_service_backed_jobs_and_replay_events() {
    let mut router = LocalApiRouter::default();

    let accepted = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    assert_eq!(accepted["startedJobIds"][0], "job.1");

    let jobs = route_json(&mut router, "GET", "/v1/jobs", "");
    assert_eq!(jobs["source"], "service_backed");
    assert_eq!(jobs["jobs"][0]["jobId"], "job.1");
    assert_eq!(jobs["jobs"][0]["state"], "running");

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    assert_eq!(replay["source"], "service_backed");
    assert!(replay["streamId"].as_str().is_some_and(|id| !id.is_empty()));
    assert_eq!(replay["oldestSequence"], 1);
    assert_eq!(replay["latestSequence"], replay["nextSequence"]);
    assert_eq!(replay["hasMore"], false);
    assert_eq!(replay["gapDetected"], false);
    assert_eq!(replay["resetRequired"], false);
    assert!(replay["frames"].as_array().unwrap().len() >= 2);
    assert_eq!(replay["frames"][0]["scope"], "job");
}

#[test]
fn event_replay_reports_page_boundaries_and_stream_reset() {
    let mut router = LocalApiRouter::default();
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    let first = route_json(&mut router, "GET", "/v1/events/replay?limit=1", "");
    assert_eq!(first["hasMore"], true);
    assert_eq!(first["frames"].as_array().unwrap().len(), 1);
    let reset = route_json(
        &mut router,
        "GET",
        "/v1/events/replay?after_sequence=999&stream_id=stale",
        "",
    );
    assert_eq!(reset["resetRequired"], true);
    assert!(!reset["frames"].as_array().unwrap().is_empty());
}

#[test]
fn event_replay_respects_after_sequence_cursor() {
    let mut router = LocalApiRouter::default();

    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let full_replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let first_sequence = full_replay["frames"][0]["sequence"]
        .as_u64()
        .expect("first frame sequence");

    let resumed = route_json(
        &mut router,
        "GET",
        &format!("/v1/events/replay?after_sequence={first_sequence}"),
        "",
    );

    let sequences = resumed["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .map(|frame| frame["sequence"].as_u64().expect("sequence"))
        .collect::<Vec<_>>();
    assert!(
        sequences.iter().all(|sequence| *sequence > first_sequence),
        "{sequences:?}"
    );
    assert!(
        sequences.len() < full_replay["frames"].as_array().unwrap().len(),
        "{sequences:?}"
    );
}

#[test]
fn runtime_install_publishes_phase_progress_events() {
    let mut router = LocalApiRouter::default();

    let install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableGb":64}"#,
    );
    assert_eq!(install["source"], "service_backed");

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = replay["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .map(|frame| frame["payload"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(payloads.contains(r#""phase":"detect""#), "{payloads}");
    assert!(payloads.contains(r#""phase":"verify""#), "{payloads}");
    assert!(payloads.contains(r#""nextAction":"#), "{payloads}");
}

#[test]
fn model_download_publishes_running_progress_events() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_host_memory_gb_for_test(32);
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"ollama 0.5.0"}"#,
    );

    let download = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    assert!(
        download["progressPercent"].as_u64().unwrap_or_default() >= 5,
        "{download}"
    );

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = replay["frames"]
        .as_array()
        .expect("frames")
        .iter()
        .map(|frame| frame["payload"].as_str().unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        payloads.contains(r#""kind":"model.download""#),
        "{payloads}"
    );
    assert!(payloads.contains(r#""state":"running""#), "{payloads}");
    assert!(payloads.contains(r#""progressPercent":"#), "{payloads}");
}

#[test]
fn local_api_event_replay_test_stays_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_event_replay.rs",
        include_str!("local_api_event_replay.rs"),
        180,
    )
    .expect("event replay test should stay focused");
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}
