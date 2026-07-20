use desktoplab_vault::{
    AuthModeMetadata, CredentialMetadata, CredentialMetadataError, DegradedVaultReason, FakeVault,
    KeychainCommandOutput, KeychainCommandRunner, MacOsKeychainVault, NativeCredentialStore,
    NativeKeyringVault, NativeStoreError, NativeVaultKind, OperatingSystem, SecretRef, SecretScope,
    SecretValue, Vault, VaultAdapterSelection, VaultError,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use xtask::check_logical_line_limit;

#[test]
fn secret_references_use_desktoplab_owned_uri_shape() {
    let secret_ref = SecretRef::new(SecretScope::Provider, "openai-api-key");

    assert_eq!(secret_ref.scope(), SecretScope::Provider);
    assert_eq!(secret_ref.id(), "openai-api-key");
    assert_eq!(
        secret_ref.as_uri(),
        "vault://desktoplab/provider/openai-api-key"
    );
    assert_eq!(
        SecretRef::from_uri(&secret_ref.as_uri()).unwrap(),
        secret_ref
    );
}

#[test]
fn credential_metadata_stores_reference_but_never_secret_value() {
    let secret_ref = SecretRef::new(SecretScope::ExternalBackend, "codex-token");
    let metadata = CredentialMetadata::new(secret_ref.clone(), "Codex token");
    let secret = SecretValue::new("sk-live-do-not-store");

    assert_eq!(metadata.secret_ref(), &secret_ref);
    assert_eq!(metadata.label(), "Codex token");
    assert!(!format!("{metadata:?}").contains("sk-live-do-not-store"));
    assert!(!format!("{secret:?}").contains("sk-live-do-not-store"));
    assert_eq!(secret.redacted(), "[REDACTED]");
}

#[test]
fn credential_metadata_describes_supported_auth_modes_without_secret_values() {
    let secret_ref = SecretRef::new(SecretScope::Provider, "openai-subscription");
    let metadata = CredentialMetadata::with_auth_mode(
        secret_ref.clone(),
        "ChatGPT Team",
        AuthModeMetadata::SubscriptionAccount,
    )
    .with_public_metadata([("provider", "openai"), ("account", "team")])
    .expect("safe public metadata should be accepted");

    assert_eq!(metadata.secret_ref(), &secret_ref);
    assert_eq!(metadata.auth_mode(), AuthModeMetadata::SubscriptionAccount);
    assert_eq!(
        [
            AuthModeMetadata::ApiKeyBilling.as_str(),
            AuthModeMetadata::SubscriptionAccount.as_str(),
            AuthModeMetadata::OauthDevice.as_str(),
            AuthModeMetadata::LocalAppSession.as_str(),
            AuthModeMetadata::CustomEndpoint.as_str(),
        ],
        [
            "api_key_billing",
            "subscription_account",
            "oauth_device",
            "local_app_session",
            "custom_endpoint",
        ]
    );
    assert!(!format!("{metadata:?}").contains("sk-"));
}

#[test]
fn credential_metadata_rejects_raw_tokens_cookies_and_browser_sessions() {
    let secret_ref = SecretRef::new(SecretScope::ExternalBackend, "codex-local-app");

    for (key, value) in [
        ("access_token", "sk-live-do-not-store"),
        ("cookie", "desktoplab_session=abc"),
        ("browser_session", "profile-cookie-material"),
        ("authorization", "Bearer live-token"),
    ] {
        let error = CredentialMetadata::with_auth_mode(
            secret_ref.clone(),
            "Codex local app",
            AuthModeMetadata::LocalAppSession,
        )
        .with_public_metadata([(key, value)])
        .expect_err("secret-like metadata must be rejected");

        assert_eq!(
            error,
            CredentialMetadataError::SecretLikeMetadata(key.to_string())
        );
    }
}

#[test]
fn fake_vault_round_trips_secret_values_for_tests() {
    let mut vault = FakeVault::default();
    let secret_ref = SecretRef::new(SecretScope::Provider, "anthropic-api-key");

    vault
        .put(secret_ref.clone(), SecretValue::new("sk-ant-test"))
        .expect("fake vault should store secret");

    let stored = vault
        .get(&secret_ref)
        .expect("fake vault should retrieve secret");

    assert_eq!(stored.expose_for_adapter(), "sk-ant-test");
}

#[test]
fn platform_selection_maps_supported_operating_systems_to_native_vaults() {
    assert_eq!(
        VaultAdapterSelection::for_os(OperatingSystem::MacOS),
        VaultAdapterSelection::Available(NativeVaultKind::MacOsKeychain)
    );
    assert_eq!(
        VaultAdapterSelection::for_os(OperatingSystem::Windows),
        VaultAdapterSelection::Available(NativeVaultKind::WindowsCredentialManager)
    );
    assert_eq!(
        VaultAdapterSelection::for_os(OperatingSystem::Linux),
        VaultAdapterSelection::Available(NativeVaultKind::LinuxSecretService)
    );
}

#[test]
fn operating_system_labels_parse_to_vault_platforms() {
    assert_eq!(OperatingSystem::from_label("macos"), OperatingSystem::MacOS);
    assert_eq!(
        OperatingSystem::from_label("windows"),
        OperatingSystem::Windows
    );
    assert_eq!(OperatingSystem::from_label("linux"), OperatingSystem::Linux);
    assert_eq!(
        OperatingSystem::from_label("plan9"),
        OperatingSystem::Unsupported("plan9".to_string())
    );
}

#[test]
fn unsupported_vault_environment_enters_explicit_degraded_mode() {
    let selection =
        VaultAdapterSelection::for_os(OperatingSystem::Unsupported("plan9".to_string()));

    assert_eq!(
        selection,
        VaultAdapterSelection::Degraded(DegradedVaultReason::UnsupportedOperatingSystem(
            "plan9".to_string()
        ))
    );
    assert!(!selection.can_save_credentials());
    assert!(!selection.allows_plaintext_fallback());
}

#[test]
fn missing_secret_fails_closed() {
    let vault = FakeVault::default();
    let error = vault
        .get(&SecretRef::new(SecretScope::PrivateRegistry, "missing"))
        .expect_err("missing secret should fail closed");

    assert_eq!(
        error,
        VaultError::SecretNotFound(SecretRef::new(SecretScope::PrivateRegistry, "missing"))
    );
}

#[test]
fn macos_keychain_vault_round_trips_through_security_cli_adapter() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let runner = RecordingKeychainRunner {
        calls: calls.clone(),
    };
    let mut vault = MacOsKeychainVault::with_runner("DesktopLab Test", runner);
    let secret_ref = SecretRef::new(SecretScope::ExternalBackend, "openai-codex/pairing-001");

    vault
        .put(
            secret_ref.clone(),
            SecretValue::new(r#"{"refresh_token":"test"}"#),
        )
        .expect("keychain vault should store secret");
    let stored = vault
        .get(&secret_ref)
        .expect("keychain vault should read secret");
    vault
        .delete(&secret_ref)
        .expect("keychain vault should delete secret");

    assert_eq!(stored.expose_for_adapter(), r#"{"refresh_token":"test"}"#);
    let calls = calls.borrow();
    assert!(
        calls
            .iter()
            .any(|call| call.contains("add-generic-password"))
    );
    assert!(
        calls
            .iter()
            .any(|call| call.contains("find-generic-password"))
    );
    assert!(
        calls
            .iter()
            .any(|call| call.contains("delete-generic-password"))
    );
    assert!(
        calls
            .iter()
            .any(|call| call.contains("external-backend:openai-codex/pairing-001"))
    );
}

#[test]
fn native_keyring_vault_round_trips_without_exposing_secret_in_its_reference() {
    let store = RecordingNativeStore::default();
    let evidence = store.values.clone();
    let mut vault = NativeKeyringVault::with_store("DesktopLab Test", store);
    let secret_ref = SecretRef::new(SecretScope::Provider, "openai:api-key");

    vault
        .put(secret_ref.clone(), SecretValue::new("sk-native-test"))
        .expect("native store should accept the credential");
    assert_eq!(
        vault
            .get(&secret_ref)
            .expect("native store should return the credential")
            .expose_for_adapter(),
        "sk-native-test"
    );
    assert!(!secret_ref.as_uri().contains("sk-native-test"));
    vault
        .delete(&secret_ref)
        .expect("native store should delete the credential");
    assert!(evidence.borrow().is_empty());
    assert_eq!(
        vault.get(&secret_ref),
        Err(VaultError::SecretNotFound(secret_ref))
    );
}

#[test]
fn vault_source_files_stay_below_initial_line_count_guard() {
    check_logical_line_limit(
        "crates/desktoplab-vault/src/lib.rs",
        include_str!("../src/lib.rs"),
        250,
    )
    .expect("vault lib should stay below the initial line-count guard");
    check_logical_line_limit(
        "crates/desktoplab-vault/src/macos_keychain.rs",
        include_str!("../src/macos_keychain.rs"),
        140,
    )
    .expect("macOS keychain adapter should stay focused");
    check_logical_line_limit(
        "crates/desktoplab-vault/src/native_keyring.rs",
        include_str!("../src/native_keyring.rs"),
        160,
    )
    .expect("cross-platform native keyring adapter should stay focused");
}

#[derive(Clone)]
struct RecordingKeychainRunner {
    calls: Rc<RefCell<Vec<String>>>,
}

impl KeychainCommandRunner for RecordingKeychainRunner {
    fn run_security(&self, args: &[String]) -> Result<KeychainCommandOutput, VaultError> {
        self.calls.borrow_mut().push(args.join(" "));
        if args
            .first()
            .is_some_and(|arg| arg == "find-generic-password")
        {
            Ok(KeychainCommandOutput::stdout(
                r#"{"refresh_token":"test"}"#.to_string(),
            ))
        } else {
            Ok(KeychainCommandOutput::stdout(String::new()))
        }
    }
}

#[derive(Clone, Default)]
struct RecordingNativeStore {
    values: Rc<RefCell<HashMap<String, String>>>,
}

impl NativeCredentialStore for RecordingNativeStore {
    fn put(&self, service: &str, account: &str, secret: &str) -> Result<(), NativeStoreError> {
        self.values
            .borrow_mut()
            .insert(format!("{service}:{account}"), secret.to_string());
        Ok(())
    }

    fn get(&self, service: &str, account: &str) -> Result<String, NativeStoreError> {
        self.values
            .borrow()
            .get(&format!("{service}:{account}"))
            .cloned()
            .ok_or(NativeStoreError::Missing)
    }

    fn delete(&self, service: &str, account: &str) -> Result<(), NativeStoreError> {
        self.values
            .borrow_mut()
            .remove(&format!("{service}:{account}"))
            .map(|_| ())
            .ok_or(NativeStoreError::Missing)
    }
}
