use desktoplab_backend_services::{
    ProviderProbeConfidence, ProviderProbeDefinition, ProviderProbeError, ProviderProbeExecution,
    ProviderProbeExecutor, ProviderProbeInitiation, ProviderProbePermission, ProviderProbeRequest,
    ProviderProbeService, ProviderProbeSource, ProviderProbeState,
};
use desktoplab_vault::{AuthModeMetadata, SecretRef, SecretScope};
use xtask::check_logical_line_limit;

#[test]
fn authenticated_probe_is_fresh_only_until_its_evidence_expires() {
    let definition = api_key_definition();
    let secret_ref = SecretRef::new(SecretScope::Provider, "provider.openai:api-key");
    let mut executor = RecordingExecutor::ready();
    let mut service = ProviderProbeService::default();
    let report = service
        .run(
            &definition,
            "macos-aarch64",
            1_000,
            ProviderProbeInitiation::UserRequested,
            Some(&secret_ref),
            &mut executor,
        )
        .expect("probe should execute");

    assert_eq!(report.state_at(1_059), ProviderProbeState::Ready);
    assert_eq!(report.state_at(1_061), ProviderProbeState::Stale);
    assert_eq!(
        report.confidence(),
        ProviderProbeConfidence::AuthenticatedProviderResponse
    );
    assert_eq!(executor.secret_refs, vec![secret_ref.as_uri()]);
    assert!(!report.diagnostic_summary().contains("secret-value"));
}

#[test]
fn sensitive_probes_require_user_intent_credentials_and_cooldown() {
    let definition = api_key_definition();
    let mut executor = RecordingExecutor::ready();
    let mut service = ProviderProbeService::default();
    let background = service
        .run(
            &definition,
            "macos-aarch64",
            10,
            ProviderProbeInitiation::Background,
            None,
            &mut executor,
        )
        .unwrap();
    assert_eq!(
        background.state_at(10),
        ProviderProbeState::PermissionRequired
    );
    let missing = service
        .run(
            &definition,
            "macos-aarch64",
            11,
            ProviderProbeInitiation::UserRequested,
            None,
            &mut executor,
        )
        .unwrap();
    assert_eq!(missing.state_at(11), ProviderProbeState::MissingCredential);

    let secret_ref = SecretRef::new(SecretScope::Provider, "provider.openai:api-key");
    service
        .run(
            &definition,
            "macos-aarch64",
            20,
            ProviderProbeInitiation::UserRequested,
            Some(&secret_ref),
            &mut executor,
        )
        .unwrap();
    let cooldown = service
        .run(
            &definition,
            "macos-aarch64",
            21,
            ProviderProbeInitiation::UserRequested,
            Some(&secret_ref),
            &mut executor,
        )
        .unwrap();
    assert_eq!(cooldown.state_at(21), ProviderProbeState::Cooldown);
}

#[test]
fn package_target_and_secret_safe_evidence_fail_closed() {
    let definition = api_key_definition();
    let mut executor = RecordingExecutor::ready();
    let mut service = ProviderProbeService::default();
    let unsupported = service
        .run(
            &definition,
            "windows-x64",
            10,
            ProviderProbeInitiation::UserRequested,
            None,
            &mut executor,
        )
        .unwrap();
    assert_eq!(
        unsupported.state_at(10),
        ProviderProbeState::UnsupportedPackage
    );
    assert!(
        ProviderProbeExecution::new(
            ProviderProbeState::Ready,
            ProviderProbeConfidence::AuthenticatedProviderResponse,
            "Bearer sk-secret",
            "evidence:provider",
        )
        .is_err()
    );
    assert!(executor.secret_refs.is_empty());
}

#[test]
fn probe_definitions_declare_every_permission_used_by_their_source() {
    let missing = ProviderProbeDefinition::new(
        "provider.openai",
        AuthModeMetadata::OauthDevice,
        ProviderProbeSource::OauthDeviceFlow,
        60,
        10,
        vec![ProviderProbePermission::ProviderNetwork],
        vec!["macos-aarch64".to_string()],
    );
    assert_eq!(
        missing,
        Err(ProviderProbeError::MissingPermission(
            ProviderProbePermission::BrowserOpen
        ))
    );
}

#[test]
fn provider_probe_source_stays_below_guardrail() {
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/provider_probes.rs",
        include_str!("../src/provider_probes.rs"),
        330,
    )
    .expect("provider probes should remain focused");
    check_logical_line_limit(
        "crates/desktoplab-backend-services/src/provider_probes/types.rs",
        include_str!("../src/provider_probes/types.rs"),
        220,
    )
    .expect("provider probe contracts should remain focused");
}

fn api_key_definition() -> ProviderProbeDefinition {
    ProviderProbeDefinition::new(
        "provider.openai",
        AuthModeMetadata::ApiKeyBilling,
        ProviderProbeSource::ApiKeyRequest,
        60,
        15,
        vec![
            ProviderProbePermission::VaultRead,
            ProviderProbePermission::ProviderNetwork,
        ],
        vec!["macos-aarch64".to_string()],
    )
    .unwrap()
}

struct RecordingExecutor {
    secret_refs: Vec<String>,
}

impl RecordingExecutor {
    fn ready() -> Self {
        Self {
            secret_refs: Vec::new(),
        }
    }
}

impl ProviderProbeExecutor for RecordingExecutor {
    fn execute(
        &mut self,
        request: ProviderProbeRequest<'_>,
    ) -> Result<ProviderProbeExecution, ProviderProbeError> {
        if let Some(reference) = request.credential_ref {
            self.secret_refs.push(reference.as_uri());
        }
        ProviderProbeExecution::new(
            ProviderProbeState::Ready,
            ProviderProbeConfidence::AuthenticatedProviderResponse,
            "authenticated response received",
            "evidence:provider-probe-1",
        )
    }
}
