#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductTruthInput<'a> {
    packaging_gate: &'a str,
    packaging_plan: &'a str,
    plan_23_5: &'a str,
    evidence_policy: &'a str,
    router_payloads: &'a str,
    control_plane_routes: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductTruthViolation {
    PackagingGateAcceptedEarly,
    PackagingPlanNotPaused,
    PackagingReopenMissingProof,
    MissingEvidenceTaxonomy,
    RouteInterceptionUsedForReadiness,
    CriticalStaticPayload,
    ReadinessShortcut,
}

pub struct ProductTruthGate;

impl<'a> ProductTruthInput<'a> {
    #[must_use]
    pub fn new(
        packaging_gate: &'a str,
        packaging_plan: &'a str,
        plan_23_5: &'a str,
        evidence_policy: &'a str,
        router_payloads: &'a str,
        control_plane_routes: &'a str,
    ) -> Self {
        Self {
            packaging_gate,
            packaging_plan,
            plan_23_5,
            evidence_policy,
            router_payloads,
            control_plane_routes,
        }
    }
}

impl ProductTruthGate {
    pub fn verify(input: &ProductTruthInput<'_>) -> Result<(), ProductTruthViolation> {
        if input
            .packaging_gate
            .contains("Status: accepted-after-product-truth")
        {
            if !input
                .packaging_plan
                .contains("implementation-ready-after-product-truth")
                || !task_90_is_complete(input.plan_23_5)
            {
                return Err(ProductTruthViolation::PackagingReopenMissingProof);
            }
        } else if input
            .packaging_gate
            .contains("Status: accepted-after-backend-owned-readiness")
        {
            if !input
                .packaging_plan
                .contains("implementation-ready-after-backend-owned-readiness")
                || !task_105_is_complete(input.plan_23_5)
            {
                return Err(ProductTruthViolation::PackagingReopenMissingProof);
            }
        } else if input.packaging_gate.contains("Status: accepted") {
            return Err(ProductTruthViolation::PackagingGateAcceptedEarly);
        }
        if !input.packaging_plan.contains("paused-by-product-truth")
            && !input
                .packaging_plan
                .contains("paused-by-backend-owned-readiness")
            && !input
                .packaging_plan
                .contains("implementation-ready-after-product-truth")
            && !input
                .packaging_plan
                .contains("implementation-ready-after-backend-owned-readiness")
        {
            return Err(ProductTruthViolation::PackagingPlanNotPaused);
        }
        if cites_route_interception_as_product(input.evidence_policy) {
            return Err(ProductTruthViolation::RouteInterceptionUsedForReadiness);
        }
        if contains_critical_static_payload(input.router_payloads) {
            return Err(ProductTruthViolation::CriticalStaticPayload);
        }
        if contains_readiness_shortcut(input.control_plane_routes) {
            return Err(ProductTruthViolation::ReadinessShortcut);
        }
        if !has_evidence_taxonomy(input.plan_23_5) && !has_evidence_taxonomy(input.evidence_policy)
        {
            return Err(ProductTruthViolation::MissingEvidenceTaxonomy);
        }
        Ok(())
    }
}

fn has_evidence_taxonomy(source: &str) -> bool {
    source.contains("product evidence")
        && source.contains("support evidence")
        && source.contains("rejected-for-readiness")
}

fn cites_route_interception_as_product(source: &str) -> bool {
    source.lines().any(|line| {
        let lower = line.to_ascii_lowercase();
        lower.contains("product evidence")
            && (lower.contains("page.route") || lower.contains("route-intercepted"))
    })
}

fn contains_critical_static_payload(source: &str) -> bool {
    [
        "session.1",
        "dry_run_contract_fixture",
        "\"state\":\"ready\"",
    ]
    .iter()
    .any(|needle| source.contains(needle))
}

fn contains_readiness_shortcut(source: &str) -> bool {
    source.lines().any(|line| {
        let line = line.trim();
        (line.contains("body_bool(body, \"runtimeReady\")")
            || line.contains("body_bool(body, \"modelReady\")")
            || line.contains("body_field(body, \"runtimeReady\")")
            || line.contains("body_field(body, \"modelReady\")"))
            && !line.starts_with("//")
    })
}

fn task_90_is_complete(plan: &str) -> bool {
    plan.split("### Task 23.5.090 - Packaging Reopen Decision")
        .nth(1)
        .and_then(|after| after.split("\n### Task ").next())
        .is_some_and(|task| task.contains("Status: complete"))
}

fn task_105_is_complete(plan: &str) -> bool {
    plan.split("### Task 23.5.105 - Backend-Owned Readiness Reopen Decision")
        .nth(1)
        .and_then(|after| after.split("\n### Task ").next())
        .is_some_and(|task| task.contains("Status: complete"))
}
