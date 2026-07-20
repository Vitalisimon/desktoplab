use xtask::product_truth::{ProductTruthGate, ProductTruthInput, ProductTruthViolation};

#[test]
fn product_truth_gate_blocks_packaging_acceptance_before_reopen() {
    let input = ProductTruthInput::new(
        "Status: accepted\n",
        "Status: implementation-ready\n",
        "Status: active\n",
        "",
        "",
        "",
    );

    assert_eq!(
        ProductTruthGate::verify(&input),
        Err(ProductTruthViolation::PackagingGateAcceptedEarly)
    );
}

#[test]
fn product_truth_gate_rejects_intercepted_readiness_evidence() {
    let input = ProductTruthInput::new(
        "Status: blocked-by-product-truth\n",
        "Status: paused-by-product-truth\n",
        "Status: active\n",
        "product evidence: apps/desktop/tests/live-capabilities.spec.ts uses page.route",
        "",
        "",
    );

    assert_eq!(
        ProductTruthGate::verify(&input),
        Err(ProductTruthViolation::RouteInterceptionUsedForReadiness)
    );
}

#[test]
fn product_truth_gate_allows_route_interception_only_as_support_evidence() {
    let input = ProductTruthInput::new(
        "Status: blocked-by-product-truth\n",
        "Status: paused-by-product-truth\n",
        "Status: active\nproduct evidence\nsupport evidence\nrejected-for-readiness\n",
        "product evidence: live local api smoke\nsupport evidence: route-intercepted visual smoke",
        "blocked without invented success",
        "",
    );

    assert_eq!(ProductTruthGate::verify(&input), Ok(()));
}

#[test]
fn product_truth_gate_rejects_critical_static_success_payloads() {
    let input = ProductTruthInput::new(
        "Status: blocked-by-product-truth\n",
        "Status: paused-by-product-truth\n",
        "Status: active\n",
        "product evidence: live local api",
        r#"json!({"sessionId":"session.1"})"#,
        "",
    );

    assert_eq!(
        ProductTruthGate::verify(&input),
        Err(ProductTruthViolation::CriticalStaticPayload)
    );
}

#[test]
fn product_truth_gate_rejects_client_supplied_readiness_shortcuts() {
    let input = ProductTruthInput::new(
        "Status: blocked-by-backend-owned-readiness\n",
        "Status: paused-by-backend-owned-readiness\n",
        "Status: active\nproduct evidence\nsupport evidence\nrejected-for-readiness\n",
        "product evidence: live local api",
        "blocked without invented success",
        r#"let runtime_ready = body_field(body, "runtimeReady");"#,
    );

    assert_eq!(
        ProductTruthGate::verify(&input),
        Err(ProductTruthViolation::ReadinessShortcut)
    );
}

#[test]
fn product_truth_gate_rejects_reopen_without_task_90_proof() {
    let input = ProductTruthInput::new(
        "Status: accepted-after-product-truth\n",
        "Status: implementation-ready-after-product-truth\n",
        "### Task 23.5.090 - Packaging Reopen Decision\nStatus: planned\n",
        "product evidence\nsupport evidence\nrejected-for-readiness\n",
        "blocked without invented success",
        "",
    );

    assert_eq!(
        ProductTruthGate::verify(&input),
        Err(ProductTruthViolation::PackagingReopenMissingProof)
    );
}

#[test]
fn product_truth_gate_accepts_blocked_packaging_and_live_evidence_policy() {
    let input = ProductTruthInput::new(
        "Status: blocked-by-product-truth\n",
        "Status: paused-by-product-truth\n",
        "Status: active\nproduct evidence\nsupport evidence\nrejected-for-readiness\n",
        "local API live smoke and fresh-state desktop smoke",
        "blocked without invented success",
        "",
    );

    assert_eq!(ProductTruthGate::verify(&input), Ok(()));
}

#[test]
fn product_truth_gate_accepts_reopen_after_task_90_proof() {
    let input = ProductTruthInput::new(
        "Status: accepted-after-product-truth\n",
        "Status: implementation-ready-after-product-truth\n",
        "### Task 23.5.090 - Packaging Reopen Decision\nStatus: complete\n",
        "product evidence\nsupport evidence\nrejected-for-readiness\n",
        "blocked without invented success",
        "",
    );

    assert_eq!(ProductTruthGate::verify(&input), Ok(()));
}

#[test]
fn product_truth_gate_rejects_backend_readiness_reopen_without_task_105() {
    let input = ProductTruthInput::new(
        "Status: accepted-after-backend-owned-readiness\n",
        "Status: implementation-ready-after-backend-owned-readiness\n",
        "### Task 23.5.105 - Backend-Owned Readiness Reopen Decision\nStatus: planned\n",
        "product evidence\nsupport evidence\nrejected-for-readiness\n",
        "blocked without invented success",
        "backend owned readiness routes",
    );

    assert_eq!(
        ProductTruthGate::verify(&input),
        Err(ProductTruthViolation::PackagingReopenMissingProof)
    );
}

#[test]
fn product_truth_gate_accepts_backend_readiness_reopen_after_task_105() {
    let input = ProductTruthInput::new(
        "Status: accepted-after-backend-owned-readiness\n",
        "Status: implementation-ready-after-backend-owned-readiness\n",
        "### Task 23.5.105 - Backend-Owned Readiness Reopen Decision\nStatus: complete\n",
        "product evidence\nsupport evidence\nrejected-for-readiness\n",
        "blocked without invented success",
        "backend owned readiness routes",
    );

    assert_eq!(ProductTruthGate::verify(&input), Ok(()));
}

#[test]
fn product_truth_gate_accepts_export_safe_fixture() {
    let input = ProductTruthInput::new(
        "Status: accepted-after-backend-owned-readiness\n",
        "Status: implementation-ready-after-backend-owned-readiness\n",
        "### Task 23.5.105 - Backend-Owned Readiness Reopen Decision\nStatus: complete\nproduct evidence\nsupport evidence\nrejected-for-readiness\n",
        "product evidence: live local api\nsupport evidence: visual screenshot\nrejected-for-readiness: route-intercepted smoke\n",
        include_str!("../../crates/desktoplab-control-plane/src/router_payloads.rs"),
        include_str!("../../crates/desktoplab-control-plane/src/router/setup_runtime_model.rs"),
    );

    assert_eq!(ProductTruthGate::verify(&input), Ok(()));
}

#[test]
fn product_truth_source_stays_small() {
    xtask::check_logical_line_limit(
        "xtask/src/product_truth.rs",
        include_str!("../src/product_truth.rs"),
        140,
    )
    .expect("product truth gate should stay focused");
}
