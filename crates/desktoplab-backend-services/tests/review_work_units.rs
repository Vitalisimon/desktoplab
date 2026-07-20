use desktoplab_backend_services::{
    DeliveryKind, PatchAttempt, ReviewFinding, ReviewWorkUnitError, ReviewWorkUnitService,
    VerificationRecord,
};
use desktoplab_storage::SqliteStore;
use tempfile::TempDir;

#[test]
fn interrupted_review_resumes_and_patch_scope_cannot_expand() {
    let fixture = TempDir::new().unwrap();
    let database = fixture.path().join("review.sqlite");
    {
        let store = migrated_store(&database);
        let service = ReviewWorkUnitService::new(&store);
        service
            .create("review.1", "feature.auth", "commit:abc", true)
            .unwrap();
        service
            .add_finding("review.1", finding("F-1", "src/auth.rs"))
            .unwrap();
        service
            .add_finding("review.1", finding("F-2", "src/http.rs"))
            .unwrap();
    }

    let store = migrated_store(&database);
    let service = ReviewWorkUnitService::new(&store);
    assert_eq!(service.load("review.1").unwrap().findings.len(), 2);
    assert_eq!(
        service.authorize_fix("review.1", &["F-1".to_string()], true, false),
        Err(ReviewWorkUnitError::DirtySourceDenied)
    );
    service
        .authorize_fix("review.1", &["F-1".to_string()], true, true)
        .unwrap();
    assert_eq!(
        service.record_patch("review.1", patch("P-1", "F-1", "src/http.rs")),
        Err(ReviewWorkUnitError::PatchScopeExpanded)
    );
    service
        .record_patch("review.1", patch("P-1", "F-1", "src/auth.rs"))
        .unwrap();
    let verified = service
        .record_verification(
            "review.1",
            VerificationRecord {
                verification_id: "V-1".to_string(),
                attempt_id: "P-1".to_string(),
                command: "cargo test auth".to_string(),
                passed: true,
                evidence_ref: "evidence://auth-pass".to_string(),
            },
        )
        .unwrap();
    assert_eq!(verified.verifications.len(), 1);
}

#[test]
fn review_stays_read_only_and_delivery_approvals_are_independent() {
    let fixture = TempDir::new().unwrap();
    let store = migrated_store(&fixture.path().join("review.sqlite"));
    let service = ReviewWorkUnitService::new(&store);
    service
        .create("review.2", "feature.ui", "commit:def", false)
        .unwrap();
    service
        .add_finding("review.2", finding("F-1", "src/ui.rs"))
        .unwrap();
    assert_eq!(
        service.record_patch("review.2", patch("P-1", "F-1", "src/ui.rs")),
        Err(ReviewWorkUnitError::ReviewIsReadOnly)
    );
    assert_eq!(
        service.approve_delivery("review.2", DeliveryKind::Commit, None),
        Err(ReviewWorkUnitError::ApprovalRequired)
    );
    let commit = service
        .approve_delivery("review.2", DeliveryKind::Commit, Some("approval.commit"))
        .unwrap();
    assert!(commit.delivery_approvals.contains_key("commit"));
    assert!(!commit.delivery_approvals.contains_key("push"));
    assert!(!commit.delivery_approvals.contains_key("pull_request"));
}

#[test]
fn review_work_unit_source_stays_bounded() {
    xtask::check_logical_line_limit(
        "crates/desktoplab-backend-services/src/review_work_units.rs",
        include_str!("../src/review_work_units.rs"),
        350,
    )
    .unwrap();
}

fn finding(id: &str, path: &str) -> ReviewFinding {
    ReviewFinding {
        finding_id: id.to_string(),
        severity: "high".to_string(),
        title: "Finding".to_string(),
        path: path.to_string(),
        line: Some(10),
        evidence: vec!["evidence://finding".to_string()],
    }
}

fn patch(id: &str, finding_id: &str, path: &str) -> PatchAttempt {
    PatchAttempt {
        attempt_id: id.to_string(),
        selected_finding_ids: vec![finding_id.to_string()],
        changed_paths: vec![path.to_string()],
        evidence_refs: vec!["evidence://patch".to_string()],
    }
}

fn migrated_store(path: &std::path::Path) -> SqliteStore {
    let store = SqliteStore::open(path).unwrap();
    store.apply_migrations().unwrap();
    store
}
