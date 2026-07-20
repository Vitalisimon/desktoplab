use std::collections::{BTreeMap, BTreeSet};

use desktoplab_storage::{ProductizationRecordKind, ProductizationStateRecord, SqliteStore};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: u32 = 1;
const MAX_FINDINGS: usize = 512;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFinding {
    pub finding_id: String,
    pub severity: String,
    pub title: String,
    pub path: String,
    pub line: Option<u32>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchAttempt {
    pub attempt_id: String,
    pub selected_finding_ids: Vec<String>,
    pub changed_paths: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationRecord {
    pub verification_id: String,
    pub attempt_id: String,
    pub command: String,
    pub passed: bool,
    pub evidence_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryKind {
    Commit,
    Push,
    PullRequest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewWorkUnit {
    schema_version: u32,
    pub work_unit_id: String,
    pub feature_id: String,
    pub source_fingerprint: String,
    pub source_dirty: bool,
    pub fix_authorized: bool,
    pub selected_finding_ids: Vec<String>,
    pub findings: BTreeMap<String, ReviewFinding>,
    pub patch_attempts: Vec<PatchAttempt>,
    pub verifications: Vec<VerificationRecord>,
    pub delivery_approvals: BTreeMap<String, String>,
    pub report: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ReviewWorkUnitError {
    Invalid(&'static str),
    NotFound,
    FindingMissing,
    ReviewIsReadOnly,
    DirtySourceDenied,
    PatchScopeExpanded,
    ApprovalRequired,
    Persistence(String),
}

pub struct ReviewWorkUnitService<'a> {
    storage: &'a SqliteStore,
}

impl<'a> ReviewWorkUnitService<'a> {
    pub fn new(storage: &'a SqliteStore) -> Self {
        Self { storage }
    }

    pub fn create(
        &self,
        work_unit_id: &str,
        feature_id: &str,
        source_fingerprint: &str,
        source_dirty: bool,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        for value in [work_unit_id, feature_id, source_fingerprint] {
            validate(value)?;
        }
        let unit = ReviewWorkUnit {
            schema_version: SCHEMA_VERSION,
            work_unit_id: work_unit_id.to_string(),
            feature_id: feature_id.to_string(),
            source_fingerprint: source_fingerprint.to_string(),
            source_dirty,
            fix_authorized: false,
            selected_finding_ids: Vec::new(),
            findings: BTreeMap::new(),
            patch_attempts: Vec::new(),
            verifications: Vec::new(),
            delivery_approvals: BTreeMap::new(),
            report: None,
        };
        self.persist(&unit)?;
        Ok(unit)
    }

    pub fn add_finding(
        &self,
        work_unit_id: &str,
        finding: ReviewFinding,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        self.update(work_unit_id, |unit| {
            if unit.findings.len() == MAX_FINDINGS {
                return Err(ReviewWorkUnitError::Invalid("finding_capacity_exceeded"));
            }
            validate(&finding.finding_id)?;
            if finding.path.is_empty() || finding.evidence.is_empty() {
                return Err(ReviewWorkUnitError::Invalid("finding_evidence_required"));
            }
            unit.findings.insert(finding.finding_id.clone(), finding);
            Ok(())
        })
    }

    pub fn authorize_fix(
        &self,
        work_unit_id: &str,
        selected_finding_ids: &[String],
        user_intent_allows_dirty: bool,
        policy_allows_dirty: bool,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        self.update(work_unit_id, |unit| {
            if selected_finding_ids.is_empty()
                || selected_finding_ids
                    .iter()
                    .any(|id| !unit.findings.contains_key(id))
            {
                return Err(ReviewWorkUnitError::FindingMissing);
            }
            if unit.source_dirty && !(user_intent_allows_dirty && policy_allows_dirty) {
                return Err(ReviewWorkUnitError::DirtySourceDenied);
            }
            unit.fix_authorized = true;
            unit.selected_finding_ids = selected_finding_ids.to_vec();
            Ok(())
        })
    }

    pub fn record_patch(
        &self,
        work_unit_id: &str,
        attempt: PatchAttempt,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        self.update(work_unit_id, |unit| {
            if !unit.fix_authorized {
                return Err(ReviewWorkUnitError::ReviewIsReadOnly);
            }
            let selected: BTreeSet<_> = unit.selected_finding_ids.iter().collect();
            if attempt
                .selected_finding_ids
                .iter()
                .any(|id| !selected.contains(id))
            {
                return Err(ReviewWorkUnitError::PatchScopeExpanded);
            }
            let allowed_paths: BTreeSet<_> = attempt
                .selected_finding_ids
                .iter()
                .filter_map(|id| unit.findings.get(id).map(|finding| &finding.path))
                .collect();
            if attempt
                .changed_paths
                .iter()
                .any(|path| !allowed_paths.contains(path))
            {
                return Err(ReviewWorkUnitError::PatchScopeExpanded);
            }
            if attempt.evidence_refs.is_empty() {
                return Err(ReviewWorkUnitError::Invalid("patch_evidence_required"));
            }
            unit.patch_attempts.push(attempt);
            Ok(())
        })
    }

    pub fn record_verification(
        &self,
        work_unit_id: &str,
        verification: VerificationRecord,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        self.update(work_unit_id, |unit| {
            if !unit
                .patch_attempts
                .iter()
                .any(|attempt| attempt.attempt_id == verification.attempt_id)
            {
                return Err(ReviewWorkUnitError::Invalid("patch_attempt_missing"));
            }
            if verification.evidence_ref.is_empty() {
                return Err(ReviewWorkUnitError::Invalid(
                    "verification_evidence_required",
                ));
            }
            unit.verifications.push(verification);
            Ok(())
        })
    }

    pub fn approve_delivery(
        &self,
        work_unit_id: &str,
        kind: DeliveryKind,
        approval_record_id: Option<&str>,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        let approval = approval_record_id
            .filter(|value| !value.is_empty())
            .ok_or(ReviewWorkUnitError::ApprovalRequired)?;
        self.update(work_unit_id, |unit| {
            unit.delivery_approvals
                .insert(delivery_key(kind).to_string(), approval.to_string());
            Ok(())
        })
    }

    pub fn finalize_report(
        &self,
        work_unit_id: &str,
        report: &str,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        self.update(work_unit_id, |unit| {
            if report.is_empty() || report.len() > 64 * 1024 {
                return Err(ReviewWorkUnitError::Invalid("invalid_review_report"));
            }
            unit.report = Some(report.to_string());
            Ok(())
        })
    }

    pub fn load(&self, work_unit_id: &str) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        let record = self
            .storage
            .get_productization_state(ProductizationRecordKind::ReviewWorkUnit, work_unit_id)
            .map_err(|error| ReviewWorkUnitError::Persistence(error.to_string()))?
            .ok_or(ReviewWorkUnitError::NotFound)?;
        let unit: ReviewWorkUnit = serde_json::from_str(record.payload())
            .map_err(|error| ReviewWorkUnitError::Persistence(error.to_string()))?;
        if unit.schema_version != SCHEMA_VERSION {
            return Err(ReviewWorkUnitError::Persistence(
                "unsupported_review_work_unit_schema".to_string(),
            ));
        }
        Ok(unit)
    }

    fn update(
        &self,
        work_unit_id: &str,
        mutation: impl FnOnce(&mut ReviewWorkUnit) -> Result<(), ReviewWorkUnitError>,
    ) -> Result<ReviewWorkUnit, ReviewWorkUnitError> {
        let mut unit = self.load(work_unit_id)?;
        mutation(&mut unit)?;
        self.persist(&unit)?;
        Ok(unit)
    }

    fn persist(&self, unit: &ReviewWorkUnit) -> Result<(), ReviewWorkUnitError> {
        let payload = serde_json::to_string(unit)
            .map_err(|error| ReviewWorkUnitError::Persistence(error.to_string()))?;
        self.storage
            .put_productization_state(ProductizationStateRecord::new(
                ProductizationRecordKind::ReviewWorkUnit,
                &unit.work_unit_id,
                payload,
            ))
            .map_err(|error| ReviewWorkUnitError::Persistence(error.to_string()))
    }
}

fn validate(value: &str) -> Result<(), ReviewWorkUnitError> {
    if value.is_empty() || value.len() > 192 {
        Err(ReviewWorkUnitError::Invalid("invalid_work_unit_field"))
    } else {
        Ok(())
    }
}

fn delivery_key(kind: DeliveryKind) -> &'static str {
    match kind {
        DeliveryKind::Commit => "commit",
        DeliveryKind::Push => "push",
        DeliveryKind::PullRequest => "pull_request",
    }
}
