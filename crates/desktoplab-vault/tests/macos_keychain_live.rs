#[cfg(target_os = "macos")]
use desktoplab_vault::{NativeKeyringVault, SecretRef, SecretScope, SecretValue, Vault};

#[cfg(target_os = "macos")]
#[test]
#[ignore = "touches the current user's macOS Keychain with an ephemeral value"]
fn macos_keychain_live_round_trip_cleans_up() {
    assert_eq!(
        std::env::var("DESKTOPLAB_LIVE_KEYCHAIN_TEST").as_deref(),
        Ok("1")
    );
    let secret_ref = SecretRef::new(
        SecretScope::Provider,
        format!("signed-shape-smoke-{}", std::process::id()),
    );
    let mut vault = NativeKeyringVault::desktoplab();
    let result = (|| {
        vault.put(
            secret_ref.clone(),
            SecretValue::new("ephemeral-signed-shape-proof"),
        )?;
        let stored = vault.get(&secret_ref)?;
        assert_eq!(stored.expose_for_adapter(), "ephemeral-signed-shape-proof");
        Ok::<_, desktoplab_vault::VaultError>(())
    })();
    let cleanup = vault.delete(&secret_ref);
    result.expect("live Keychain round trip should succeed");
    cleanup.expect("live Keychain smoke credential should be deleted");
    assert!(
        vault.get(&secret_ref).is_err(),
        "deleted smoke credential must stay absent"
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn macos_keychain_live_test_is_macos_only() {
    eprintln!("macOS Keychain live smoke is unavailable on this host");
}
