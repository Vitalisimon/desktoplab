use desktoplab_control_plane::LocalApiRouter;
use xtask::check_logical_line_limit;

#[test]
fn uncertified_mlx_model_is_absent_from_download_and_verify_routes() {
    let mut router = LocalApiRouter::default();
    router.plan_model_downloads_for_test();
    router.set_host_memory_gb_for_test(32);
    router.mark_runtime_verified_for_test("runtime.mlx-lm", "mlx-lm import ok");

    let download = router
        .route(
            "POST",
            "/v1/models/model.mlx-qwen-3.5-4b-8bit/download",
            r#"{"setupAccepted":true,"networkAvailable":true,"diskAvailableMb":100000}"#,
        )
        .unwrap();
    let download_payload: serde_json::Value = serde_json::from_str(download.body()).unwrap();
    assert_eq!(download.status(), "200 OK");
    assert_eq!(download_payload["state"], "blocked");
    assert_eq!(download_payload["blockedReason"], "unknown model");

    let verify = router
        .route(
            "POST",
            "/v1/models/model.mlx-qwen-3.5-4b-8bit/verify",
            r#"{"inventoryOutput":"mlx-community/Qwen-3.5-4B-8bit"}"#,
        )
        .unwrap();
    let verify_payload: serde_json::Value = serde_json::from_str(verify.body()).unwrap();
    assert_eq!(verify.status(), "400 Bad Request");
    assert_eq!(verify_payload["code"], "UNKNOWN_MODEL");
}

#[test]
fn mlx_lm_route_tests_stay_small() {
    check_logical_line_limit(
        "crates/desktoplab-control-plane/tests/local_api_mlx_lm_model_routes.rs",
        include_str!("local_api_mlx_lm_model_routes.rs"),
        120,
    )
    .expect("MLX-LM route tests should stay focused");
}
