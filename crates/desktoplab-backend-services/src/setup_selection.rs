use crate::{
    SetupPlanPreview, SetupRecommendation, SetupRecommendationRole, SetupWizardApiService,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupAcceptance {
    started_job_ids: Vec<String>,
}

impl SetupAcceptance {
    #[must_use]
    pub fn new(started_job_ids: Vec<String>) -> Self {
        Self { started_job_ids }
    }

    #[must_use]
    pub fn started_job_ids(&self) -> &[String] {
        &self.started_job_ids
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupPlanSelection {
    runtime_id: String,
    model_id: Option<String>,
}

impl SetupPlanSelection {
    #[must_use]
    pub fn new(runtime_id: impl Into<String>, model_id: Option<impl Into<String>>) -> Self {
        Self {
            runtime_id: runtime_id.into(),
            model_id: model_id.map(Into::into),
        }
    }
}

impl SetupPlanPreview {
    #[must_use]
    pub fn recommended_runtime_id(&self) -> Option<&str> {
        recommended_id(&self.runtime_recommendations)
    }

    #[must_use]
    pub fn recommended_model_id(&self) -> Option<&str> {
        recommended_id(&self.model_recommendations)
    }

    #[must_use]
    pub fn alternative_model_ids(&self) -> Vec<&str> {
        self.model_recommendations
            .iter()
            .filter(|recommendation| recommendation.role() == SetupRecommendationRole::Alternative)
            .map(SetupRecommendation::manifest_id)
            .collect()
    }
}

impl SetupWizardApiService {
    #[must_use]
    pub fn accept(&self, preview: SetupPlanPreview) -> SetupAcceptance {
        let selection = SetupPlanSelection {
            runtime_id: preview
                .recommended_runtime_id()
                .unwrap_or_default()
                .to_string(),
            model_id: preview.recommended_model_id().map(ToString::to_string),
        };
        self.accept_selected(&preview, selection)
    }

    #[must_use]
    pub fn accept_selected(
        &self,
        preview: &SetupPlanPreview,
        selection: SetupPlanSelection,
    ) -> SetupAcceptance {
        let mut started_job_ids = Vec::new();
        if contains_id(preview.runtime_recommendations(), &selection.runtime_id) {
            started_job_ids.push(format!("runtime.install:{}", selection.runtime_id));
        }
        if let Some(model_id) = selection.model_id {
            if contains_id(preview.model_recommendations(), &model_id) {
                started_job_ids.push(format!("model.download:{model_id}"));
            }
        }
        SetupAcceptance::new(started_job_ids)
    }
}

fn recommended_id(recommendations: &[SetupRecommendation]) -> Option<&str> {
    recommendations
        .iter()
        .find(|recommendation| recommendation.role() == SetupRecommendationRole::Recommended)
        .map(SetupRecommendation::manifest_id)
}

fn contains_id(recommendations: &[SetupRecommendation], manifest_id: &str) -> bool {
    recommendations
        .iter()
        .any(|recommendation| recommendation.manifest_id() == manifest_id)
}
