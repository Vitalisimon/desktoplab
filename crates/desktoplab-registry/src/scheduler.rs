use crate::{
    ManifestFamily, ManifestGroup, RegistryClient, RegistryError, RegistryRecommendation,
    RegistrySource, SignatureVerifier,
};
use std::collections::HashMap;

pub struct RegistryRefreshScheduler<S, V> {
    client: RegistryClient<S, V>,
}

impl<S, V> RegistryRefreshScheduler<S, V>
where
    S: RegistrySource,
    V: SignatureVerifier,
{
    #[must_use]
    pub fn new(client: RegistryClient<S, V>) -> Self {
        Self { client }
    }

    #[must_use]
    pub fn startup_refresh(
        &mut self,
        families: impl IntoIterator<Item = ManifestFamily>,
    ) -> RegistryRefreshReport {
        let mut report = RegistryRefreshReport::new("registry.startup");
        report.push_event(RegistryRefreshEventKind::JobStarted, None);
        self.refresh_families(families, &mut report);
        report.finish();
        report
    }

    #[must_use]
    pub fn manual_refresh(
        &mut self,
        job_id: impl Into<String>,
        families: impl IntoIterator<Item = ManifestFamily>,
    ) -> RegistryRefreshReport {
        let mut report = RegistryRefreshReport::new(job_id);
        report.push_event(RegistryRefreshEventKind::JobQueued, None);
        report.push_event(RegistryRefreshEventKind::JobStarted, None);
        self.refresh_families(families, &mut report);
        report.finish();
        report
    }

    fn refresh_families(
        &mut self,
        families: impl IntoIterator<Item = ManifestFamily>,
        report: &mut RegistryRefreshReport,
    ) {
        for family in families {
            match self.client.refresh_family(family) {
                Ok(group) if group.from_last_known_good() => {
                    report.push_event(RegistryRefreshEventKind::FamilyDegraded, Some(family));
                    report.store_group(group);
                }
                Ok(group) => {
                    report.push_event(RegistryRefreshEventKind::FamilyRefreshed, Some(family));
                    report.store_group(group);
                }
                Err(error) => {
                    report.push_event(RegistryRefreshEventKind::FamilyBlocked, Some(family));
                    report.store_error(family, error);
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryCatalogReadiness {
    Ready,
    Degraded,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryRefreshEventKind {
    JobQueued,
    JobStarted,
    FamilyRefreshed,
    FamilyDegraded,
    FamilyBlocked,
    JobCompleted,
    JobBlocked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryRefreshEvent {
    sequence: u64,
    job_id: String,
    kind: RegistryRefreshEventKind,
    family: Option<ManifestFamily>,
}

impl RegistryRefreshEvent {
    #[must_use]
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    #[must_use]
    pub fn kind(&self) -> RegistryRefreshEventKind {
        self.kind
    }

    #[must_use]
    pub fn family(&self) -> Option<ManifestFamily> {
        self.family
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryRefreshStatus {
    pub readiness: RegistryCatalogReadiness,
    pub last_known_good_available: bool,
    pub degraded_reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryManualRefreshResult {
    pub job_id: Option<String>,
    pub blocked_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub struct RegistryRefreshReport {
    job_id: String,
    next_sequence: u64,
    events: Vec<RegistryRefreshEvent>,
    groups: HashMap<ManifestFamily, ManifestGroup>,
    errors: HashMap<ManifestFamily, RegistryError>,
}

impl RegistryRefreshReport {
    #[must_use]
    pub fn new(job_id: impl Into<String>) -> Self {
        Self {
            job_id: job_id.into(),
            next_sequence: 1,
            events: Vec::new(),
            groups: HashMap::new(),
            errors: HashMap::new(),
        }
    }

    #[must_use]
    pub fn readiness(&self) -> RegistryCatalogReadiness {
        if !self.errors.is_empty() {
            return RegistryCatalogReadiness::Blocked;
        }

        if self
            .groups
            .values()
            .any(ManifestGroup::from_last_known_good)
        {
            return RegistryCatalogReadiness::Degraded;
        }

        RegistryCatalogReadiness::Ready
    }

    #[must_use]
    pub fn status(&self) -> RegistryRefreshStatus {
        RegistryRefreshStatus {
            readiness: self.readiness(),
            last_known_good_available: self
                .groups
                .values()
                .any(ManifestGroup::from_last_known_good),
            degraded_reasons: self.degraded_reasons(),
        }
    }

    #[must_use]
    pub fn manual_refresh_result(&self) -> RegistryManualRefreshResult {
        if self.readiness() == RegistryCatalogReadiness::Blocked && self.groups.is_empty() {
            RegistryManualRefreshResult {
                job_id: None,
                blocked_reason: Some("No safe compatibility catalog is available.".to_string()),
            }
        } else {
            RegistryManualRefreshResult {
                job_id: Some(self.job_id.clone()),
                blocked_reason: None,
            }
        }
    }

    #[must_use]
    pub fn events(&self) -> &[RegistryRefreshEvent] {
        &self.events
    }

    #[must_use]
    pub fn event_kinds(&self) -> Vec<RegistryRefreshEventKind> {
        self.events.iter().map(RegistryRefreshEvent::kind).collect()
    }

    #[must_use]
    pub fn event_sequences(&self) -> Vec<u64> {
        self.events
            .iter()
            .map(RegistryRefreshEvent::sequence)
            .collect()
    }

    #[must_use]
    pub fn group(&self, family: ManifestFamily) -> Option<&ManifestGroup> {
        self.groups.get(&family)
    }

    pub fn recommendations(
        &self,
        family: ManifestFamily,
    ) -> Result<RegistryRecommendation, RegistryError> {
        self.groups
            .get(&family)
            .map(RegistryRecommendation::from_group)
            .ok_or_else(|| {
                RegistryError::NoSafeCatalog(format!("{} catalog is unavailable", family.as_str()))
            })
    }

    fn push_event(&mut self, kind: RegistryRefreshEventKind, family: Option<ManifestFamily>) {
        self.events.push(RegistryRefreshEvent {
            sequence: self.next_sequence,
            job_id: self.job_id.clone(),
            kind,
            family,
        });
        self.next_sequence += 1;
    }

    fn store_group(&mut self, group: ManifestGroup) {
        self.groups.insert(group.family(), group);
    }

    fn store_error(&mut self, family: ManifestFamily, error: RegistryError) {
        self.errors.insert(family, error);
    }

    fn finish(&mut self) {
        let kind = if self.errors.is_empty() {
            RegistryRefreshEventKind::JobCompleted
        } else {
            RegistryRefreshEventKind::JobBlocked
        };
        self.push_event(kind, None);
    }

    fn degraded_reasons(&self) -> Vec<String> {
        let mut reasons = Vec::new();
        for group in self.groups.values() {
            if group.from_last_known_good() {
                reasons.push(format!(
                    "Using last-known-good {} catalog because refresh is unavailable.",
                    group.family().as_str()
                ));
            }
        }
        for family in self.errors.keys() {
            reasons.push(format!(
                "{} catalog refresh is unavailable.",
                family.as_str()
            ));
        }
        reasons.sort();
        reasons
    }
}
