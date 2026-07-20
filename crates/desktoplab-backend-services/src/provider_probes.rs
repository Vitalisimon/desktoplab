use std::collections::HashMap;

use desktoplab_vault::{AuthModeMetadata, SecretRef};

mod types;
pub use types::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderProbeReport {
    provider_id: String,
    target: String,
    source: ProviderProbeSource,
    auth_mode: AuthModeMetadata,
    state: ProviderProbeState,
    confidence: ProviderProbeConfidence,
    observed_at: u64,
    fresh_until: u64,
    next_allowed_at: u64,
    summary: String,
    evidence_ref: String,
}

impl ProviderProbeReport {
    #[must_use]
    pub fn state_at(&self, now: u64) -> ProviderProbeState {
        if self.state == ProviderProbeState::Ready && now > self.fresh_until {
            ProviderProbeState::Stale
        } else {
            self.state
        }
    }

    #[must_use]
    pub fn confidence(&self) -> ProviderProbeConfidence {
        self.confidence
    }

    #[must_use]
    pub fn evidence_ref(&self) -> &str {
        &self.evidence_ref
    }

    #[must_use]
    pub fn target(&self) -> &str {
        &self.target
    }

    #[must_use]
    pub fn diagnostic_summary(&self) -> String {
        format!(
            "provider={}; target={}; source={:?}; auth={}; state={:?}; observed_at={}; summary={}; evidence={}",
            self.provider_id,
            self.target,
            self.source,
            self.auth_mode.as_str(),
            self.state,
            self.observed_at,
            self.summary,
            self.evidence_ref
        )
    }
}

#[derive(Debug, Default)]
pub struct ProviderProbeService {
    reports: HashMap<(String, String), ProviderProbeReport>,
}

impl ProviderProbeService {
    pub fn run<E: ProviderProbeExecutor>(
        &mut self,
        definition: &ProviderProbeDefinition,
        target: &str,
        now: u64,
        initiation: ProviderProbeInitiation,
        credential_ref: Option<&SecretRef>,
        executor: &mut E,
    ) -> Result<ProviderProbeReport, ProviderProbeError> {
        let key = (definition.provider_id.clone(), target.to_string());
        if !definition
            .supported_targets
            .iter()
            .any(|item| item == target)
        {
            return Ok(blocked_report(
                definition,
                target,
                now,
                ProviderProbeState::UnsupportedPackage,
            ));
        }
        if initiation == ProviderProbeInitiation::Background
            && definition.permissions.iter().any(|permission| {
                matches!(
                    permission,
                    ProviderProbePermission::BrowserOpen
                        | ProviderProbePermission::VaultRead
                        | ProviderProbePermission::ProcessExecution
                )
            })
        {
            return Ok(blocked_report(
                definition,
                target,
                now,
                ProviderProbeState::PermissionRequired,
            ));
        }
        if requires_credential(definition.source) && credential_ref.is_none() {
            return Ok(blocked_report(
                definition,
                target,
                now,
                ProviderProbeState::MissingCredential,
            ));
        }
        if self
            .reports
            .get(&key)
            .is_some_and(|report| now < report.next_allowed_at)
        {
            return Ok(blocked_report(
                definition,
                target,
                now,
                ProviderProbeState::Cooldown,
            ));
        }
        let execution = executor.execute(ProviderProbeRequest {
            definition,
            credential_ref,
            target,
        })?;
        let report = ProviderProbeReport {
            provider_id: definition.provider_id.clone(),
            target: target.to_string(),
            source: definition.source,
            auth_mode: definition.auth_mode,
            state: execution.state,
            confidence: execution.confidence,
            observed_at: now,
            fresh_until: now.saturating_add(definition.max_age_seconds),
            next_allowed_at: now.saturating_add(definition.cooldown_seconds),
            summary: execution.summary,
            evidence_ref: execution.evidence_ref,
        };
        self.reports.insert(key, report.clone());
        Ok(report)
    }

    #[must_use]
    pub fn report(&self, provider_id: &str, target: &str) -> Option<&ProviderProbeReport> {
        self.reports
            .get(&(provider_id.to_string(), target.to_string()))
    }
}

fn blocked_report(
    definition: &ProviderProbeDefinition,
    target: &str,
    now: u64,
    state: ProviderProbeState,
) -> ProviderProbeReport {
    ProviderProbeReport {
        provider_id: definition.provider_id.clone(),
        target: target.to_string(),
        source: definition.source,
        auth_mode: definition.auth_mode,
        state,
        confidence: ProviderProbeConfidence::ConfigOnly,
        observed_at: now,
        fresh_until: now,
        next_allowed_at: now,
        summary: format!("probe_{:?}", state).to_ascii_lowercase(),
        evidence_ref: "evidence:none".to_string(),
    }
}

fn requires_credential(source: ProviderProbeSource) -> bool {
    matches!(
        source,
        ProviderProbeSource::OauthDeviceFlow | ProviderProbeSource::ApiKeyRequest
    )
}
