use desktoplab_control_plane::LocalApiRouter;
use desktoplab_model_manager::{AgentContextWindowPolicy, AgentRequestTimeoutPolicy};
use serde_json::Value;
use xtask::check_logical_line_limit;

#[test]
fn setup_preview_comes_from_catalog_and_hardware_services() {
    let mut router = LocalApiRouter::default();
    let preview = route_json(&mut router, "GET", "/v1/setup/preview", "");

    assert_eq!(preview["source"], "service_backed");
    assert_eq!(preview["catalogSource"], "bundled_seed_catalog");
    assert_eq!(
        preview["hardware"]["operatingSystem"]["confidence"],
        "confirmed"
    );
    let models = preview["modelRecommendations"].as_array().unwrap();
    assert!(!models.is_empty());
    assert!(contains_manifest(models, "model.gemma4-12b-q4"));
    let available_memory_gb = ["ramGb", "vramGb", "unifiedMemoryGb"]
        .iter()
        .filter_map(|key| preview["hardware"][key]["value"].as_u64())
        .max()
        .unwrap_or_default();
    assert!(models.iter().all(|model| {
        model["requiredMemoryGb"]
            .as_u64()
            .is_some_and(|required| required <= available_memory_gb)
    }));
    let gemma = model(models, "model.gemma4-12b-q4");
    assert_eq!(gemma["familyId"], "family.gemma4");
    assert_eq!(gemma["familyName"], "Gemma 4");
    assert_eq!(gemma["parameterClass"], "small");
    assert_eq!(gemma["parametersBillion"], 12);
    assert_eq!(gemma["quantization"], "Q4");
    assert_eq!(gemma["contextWindowTokens"], 256_000);
    assert_eq!(
        gemma["agentContextWindowTokens"],
        AgentContextWindowPolicy::from_capacity(256_000, 16, available_memory_gb as u32)
    );
    assert_eq!(
        gemma["agentRequestTimeoutSeconds"],
        AgentRequestTimeoutPolicy::from_capacity(16, available_memory_gb as u32)
    );
    assert_eq!(gemma["requiredMemoryGb"], 16);
    assert_eq!(gemma["expectedDiskMb"], 7_600);
    assert_eq!(gemma["runtimeId"], "runtime.ollama");
    assert_eq!(gemma["licenseState"], "known");
    assert_eq!(gemma["trustLabel"], "License verified");
    assert_eq!(gemma["agentQualification"], "runtime_validation_required");
    assert_eq!(gemma["compatibilityReason"], "fits this machine");
    assert!(
        preview["hiddenReasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|reason| reason.as_str().is_some_and(|reason| {
                reason.starts_with("model.qwen3-coder-480b-q4:hidden_hardware")
            }))
    );
}

#[test]
fn catalog_refresh_creates_durable_backend_job() {
    let mut router = LocalApiRouter::default();

    let status = route_json(&mut router, "GET", "/v1/setup/catalog-refresh", "");
    assert_eq!(status["source"], "service_backed");
    assert_ne!(status["source"], "dry_run_contract_fixture");

    let refresh = route_json(&mut router, "POST", "/v1/setup/catalog-refresh", "");
    assert_eq!(refresh["source"], "service_backed");
    assert!(refresh["jobId"].as_str().unwrap().starts_with("job."));
    assert_eq!(refresh["state"], "completed");
    assert_eq!(refresh["catalogSource"], "bundled_seed_catalog");
}

#[test]
fn setup_rejects_incompatible_selection_and_orders_jobs() {
    let mut router = LocalApiRouter::default();

    let rejected = router
        .route(
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.future","modelId":"model.gemma4-12b-q4"}"#,
        )
        .expect("route should exist");
    assert_eq!(rejected.status(), "400 Bad Request");

    let accepted = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );

    assert_eq!(accepted["setup"]["state"], "in_progress");
    assert_eq!(accepted["pipeline"]["state"], "runtime_installing");
    assert_eq!(accepted["jobs"][0]["kind"], "runtime.install");
    assert_eq!(accepted["jobs"][0]["state"], "running");
    assert_eq!(accepted["jobs"][0]["pipelineState"], "runtime_installing");
    assert_eq!(accepted["jobs"][1]["kind"], "model.download");
    assert_eq!(accepted["jobs"][1]["state"], "blocked");
    assert_eq!(accepted["jobs"][1]["blockedReason"], "runtime_not_ready");
    assert_eq!(accepted["jobs"][1]["pipelineState"], "runtime_installing");
}

#[test]
fn setup_rejects_uncertified_mlx_model_on_every_host() {
    let mut router = LocalApiRouter::default();

    let response = router
        .route(
            "POST",
            "/v1/setup/accept",
            r#"{"runtimeId":"runtime.mlx-lm","modelId":"model.mlx-qwen-3.5-4b-8bit"}"#,
        )
        .expect("route should exist");

    let payload: Value = serde_json::from_str(response.body()).unwrap();
    assert_eq!(response.status(), "400 Bad Request");
    assert_eq!(payload["code"], "SETUP_SELECTION_INCOMPATIBLE");
}

#[test]
fn setup_readiness_events_show_ordered_backend_transitions() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_host_memory_gb_for_test(32);
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b"]);

    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/download",
        r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
    );
    let _ = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"client supplied inventory must be ignored"}"#,
    );
    let ready = route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    assert_eq!(ready["setup"]["state"], "ready");

    let replay = route_json(&mut router, "GET", "/v1/events/replay", "");
    let payloads = replay["frames"]
        .as_array()
        .expect("frames should be an array")
        .iter()
        .map(|frame| frame["payload"].as_str().unwrap_or_default())
        .collect::<Vec<_>>();

    let runtime_install = payload_index(&payloads, &[("kind", "runtime.install")]);
    let runtime_verify = payload_index(&payloads, &[("kind", "runtime.verify")]);
    let model_download = payload_index(
        &payloads,
        &[
            ("kind", "model.download"),
            ("modelId", "model.gemma4-12b-q4"),
            ("state", "running"),
        ],
    );
    let model_verify = payload_index(&payloads, &[("kind", "model.verify")]);

    assert!(runtime_install < runtime_verify);
    assert!(runtime_verify < model_download);
    assert!(model_download < model_verify);
}

#[test]
fn setup_can_finish_offline_when_runtime_and_model_are_already_present() {
    let mut router = LocalApiRouter::default();
    router.set_runtime_verification_for_test(true, "backend detected ollama 0.5.0");
    router.set_local_model_inventory_for_test(&["gemma4:12b"]);

    let _ = route_json(
        &mut router,
        "POST",
        "/v1/setup/accept",
        r#"{"runtimeId":"runtime.ollama","modelId":"model.gemma4-12b-q4"}"#,
    );
    let offline_install = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/install",
        r#"{"setupAccepted":true,"networkAvailable":false,"diskAvailableGb":64}"#,
    );
    assert_eq!(offline_install["state"], "blocked");
    assert_eq!(offline_install["blockedReason"], "network unavailable");

    let runtime = route_json(
        &mut router,
        "POST",
        "/v1/runtimes/runtime.ollama/verify",
        r#"{"versionOutput":"client supplied text must be ignored"}"#,
    );
    assert_eq!(runtime["verificationState"], "verified");

    let model = route_json(
        &mut router,
        "POST",
        "/v1/models/model.gemma4-12b-q4/verify",
        r#"{"inventoryOutput":"client supplied inventory must be ignored"}"#,
    );
    assert_eq!(model["verificationState"], "verified");

    let ready = route_json(&mut router, "POST", "/v1/setup/complete", "{}");
    assert_eq!(ready["setup"]["state"], "ready");
    assert_eq!(ready["setupPipeline"]["state"], "ready");
}

#[test]
fn setup_truth_sources_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_setup_truth.rs",
        include_str!("local_api_setup_truth.rs"),
        250,
    )
    .expect("setup truth test should stay focused");
}

fn contains_manifest(models: &[Value], manifest_id: &str) -> bool {
    models
        .iter()
        .any(|model| model["manifestId"].as_str() == Some(manifest_id))
}

fn model<'a>(models: &'a [Value], manifest_id: &str) -> &'a Value {
    models
        .iter()
        .find(|model| model["manifestId"].as_str() == Some(manifest_id))
        .expect("model recommendation should exist")
}

fn route_json(router: &mut LocalApiRouter, method: &str, path: &str, body: &str) -> Value {
    let response = router
        .route(method, path, body)
        .unwrap_or_else(|| panic!("{method} {path} should be routed"));
    assert_eq!(response.status(), "200 OK", "{}", response.body());
    serde_json::from_str(response.body()).expect("response should be json")
}

fn payload_index(payloads: &[&str], expected: &[(&str, &str)]) -> usize {
    payloads
        .iter()
        .position(|payload| {
            serde_json::from_str::<Value>(payload).is_ok_and(|value| {
                expected.iter().all(|(field, expected)| {
                    value.get(field).and_then(Value::as_str) == Some(*expected)
                })
            })
        })
        .unwrap_or_else(|| panic!("payload matching {expected:?} should exist: {payloads:?}"))
}
