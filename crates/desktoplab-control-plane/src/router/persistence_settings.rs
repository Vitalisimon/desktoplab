use desktoplab_storage::{SettingRecord, SettingValue};

use super::LocalApiRouter;

impl LocalApiRouter {
    pub(crate) fn persist_default_approval_mode(&mut self) {
        let Some(storage) = &self.storage else {
            return;
        };
        let result = storage.put_setting(SettingRecord::new(
            "approval.default_mode",
            SettingValue::String(self.default_approval_mode.as_str().to_string()),
        ));
        self.record_state_journal_result(result);
    }

    pub(crate) fn persist_high_end_runtime(&mut self) {
        let (Some(storage), Some(runtime)) = (&self.storage, &self.high_end_runtime) else {
            return;
        };
        let payload = serde_json::json!({
            "runtimeId":runtime.contract().runtime_id().as_str(),
            "endpoint":runtime.endpoint().base_url(),
            "modelId":runtime.endpoint().model_id()
        });
        let result = storage.put_setting(SettingRecord::new(
            "runtime.high_end.config",
            SettingValue::String(payload.to_string()),
        ));
        self.record_state_journal_result(result);
    }
}
