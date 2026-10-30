use desktoplab_storage::{SettingRecord, SettingValue};
use serde_json::{Value, json};

use super::{ApiRouteResponse, LocalApiRouter};

// Increment when the terms presented by the managed LM Studio flow change.
const LM_TERMS_REVISION: &str = "1";

fn key(id: &str) -> Option<String> {
    match id {
        "mlx" => Some(format!(
            "runtime.consent.mlx.apache-2.0.{}",
            crate::runtime_routes::mlx_lm_managed::MODEL_REVISION
        )),
        "lmStudio" => Some(format!(
            "runtime.consent.lm-studio.vendor-terms.{LM_TERMS_REVISION}"
        )),
        _ => None,
    }
}

impl LocalApiRouter {
    pub(crate) fn runtime_consents(&self) -> ApiRouteResponse {
        let Some(storage) = &self.storage else {
            return ApiRouteResponse::state_journal_failed("runtime consent storage unavailable");
        };
        let read = |id| {
            storage
                .get_setting(&key(id).expect("known consent"))
                .map(|record| {
                    record.is_some_and(|record| record.value() == &SettingValue::Boolean(true))
                })
        };
        match (read("mlx"), read("lmStudio")) {
            (Ok(mlx), Ok(lm_studio)) => {
                ApiRouteResponse::ok(json!({"consents":{"mlx":mlx,"lmStudio":lm_studio}}))
            }
            _ => ApiRouteResponse::state_journal_failed("runtime consent storage unavailable"),
        }
    }

    pub(crate) fn update_runtime_consent(&mut self, body: &str) -> ApiRouteResponse {
        let parsed = serde_json::from_str::<Value>(body).ok();
        let id = parsed
            .as_ref()
            .and_then(|value| value.get("id"))
            .and_then(Value::as_str);
        let accepted = parsed
            .as_ref()
            .and_then(|value| value.get("accepted"))
            .and_then(Value::as_bool);
        let (Some(key), Some(accepted)) = (id.and_then(key), accepted) else {
            return ApiRouteResponse::bad_request(json!({"code":"INVALID_RUNTIME_CONSENT"}));
        };
        let Some(storage) = &self.storage else {
            return ApiRouteResponse::state_journal_failed("runtime consent storage unavailable");
        };
        if storage
            .put_setting(SettingRecord::new(key, SettingValue::Boolean(accepted)))
            .is_err()
        {
            return ApiRouteResponse::state_journal_failed("runtime consent storage unavailable");
        }
        self.runtime_consents()
    }
}
