use desktoplab_control_plane::LocalApiRouter;
use desktoplab_storage::{SettingRecord, SettingValue, SqliteStore};
use serde_json::{Value, json};
use tempfile::TempDir;

fn route(router: &mut LocalApiRouter, method: &str, body: Value) -> (String, Value) {
    let response = router
        .route(method, "/v1/runtime-consents", &body.to_string())
        .unwrap();
    (
        response.status().to_string(),
        serde_json::from_str(response.body()).unwrap(),
    )
}

#[test]
fn explicit_consent_is_scoped_persistent_revocable_and_profile_isolated() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("profile.sqlite");
    let other = temp.path().join("other.sqlite");
    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db).unwrap();
    let (_, initial) = route(&mut router, "GET", json!({}));
    assert_eq!(initial["consents"]["mlx"], false);
    assert_eq!(initial["consents"]["lmStudio"], false);
    let (status, _) = route(&mut router, "POST", json!({"id":"mlx", "accepted":true}));
    assert_eq!(status, "200 OK");
    drop(router);
    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db).unwrap();
    let (_, state) = route(&mut router, "GET", json!({}));
    assert_eq!(state["consents"]["mlx"], true);
    assert_eq!(state["consents"]["lmStudio"], false);
    let mut isolated =
        LocalApiRouter::with_storage_path_without_host_recovery_for_test(&other).unwrap();
    assert_eq!(
        route(&mut isolated, "GET", json!({})).1["consents"]["mlx"],
        false
    );
    assert_eq!(
        route(&mut router, "POST", json!({"id":"mlx", "accepted":false})).0,
        "200 OK"
    );
    assert_eq!(
        route(&mut router, "GET", json!({})).1["consents"]["mlx"],
        false
    );
}

#[test]
fn unknown_or_implicit_consents_and_missing_storage_fail_closed() {
    let mut router = LocalApiRouter::default();
    assert_ne!(
        route(&mut router, "POST", json!({"id":"mlx", "accepted":true})).0,
        "200 OK"
    );
    let temp = TempDir::new().unwrap();
    let mut router =
        LocalApiRouter::with_storage_path_without_host_recovery_for_test(&temp.path().join("db"))
            .unwrap();
    for body in [
        json!({"id":"other", "accepted":true}),
        json!({"id":"mlx"}),
        json!({"id":"mlx", "accepted":"true"}),
    ] {
        assert_eq!(route(&mut router, "POST", body).0, "400 Bad Request");
    }
    assert_eq!(
        route(&mut router, "GET", json!({})).1["consents"]["mlx"],
        false
    );
}

#[test]
fn a_different_model_revision_does_not_authorize_the_current_model() {
    let temp = TempDir::new().unwrap();
    let db = temp.path().join("profile.sqlite");
    {
        let storage = SqliteStore::open(&db).unwrap();
        storage.apply_migrations().unwrap();
        storage
            .put_setting(SettingRecord::new(
                "runtime.consent.mlx.apache-2.0.obsolete-revision",
                SettingValue::Boolean(true),
            ))
            .unwrap();
    }
    let mut router = LocalApiRouter::with_storage_path_without_host_recovery_for_test(&db).unwrap();
    assert_eq!(
        route(&mut router, "GET", json!({})).1["consents"]["mlx"],
        false
    );
}
